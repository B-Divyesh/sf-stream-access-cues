use axum::{
    body::Body,
    extract::{FromRequestParts, State},
    http::{header, request::Parts, HeaderValue, Request, StatusCode},
    middleware::{from_fn, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use std::{collections::HashSet, env, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tower_http::{
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};
use tracing::{info, warn};

const OPERATOR_HEADER: &str = "x-operator-key";
const BUILD_SHA: &str = env!("BUILD_SHA");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeploymentMode {
    /// The operator runs the service on the same machine (or trusted LAN) as OBS.
    Local,
    /// The factory's public guidance surface. It must never make network requests to OBS.
    Hosted,
}

impl DeploymentMode {
    fn from_env() -> Self {
        match env::var("DEPLOYMENT_MODE").as_deref() {
            Ok("hosted") => Self::Hosted,
            Ok("local") | Err(_) => Self::Local,
            Ok(value) => {
                warn!(
                    deployment_mode = value,
                    "unknown deployment mode; using local mode"
                );
                Self::Local
            }
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Hosted => "hosted",
        }
    }

    fn allows_obs_control(self) -> bool {
        self == Self::Local
    }
}

struct AppState {
    db: SqlitePool,
    initialization_lock: tokio::sync::Mutex<()>,
    deployment_mode: DeploymentMode,
}

type SharedState = Arc<AppState>;

/// A browser-local 256-bit capability. Only its SHA-256 digest is persisted or logged.
#[derive(Clone)]
struct Operator(String);

#[derive(Debug)]
struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({ "error": self.1 }))).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        warn!(%error, "database request failed");
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "The local database could not complete that request.".into(),
        )
    }
}

impl<S> FromRequestParts<S> for Operator
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let key = parts
            .headers
            .get(OPERATOR_HEADER)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                ApiError(
                    StatusCode::UNAUTHORIZED,
                    "This browser does not have a private workspace key. Reload the app to create one.".into(),
                )
            })?;
        if key.len() != 43
            || !key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(ApiError(
                StatusCode::UNAUTHORIZED,
                "The private workspace key was invalid. Reload the app to create a new one.".into(),
            ));
        }
        Ok(Self(format!("{:x}", Sha256::digest(key.as_bytes()))))
    }
}

#[derive(Serialize)]
struct HealthResponse<'a> {
    status: &'a str,
    build_sha: &'a str,
}

#[derive(Serialize)]
struct RuntimeResponse<'a> {
    build_sha: &'a str,
    deployment_mode: &'a str,
    obs_control_available: bool,
}

#[derive(Serialize)]
struct SettingsResponse {
    obs_host: String,
    obs_port: u16,
    configured: bool,
    password_saved: bool,
}

#[derive(Deserialize)]
struct SettingsInput {
    obs_host: String,
    obs_port: u16,
    obs_password: Option<String>,
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq)]
struct ChecklistItem {
    id: String,
    text: String,
    done: bool,
}

#[derive(Clone, Deserialize, Serialize)]
struct Cue {
    id: String,
    label: String,
    scene_name: String,
}

#[derive(Clone, Deserialize, Serialize)]
struct PlatformLink {
    id: String,
    label: String,
    url: String,
}

#[derive(Debug, Serialize)]
struct ObsStatus {
    connected: bool,
    message: String,
    scenes: Vec<String>,
    current_scene: Option<String>,
}

#[derive(Deserialize)]
struct SceneInput {
    scene_name: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("stream_access_cues=info".parse().expect("valid log filter")),
        )
        .init();

    let data_dir = PathBuf::from(env::var("DATA_DIR").unwrap_or_else(|_| "data".into()));
    std::fs::create_dir_all(&data_dir).expect("create data directory");
    let db_path = data_dir.join("stream-access-cues.sqlite");
    let database_url = format!("sqlite://{}?mode=rwc", db_path.display());
    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("connect sqlite");
    migrate(&db).await.expect("run database migrations");
    let deployment_mode = DeploymentMode::from_env();
    if deployment_mode == DeploymentMode::Hosted {
        // A previously public deployment may contain an old private-workspace setting.
        // Do not retain a credential or an enabled connection in a public container.
        disable_hosted_obs_settings(&db)
            .await
            .expect("disable hosted OBS settings");
    }

    let dist_dir = env::var("DIST_DIR").unwrap_or_else(|_| "dist".into());
    let app = app_router(
        Arc::new(AppState {
            db,
            initialization_lock: tokio::sync::Mutex::new(()),
            deployment_mode,
        }),
        dist_dir,
    );
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind server");
    info!(%addr, build_sha = BUILD_SHA, deployment_mode = deployment_mode.name(), "stream access cues listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("serve application");
}

fn app_router(state: SharedState, dist_dir: String) -> Router {
    let index = PathBuf::from(&dist_dir).join("index.html");
    Router::new()
        .route("/health", get(health))
        .nest("/api", api_router())
        .fallback_service(ServeDir::new(&dist_dir).not_found_service(ServeFile::new(index)))
        .with_state(state)
        .layer(from_fn(response_policy))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("default-src 'self'; connect-src 'self'; img-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'"),
        ))
        .layer(TraceLayer::new_for_http())
}

fn api_router() -> Router<SharedState> {
    Router::new()
        .route("/runtime", get(runtime))
        .route("/settings", get(get_settings).put(put_settings))
        .route("/checklist", get(get_checklist).put(put_checklist))
        .route("/cues", get(get_cues).put(put_cues))
        .route("/links", get(get_links).put(put_links))
        .route("/obs/status", get(get_obs_status))
        .route("/obs/test", post(get_obs_status))
        .route("/obs/scene", post(set_scene))
}

async fn response_policy(request: Request<Body>, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("geolocation=(), microphone=(), camera=(), payment=(), usb=()"),
    );
    headers.insert(
        "cross-origin-opener-policy",
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        "cross-origin-resource-policy",
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        "strict-transport-security",
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    let cache = if path.starts_with("/api/") || path == "/health" {
        "no-store"
    } else if path.starts_with("/assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    response
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl+C handler")
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install signal handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
}

async fn migrate(db: &SqlitePool) -> Result<(), sqlx::Error> {
    // The previous release stored one globally readable namespace. It has no safe owner to
    // migrate, so remove it rather than accidentally assigning it to the next visitor.
    for table in ["settings", "checklist", "cues", "platform_links"] {
        sqlx::query(&format!("DROP TABLE IF EXISTS {table}"))
            .execute(db)
            .await?;
    }
    for statement in [
        "CREATE TABLE IF NOT EXISTS operator_settings (operator_id TEXT PRIMARY KEY, obs_host TEXT NOT NULL, obs_port INTEGER NOT NULL, obs_password TEXT NOT NULL, configured INTEGER NOT NULL DEFAULT 0, initialized INTEGER NOT NULL DEFAULT 0)",
        "CREATE TABLE IF NOT EXISTS operator_checklist (operator_id TEXT NOT NULL, id TEXT NOT NULL, position INTEGER NOT NULL, text TEXT NOT NULL, done INTEGER NOT NULL DEFAULT 0, PRIMARY KEY (operator_id, id))",
        "CREATE TABLE IF NOT EXISTS operator_cues (operator_id TEXT NOT NULL, id TEXT NOT NULL, position INTEGER NOT NULL, label TEXT NOT NULL, scene_name TEXT NOT NULL, PRIMARY KEY (operator_id, id))",
        "CREATE TABLE IF NOT EXISTS operator_platform_links (operator_id TEXT NOT NULL, id TEXT NOT NULL, position INTEGER NOT NULL, label TEXT NOT NULL, url TEXT NOT NULL, PRIMARY KEY (operator_id, id))",
    ] {
        sqlx::query(statement).execute(db).await?;
    }
    Ok(())
}

async fn disable_hosted_obs_settings(db: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE operator_settings SET obs_host = '127.0.0.1', obs_port = 4455, obs_password = '', configured = 0")
        .execute(db)
        .await?;
    Ok(())
}

async fn ensure_operator(state: &AppState, operator_id: &str) -> Result<(), ApiError> {
    // The first dashboard load asks four routes in parallel. Serialize only the tiny
    // first-workspace seed transaction so SQLite cannot race its starter rows.
    let _guard = state.initialization_lock.lock().await;
    let db = &state.db;
    sqlx::query("INSERT OR IGNORE INTO operator_settings (operator_id, obs_host, obs_port, obs_password, configured, initialized) VALUES (?, '127.0.0.1', 4455, '', 0, 0)")
        .bind(operator_id)
        .execute(db)
        .await?;
    let initialized: i64 =
        sqlx::query_scalar("SELECT initialized FROM operator_settings WHERE operator_id = ?")
            .bind(operator_id)
            .fetch_one(db)
            .await?;
    if initialized == 0 {
        let mut tx = db.begin().await?;
        for (position, text) in [
            "Set stream title and category",
            "Check microphone level",
            "Confirm recording path",
            "Test scene cues",
            "Start broadcast",
        ]
        .iter()
        .enumerate()
        {
            sqlx::query("INSERT OR IGNORE INTO operator_checklist (operator_id, id, position, text, done) VALUES (?, ?, ?, ?, 0)")
                .bind(operator_id)
                .bind(format!("starter-{}", position + 1))
                .bind(position as i64)
                .bind(text)
                .execute(&mut *tx)
                .await?;
        }
        for (position, (id, label, url)) in [
            (
                "twitch",
                "Open Twitch dashboard",
                "https://dashboard.twitch.tv/",
            ),
            (
                "youtube",
                "Open YouTube Studio",
                "https://studio.youtube.com/",
            ),
        ]
        .iter()
        .enumerate()
        {
            sqlx::query("INSERT OR IGNORE INTO operator_platform_links (operator_id, id, position, label, url) VALUES (?, ?, ?, ?, ?)")
                .bind(operator_id)
                .bind(id)
                .bind(position as i64)
                .bind(label)
                .bind(url)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("UPDATE operator_settings SET initialized = 1 WHERE operator_id = ?")
            .bind(operator_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
    }
    Ok(())
}

async fn health() -> Json<HealthResponse<'static>> {
    Json(HealthResponse {
        status: "ok",
        build_sha: BUILD_SHA,
    })
}

async fn runtime(State(state): State<SharedState>) -> Json<RuntimeResponse<'static>> {
    Json(RuntimeResponse {
        build_sha: BUILD_SHA,
        deployment_mode: state.deployment_mode.name(),
        obs_control_available: state.deployment_mode.allows_obs_control(),
    })
}

async fn get_settings(
    State(state): State<SharedState>,
    operator: Operator,
) -> Result<Json<SettingsResponse>, ApiError> {
    if !state.deployment_mode.allows_obs_control() {
        return Ok(Json(SettingsResponse {
            obs_host: "127.0.0.1".into(),
            obs_port: 4455,
            configured: false,
            password_saved: false,
        }));
    }
    ensure_operator(&state, &operator.0).await?;
    settings_response(&state.db, &operator.0).await
}

async fn settings_response(
    db: &SqlitePool,
    operator_id: &str,
) -> Result<Json<SettingsResponse>, ApiError> {
    let row = sqlx::query("SELECT obs_host, obs_port, obs_password, configured FROM operator_settings WHERE operator_id = ?")
        .bind(operator_id)
        .fetch_one(db)
        .await?;
    Ok(Json(SettingsResponse {
        obs_host: row.get("obs_host"),
        obs_port: row.get::<i64, _>("obs_port") as u16,
        configured: row.get::<i64, _>("configured") != 0,
        password_saved: !row.get::<String, _>("obs_password").is_empty(),
    }))
}

async fn put_settings(
    State(state): State<SharedState>,
    operator: Operator,
    Json(input): Json<SettingsInput>,
) -> Result<Json<SettingsResponse>, ApiError> {
    require_local_obs(&state)?;
    validate_host(&input.obs_host)?;
    if input.obs_port == 0 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "OBS port must be between 1 and 65535.".into(),
        ));
    }
    if input
        .obs_password
        .as_deref()
        .is_some_and(|value| value.len() > 256)
    {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "OBS password is too long.".into(),
        ));
    }
    ensure_operator(&state, &operator.0).await?;
    if let Some(password) = input.obs_password {
        sqlx::query("UPDATE operator_settings SET obs_host = ?, obs_port = ?, obs_password = ?, configured = 1 WHERE operator_id = ?")
            .bind(input.obs_host.trim()).bind(input.obs_port as i64).bind(password).bind(&operator.0).execute(&state.db).await?;
    } else {
        sqlx::query("UPDATE operator_settings SET obs_host = ?, obs_port = ?, configured = 1 WHERE operator_id = ?")
            .bind(input.obs_host.trim()).bind(input.obs_port as i64).bind(&operator.0).execute(&state.db).await?;
    }
    settings_response(&state.db, &operator.0).await
}

fn validate_host(host: &str) -> Result<(), ApiError> {
    let trimmed = host.trim();
    if trimmed.is_empty()
        || trimmed.len() > 253
        || trimmed.contains('/')
        || trimmed.contains(':')
        || trimmed.chars().any(char::is_whitespace)
    {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "Enter a host name only, such as 127.0.0.1 or host.docker.internal.".into(),
        ));
    }
    Ok(())
}

fn require_local_obs(state: &AppState) -> Result<(), ApiError> {
    if !state.deployment_mode.allows_obs_control() {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "OBS control is available only from the local Stream Access Cues service. Run the local container on the computer where OBS is running; this hosted site never connects to your OBS WebSocket.".into(),
        ));
    }
    Ok(())
}

async fn get_checklist(
    State(state): State<SharedState>,
    operator: Operator,
) -> Result<Json<Vec<ChecklistItem>>, ApiError> {
    ensure_operator(&state, &operator.0).await?;
    checklist_response(&state.db, &operator.0).await
}

async fn checklist_response(
    db: &SqlitePool,
    operator_id: &str,
) -> Result<Json<Vec<ChecklistItem>>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, text, done FROM operator_checklist WHERE operator_id = ? ORDER BY position",
    )
    .bind(operator_id)
    .fetch_all(db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| ChecklistItem {
                id: row.get("id"),
                text: row.get("text"),
                done: row.get::<i64, _>("done") != 0,
            })
            .collect(),
    ))
}

async fn put_checklist(
    State(state): State<SharedState>,
    operator: Operator,
    Json(items): Json<Vec<ChecklistItem>>,
) -> Result<Json<Vec<ChecklistItem>>, ApiError> {
    if items.len() > 50 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "A checklist can contain at most 50 items.".into(),
        ));
    }
    validate_unique_ids(
        items.iter().map(|item| item.id.as_str()),
        "Each checklist item needs a unique identifier.",
    )?;
    for item in &items {
        validate_text("Checklist item", &item.text, 200)?;
        validate_id(&item.id)?;
    }
    ensure_operator(&state, &operator.0).await?;
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM operator_checklist WHERE operator_id = ?")
        .bind(&operator.0)
        .execute(&mut *tx)
        .await?;
    for (position, item) in items.iter().enumerate() {
        sqlx::query("INSERT INTO operator_checklist (operator_id, id, position, text, done) VALUES (?, ?, ?, ?, ?)")
            .bind(&operator.0).bind(&item.id).bind(position as i64).bind(item.text.trim()).bind(item.done).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    checklist_response(&state.db, &operator.0).await
}

async fn get_cues(
    State(state): State<SharedState>,
    operator: Operator,
) -> Result<Json<Vec<Cue>>, ApiError> {
    ensure_operator(&state, &operator.0).await?;
    cues_response(&state.db, &operator.0).await
}

async fn cues_response(db: &SqlitePool, operator_id: &str) -> Result<Json<Vec<Cue>>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, label, scene_name FROM operator_cues WHERE operator_id = ? ORDER BY position",
    )
    .bind(operator_id)
    .fetch_all(db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| Cue {
                id: row.get("id"),
                label: row.get("label"),
                scene_name: row.get("scene_name"),
            })
            .collect(),
    ))
}

async fn put_cues(
    State(state): State<SharedState>,
    operator: Operator,
    Json(cues): Json<Vec<Cue>>,
) -> Result<Json<Vec<Cue>>, ApiError> {
    if cues.len() > 9 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "You can assign at most nine keyboard cues.".into(),
        ));
    }
    validate_unique_ids(
        cues.iter().map(|cue| cue.id.as_str()),
        "Each cue needs a unique identifier.",
    )?;
    for cue in &cues {
        validate_id(&cue.id)?;
        validate_text("Cue label", &cue.label, 60)?;
        validate_text("Scene name", &cue.scene_name, 128)?;
    }
    ensure_operator(&state, &operator.0).await?;
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM operator_cues WHERE operator_id = ?")
        .bind(&operator.0)
        .execute(&mut *tx)
        .await?;
    for (position, cue) in cues.iter().enumerate() {
        sqlx::query("INSERT INTO operator_cues (operator_id, id, position, label, scene_name) VALUES (?, ?, ?, ?, ?)")
            .bind(&operator.0).bind(&cue.id).bind(position as i64).bind(cue.label.trim()).bind(cue.scene_name.trim()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    cues_response(&state.db, &operator.0).await
}

async fn get_links(
    State(state): State<SharedState>,
    operator: Operator,
) -> Result<Json<Vec<PlatformLink>>, ApiError> {
    ensure_operator(&state, &operator.0).await?;
    links_response(&state.db, &operator.0).await
}

async fn links_response(
    db: &SqlitePool,
    operator_id: &str,
) -> Result<Json<Vec<PlatformLink>>, ApiError> {
    let rows = sqlx::query("SELECT id, label, url FROM operator_platform_links WHERE operator_id = ? ORDER BY position")
        .bind(operator_id).fetch_all(db).await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| PlatformLink {
                id: row.get("id"),
                label: row.get("label"),
                url: row.get("url"),
            })
            .collect(),
    ))
}

async fn put_links(
    State(state): State<SharedState>,
    operator: Operator,
    Json(links): Json<Vec<PlatformLink>>,
) -> Result<Json<Vec<PlatformLink>>, ApiError> {
    if links.len() > 8 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "You can save at most eight metadata links.".into(),
        ));
    }
    validate_unique_ids(
        links.iter().map(|link| link.id.as_str()),
        "Each metadata link needs a unique identifier.",
    )?;
    for link in &links {
        validate_id(&link.id)?;
        validate_text("Link label", &link.label, 80)?;
        let url = url::Url::parse(&link.url).map_err(|_| {
            ApiError(
                StatusCode::BAD_REQUEST,
                "Each metadata link needs a complete web address.".into(),
            )
        })?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(ApiError(
                StatusCode::BAD_REQUEST,
                "Metadata links must use http or https.".into(),
            ));
        }
    }
    ensure_operator(&state, &operator.0).await?;
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM operator_platform_links WHERE operator_id = ?")
        .bind(&operator.0)
        .execute(&mut *tx)
        .await?;
    for (position, link) in links.iter().enumerate() {
        sqlx::query("INSERT INTO operator_platform_links (operator_id, id, position, label, url) VALUES (?, ?, ?, ?, ?)")
            .bind(&operator.0).bind(&link.id).bind(position as i64).bind(link.label.trim()).bind(link.url.trim()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    links_response(&state.db, &operator.0).await
}

fn validate_text(name: &str, value: &str, max: usize) -> Result<(), ApiError> {
    let length = value.trim().chars().count();
    if length == 0 || length > max {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            format!("{name} must be between 1 and {max} characters."),
        ));
    }
    Ok(())
}

fn validate_id(id: &str) -> Result<(), ApiError> {
    if id.is_empty()
        || id.len() > 80
        || !id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "An item identifier was invalid.".into(),
        ));
    }
    Ok(())
}

fn validate_unique_ids<'a>(
    mut ids: impl Iterator<Item = &'a str>,
    message: &str,
) -> Result<(), ApiError> {
    let mut seen = HashSet::new();
    if ids.any(|id| !seen.insert(id)) {
        return Err(ApiError(StatusCode::BAD_REQUEST, message.into()));
    }
    Ok(())
}

async fn obs_client(state: &AppState, operator_id: &str) -> Result<obws::Client, ApiError> {
    require_local_obs(state)?;
    ensure_operator(state, operator_id).await?;
    let db = &state.db;
    let row = sqlx::query("SELECT obs_host, obs_port, obs_password, configured FROM operator_settings WHERE operator_id = ?")
        .bind(operator_id).fetch_one(db).await?;
    if row.get::<i64, _>("configured") == 0 {
        return Err(ApiError(
            StatusCode::PRECONDITION_REQUIRED,
            "Configure the OBS WebSocket connection first.".into(),
        ));
    }
    let host: String = row.get("obs_host");
    let port = row.get::<i64, _>("obs_port") as u16;
    let password: String = row.get("obs_password");
    tokio::time::timeout(Duration::from_secs(4), obws::Client::connect(host, port, if password.is_empty() { None } else { Some(password) }))
        .await.map_err(|_| ApiError(StatusCode::GATEWAY_TIMEOUT, "OBS did not answer within four seconds. Check that WebSocket server is enabled.".into()))?
        .map_err(|_| ApiError(StatusCode::BAD_GATEWAY, "Could not connect to OBS. Check the host, port, password, and WebSocket server setting.".into()))
}

async fn get_obs_status(
    State(state): State<SharedState>,
    operator: Operator,
) -> Result<Json<ObsStatus>, ApiError> {
    let client = obs_client(&state, &operator.0).await?;
    let response = client.scenes().list().await.map_err(|_| {
        ApiError(
            StatusCode::BAD_GATEWAY,
            "Connected to OBS, but could not read its scenes.".into(),
        )
    })?;
    let scenes = response
        .scenes
        .into_iter()
        .map(|scene| scene.id.name)
        .collect();
    let current_scene = response.current_program_scene.map(|scene| scene.name);
    Ok(Json(ObsStatus {
        connected: true,
        message: "OBS is connected and ready.".into(),
        scenes,
        current_scene,
    }))
}

async fn set_scene(
    State(state): State<SharedState>,
    operator: Operator,
    Json(input): Json<SceneInput>,
) -> Result<Json<ObsStatus>, ApiError> {
    validate_text("Scene name", &input.scene_name, 128)?;
    let client = obs_client(&state, &operator.0).await?;
    client
        .scenes()
        .set_current_program_scene(input.scene_name.as_str())
        .await
        .map_err(|_| {
            ApiError(
                StatusCode::BAD_GATEWAY,
                format!(
                    "OBS could not switch to scene ‘{}’. Refresh scenes and check the cue name.",
                    input.scene_name
                ),
            )
        })?;
    let mut status = get_obs_status(State(state), operator).await?.0;
    status.message = format!("Scene changed to {}.", input.scene_name);
    Ok(Json(status))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use tower::ServiceExt;

    async fn test_state() -> SharedState {
        test_state_for(DeploymentMode::Local).await
    }

    async fn test_state_for(deployment_mode: DeploymentMode) -> SharedState {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("memory database");
        migrate(&db).await.expect("migrate test database");
        Arc::new(AppState {
            db,
            initialization_lock: tokio::sync::Mutex::new(()),
            deployment_mode,
        })
    }

    fn request(method: &str, path: &str, key: Option<&str>, body: &str) -> Request<Body> {
        let mut builder = Request::builder()
            .method(method)
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(key) = key {
            builder = builder.header(OPERATOR_HEADER, key);
        }
        builder.body(Body::from(body.to_owned())).expect("request")
    }

    #[test]
    fn host_validation_rejects_urls_and_accepts_local_hosts() {
        assert!(validate_host("127.0.0.1").is_ok());
        assert!(validate_host("host.docker.internal").is_ok());
        assert!(validate_host("ws://localhost").is_err());
        assert!(validate_host("").is_err());
    }

    #[test]
    fn identifiers_are_constrained() {
        assert!(validate_id("cue-123_abc").is_ok());
        assert!(validate_id("bad id").is_err());
        assert!(validate_id("<script>").is_err());
    }

    #[tokio::test]
    async fn anonymous_requests_are_rejected_and_operator_data_isolated() {
        let state = test_state().await;
        let app = api_router().with_state(state);
        let first = "a".repeat(43);
        let second = "b".repeat(43);
        let response = app
            .clone()
            .oneshot(request("GET", "/checklist", None, ""))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let response = app
            .clone()
            .oneshot(request(
                "PUT",
                "/checklist",
                Some(&first),
                r#"[{"id":"private","text":"Only first operator","done":true}]"#,
            ))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        for (path, body) in [
            (
                "/settings",
                r#"{"obs_host":"first-workspace.invalid","obs_port":4455,"obs_password":"secret"}"#,
            ),
            (
                "/cues",
                r#"[{"id":"first-cue","label":"First only cue","scene_name":"First only scene"}]"#,
            ),
            (
                "/links",
                r#"[{"id":"first-link","label":"First only link","url":"https://example.com/first"}]"#,
            ),
        ] {
            let response = app
                .clone()
                .oneshot(request("PUT", path, Some(&first), body))
                .await
                .expect("response");
            assert_eq!(response.status(), StatusCode::OK, "{path}");
        }
        let response = app
            .clone()
            .oneshot(request("GET", "/checklist", Some(&second), ""))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("body");
        let second_items: Vec<ChecklistItem> = serde_json::from_slice(&body).expect("json");
        assert_eq!(second_items.len(), 5);
        assert!(second_items.iter().all(|item| item.id != "private"));
        for path in ["/settings", "/cues", "/links"] {
            let response = app
                .clone()
                .oneshot(request("GET", path, Some(&second), ""))
                .await
                .expect("response");
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            let body = to_bytes(response.into_body(), 1024 * 1024)
                .await
                .expect("body");
            let text = String::from_utf8(body.to_vec()).expect("utf8");
            assert!(
                !text.contains("first-workspace") && !text.contains("First only"),
                "second operator received first operator's {path}"
            );
        }
        let response = app
            .oneshot(request("GET", "/checklist", Some(&first), ""))
            .await
            .expect("response");
        let body = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("body");
        let first_items: Vec<ChecklistItem> = serde_json::from_slice(&body).expect("json");
        assert_eq!(
            first_items,
            vec![ChecklistItem {
                id: "private".into(),
                text: "Only first operator".into(),
                done: true
            }]
        );
    }

    #[tokio::test]
    async fn duplicate_checklist_ids_return_400_without_replacing_saved_items() {
        let state = test_state().await;
        let app = api_router().with_state(state);
        let key = "c".repeat(43);
        let response = app.clone().oneshot(request("PUT", "/checklist", Some(&key), r#"[{"id":"same","text":"One","done":false},{"id":"same","text":"Two","done":false}]"#)).await.expect("response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let response = app
            .oneshot(request("GET", "/checklist", Some(&key), ""))
            .await
            .expect("response");
        let body = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("body");
        let items: Vec<ChecklistItem> = serde_json::from_slice(&body).expect("json");
        assert_eq!(items.len(), 5);
    }

    #[tokio::test]
    async fn health_has_an_immutable_build_identity() {
        let response = health().await.0;
        assert_eq!(response.status, "ok");
        assert_ne!(response.build_sha, "unversioned-build");
        assert!(!response.build_sha.is_empty());
    }

    #[tokio::test]
    async fn hosted_mode_never_stores_or_connects_obs_credentials() {
        let state = test_state_for(DeploymentMode::Hosted).await;
        let app = api_router().with_state(state);
        let key = "h".repeat(43);

        let runtime_response = app
            .clone()
            .oneshot(request("GET", "/runtime", None, ""))
            .await
            .expect("runtime response");
        assert_eq!(runtime_response.status(), StatusCode::OK);
        let runtime_body = to_bytes(runtime_response.into_body(), 1024 * 1024)
            .await
            .expect("runtime body");
        let runtime: serde_json::Value =
            serde_json::from_slice(&runtime_body).expect("runtime json");
        assert_eq!(runtime["deployment_mode"], "hosted");
        assert_eq!(runtime["obs_control_available"], false);

        for (method, path, body) in [
            (
                "PUT",
                "/settings",
                r#"{"obs_host":"127.0.0.1","obs_port":4455,"obs_password":"never-save-this"}"#,
            ),
            ("GET", "/obs/status", ""),
            ("POST", "/obs/scene", r#"{"scene_name":"Live"}"#),
        ] {
            let response = app
                .clone()
                .oneshot(request(method, path, Some(&key), body))
                .await
                .expect("hosted response");
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{path}");
            let text = String::from_utf8(
                to_bytes(response.into_body(), 1024 * 1024)
                    .await
                    .expect("error body")
                    .to_vec(),
            )
            .expect("utf8");
            assert!(text.contains("local Stream Access Cues service"));
        }

        let response = app
            .oneshot(request("GET", "/settings", Some(&key), ""))
            .await
            .expect("settings response");
        assert_eq!(response.status(), StatusCode::OK);
        let text = String::from_utf8(
            to_bytes(response.into_body(), 1024 * 1024)
                .await
                .expect("settings body")
                .to_vec(),
        )
        .expect("utf8");
        assert!(text.contains("\"configured\":false"));
        assert!(!text.contains("never-save-this"));
    }
}

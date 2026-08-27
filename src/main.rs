use axum::{
    extract::{DefaultBodyLimit, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use std::{env, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tower_http::{
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
}

type SharedState = Arc<AppState>;

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

#[derive(Serialize)]
struct HealthResponse<'a> {
    status: &'a str,
    build_sha: &'a str,
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

#[derive(Clone, Deserialize, Serialize)]
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
    seed(&db).await.expect("seed local defaults");

    let state = Arc::new(AppState { db });
    let dist_dir = env::var("DIST_DIR").unwrap_or_else(|_| "dist".into());
    let index = PathBuf::from(&dist_dir).join("index.html");
    let api = Router::new()
        .route("/settings", get(get_settings).put(put_settings))
        .route("/checklist", get(get_checklist).put(put_checklist))
        .route("/cues", get(get_cues).put(put_cues))
        .route("/links", get(get_links).put(put_links))
        .route("/obs/status", get(get_obs_status))
        .route("/obs/test", post(get_obs_status))
        .route("/obs/scene", post(set_scene));

    let app = Router::new()
        .route("/health", get(health))
        .nest("/api", api)
        .fallback_service(ServeDir::new(&dist_dir).not_found_service(ServeFile::new(index)))
        .with_state(state)
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(SetResponseHeaderLayer::if_not_present(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")))
        .layer(SetResponseHeaderLayer::if_not_present(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY")))
        .layer(SetResponseHeaderLayer::if_not_present(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer")))
        .layer(SetResponseHeaderLayer::if_not_present(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'self'; connect-src 'self'; img-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'")))
        .layer(TraceLayer::new_for_http());

    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind server");
    info!(%addr, "stream access cues listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("serve application");
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
    for statement in [
        "CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK (id = 1), obs_host TEXT NOT NULL, obs_port INTEGER NOT NULL, obs_password TEXT NOT NULL, configured INTEGER NOT NULL DEFAULT 0)",
        "CREATE TABLE IF NOT EXISTS checklist (id TEXT PRIMARY KEY, position INTEGER NOT NULL, text TEXT NOT NULL, done INTEGER NOT NULL DEFAULT 0)",
        "CREATE TABLE IF NOT EXISTS cues (id TEXT PRIMARY KEY, position INTEGER NOT NULL, label TEXT NOT NULL, scene_name TEXT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS platform_links (id TEXT PRIMARY KEY, position INTEGER NOT NULL, label TEXT NOT NULL, url TEXT NOT NULL)",
    ] { sqlx::query(statement).execute(db).await?; }
    sqlx::query("INSERT OR IGNORE INTO settings (id, obs_host, obs_port, obs_password, configured) VALUES (1, '127.0.0.1', 4455, '', 0)").execute(db).await?;
    Ok(())
}

async fn seed(db: &SqlitePool) -> Result<(), sqlx::Error> {
    let checklist_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM checklist")
        .fetch_one(db)
        .await?;
    if checklist_count == 0 {
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
            sqlx::query("INSERT INTO checklist (id, position, text, done) VALUES (?, ?, ?, 0)")
                .bind(format!("starter-{}", position + 1))
                .bind(position as i64)
                .bind(text)
                .execute(db)
                .await?;
        }
    }
    let link_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM platform_links")
        .fetch_one(db)
        .await?;
    if link_count == 0 {
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
            sqlx::query(
                "INSERT INTO platform_links (id, position, label, url) VALUES (?, ?, ?, ?)",
            )
            .bind(id)
            .bind(position as i64)
            .bind(label)
            .bind(url)
            .execute(db)
            .await?;
        }
    }
    Ok(())
}

async fn health() -> Json<HealthResponse<'static>> {
    Json(HealthResponse {
        status: "ok",
        build_sha: option_env!("BUILD_SHA").unwrap_or("development"),
    })
}

async fn get_settings(
    State(state): State<SharedState>,
) -> Result<Json<SettingsResponse>, ApiError> {
    let row = sqlx::query(
        "SELECT obs_host, obs_port, obs_password, configured FROM settings WHERE id = 1",
    )
    .fetch_one(&state.db)
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
    Json(input): Json<SettingsInput>,
) -> Result<Json<SettingsResponse>, ApiError> {
    validate_host(&input.obs_host)?;
    if input.obs_port == 0 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "OBS port must be between 1 and 65535.".into(),
        ));
    }
    if input.obs_password.as_deref().is_some_and(|v| v.len() > 256) {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "OBS password is too long.".into(),
        ));
    }
    if let Some(password) = input.obs_password {
        sqlx::query("UPDATE settings SET obs_host = ?, obs_port = ?, obs_password = ?, configured = 1 WHERE id = 1")
            .bind(input.obs_host.trim()).bind(input.obs_port as i64).bind(password).execute(&state.db).await?;
    } else {
        sqlx::query("UPDATE settings SET obs_host = ?, obs_port = ?, configured = 1 WHERE id = 1")
            .bind(input.obs_host.trim())
            .bind(input.obs_port as i64)
            .execute(&state.db)
            .await?;
    }
    get_settings(State(state)).await
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

async fn get_checklist(
    State(state): State<SharedState>,
) -> Result<Json<Vec<ChecklistItem>>, ApiError> {
    let rows = sqlx::query("SELECT id, text, done FROM checklist ORDER BY position")
        .fetch_all(&state.db)
        .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| ChecklistItem {
                id: r.get("id"),
                text: r.get("text"),
                done: r.get::<i64, _>("done") != 0,
            })
            .collect(),
    ))
}

async fn put_checklist(
    State(state): State<SharedState>,
    Json(items): Json<Vec<ChecklistItem>>,
) -> Result<Json<Vec<ChecklistItem>>, ApiError> {
    if items.len() > 50 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "A checklist can contain at most 50 items.".into(),
        ));
    }
    for item in &items {
        validate_text("Checklist item", &item.text, 200)?;
        validate_id(&item.id)?;
    }
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM checklist")
        .execute(&mut *tx)
        .await?;
    for (position, item) in items.iter().enumerate() {
        sqlx::query("INSERT INTO checklist (id, position, text, done) VALUES (?, ?, ?, ?)")
            .bind(&item.id)
            .bind(position as i64)
            .bind(item.text.trim())
            .bind(item.done)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    get_checklist(State(state)).await
}

async fn get_cues(State(state): State<SharedState>) -> Result<Json<Vec<Cue>>, ApiError> {
    let rows = sqlx::query("SELECT id, label, scene_name FROM cues ORDER BY position")
        .fetch_all(&state.db)
        .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| Cue {
                id: r.get("id"),
                label: r.get("label"),
                scene_name: r.get("scene_name"),
            })
            .collect(),
    ))
}

async fn put_cues(
    State(state): State<SharedState>,
    Json(cues): Json<Vec<Cue>>,
) -> Result<Json<Vec<Cue>>, ApiError> {
    if cues.len() > 9 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "You can assign at most nine keyboard cues.".into(),
        ));
    }
    for cue in &cues {
        validate_id(&cue.id)?;
        validate_text("Cue label", &cue.label, 60)?;
        validate_text("Scene name", &cue.scene_name, 128)?;
    }
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM cues").execute(&mut *tx).await?;
    for (position, cue) in cues.iter().enumerate() {
        sqlx::query("INSERT INTO cues (id, position, label, scene_name) VALUES (?, ?, ?, ?)")
            .bind(&cue.id)
            .bind(position as i64)
            .bind(cue.label.trim())
            .bind(cue.scene_name.trim())
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    get_cues(State(state)).await
}

async fn get_links(State(state): State<SharedState>) -> Result<Json<Vec<PlatformLink>>, ApiError> {
    let rows = sqlx::query("SELECT id, label, url FROM platform_links ORDER BY position")
        .fetch_all(&state.db)
        .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| PlatformLink {
                id: r.get("id"),
                label: r.get("label"),
                url: r.get("url"),
            })
            .collect(),
    ))
}

async fn put_links(
    State(state): State<SharedState>,
    Json(links): Json<Vec<PlatformLink>>,
) -> Result<Json<Vec<PlatformLink>>, ApiError> {
    if links.len() > 8 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "You can save at most eight metadata links.".into(),
        ));
    }
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
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM platform_links")
        .execute(&mut *tx)
        .await?;
    for (position, link) in links.iter().enumerate() {
        sqlx::query("INSERT INTO platform_links (id, position, label, url) VALUES (?, ?, ?, ?)")
            .bind(&link.id)
            .bind(position as i64)
            .bind(link.label.trim())
            .bind(link.url.trim())
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    get_links(State(state)).await
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
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "An item identifier was invalid.".into(),
        ));
    }
    Ok(())
}

async fn obs_client(db: &SqlitePool) -> Result<obws::Client, ApiError> {
    let row = sqlx::query(
        "SELECT obs_host, obs_port, obs_password, configured FROM settings WHERE id = 1",
    )
    .fetch_one(db)
    .await?;
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

async fn get_obs_status(State(state): State<SharedState>) -> Result<Json<ObsStatus>, ApiError> {
    let client = obs_client(&state.db).await?;
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
    Json(input): Json<SceneInput>,
) -> Result<Json<ObsStatus>, ApiError> {
    validate_text("Scene name", &input.scene_name, 128)?;
    let client = obs_client(&state.db).await?;
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
    let mut status = get_obs_status(State(state)).await?.0;
    status.message = format!("Scene changed to {}.", input.scene_name);
    Ok(Json(status))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_state() -> SharedState {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("memory database");
        migrate(&db).await.expect("migrate test database");
        seed(&db).await.expect("seed test database");
        Arc::new(AppState { db })
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
    async fn persistent_routes_round_trip_and_obs_requires_setup() {
        let state = test_state().await;
        let settings = get_settings(State(state.clone()))
            .await
            .expect("get settings")
            .0;
        assert!(!settings.configured);

        let initial = get_checklist(State(state.clone()))
            .await
            .expect("get checklist")
            .0;
        assert_eq!(initial.len(), 5);
        let saved = put_checklist(
            State(state.clone()),
            Json(vec![ChecklistItem {
                id: "test-item".into(),
                text: "Check captions".into(),
                done: true,
            }]),
        )
        .await
        .expect("save checklist")
        .0;
        assert_eq!(saved[0].text, "Check captions");
        assert!(saved[0].done);

        let cues = put_cues(
            State(state.clone()),
            Json(vec![Cue {
                id: "test-cue".into(),
                label: "Starting soon".into(),
                scene_name: "Intro".into(),
            }]),
        )
        .await
        .expect("save cues")
        .0;
        let loaded_cues = get_cues(State(state.clone())).await.expect("get cues").0;
        assert_eq!(loaded_cues.len(), cues.len());
        assert_eq!(loaded_cues[0].scene_name, "Intro");

        let links = put_links(
            State(state.clone()),
            Json(vec![PlatformLink {
                id: "test-link".into(),
                label: "Creator page".into(),
                url: "https://example.com/creator".into(),
            }]),
        )
        .await
        .expect("save links")
        .0;
        let loaded_links = get_links(State(state.clone())).await.expect("get links").0;
        assert_eq!(loaded_links.len(), links.len());
        assert_eq!(loaded_links[0].url, "https://example.com/creator");

        let status_error = get_obs_status(State(state.clone()))
            .await
            .expect_err("unconfigured OBS should fail");
        assert_eq!(status_error.0, StatusCode::PRECONDITION_REQUIRED);
        let scene_error = set_scene(
            State(state),
            Json(SceneInput {
                scene_name: "Intro".into(),
            }),
        )
        .await
        .expect_err("unconfigured scene change should fail");
        assert_eq!(scene_error.0, StatusCode::PRECONDITION_REQUIRED);
    }
}

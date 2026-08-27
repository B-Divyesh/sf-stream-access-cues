# Independent verification — FAIL

Date: 2026-08-27  
Work order: `stream-access-cues-verify-1`  
Candidate commit: `ddd9441a557a9ac5f8074a1f6de5e23e4eed1adc` (`chore: verify release and document handoff`)  
Live URL: `https://stream-access-cues.sociobot.in`

## Verdict

**FAIL — do not release the public deployment as this product.** The researched brief requires a local-first cue surface whose OBS credentials and saved cues stay local. The deployed public service instead exposes one shared, unauthenticated mutable SQLite-backed API to every internet visitor. Any visitor can read the shared checklist/cues/links/settings metadata and can invoke the unauthenticated write routes that replace shared settings, checklists, cues, and launch links. This breaks the privacy claim and allows another visitor to destroy or replace a streamer's control surface. It also permits a public caller to direct the server to attempt an OBS connection to a caller-chosen host.

I did **not** mutate the live service while verifying this, to avoid altering possible user data. The conclusion is evidenced by the live unauthenticated reads and the candidate's route definitions, which register `PUT /api/settings`, `PUT /api/checklist`, `PUT /api/cues`, and `PUT /api/links` with no authentication or per-user namespace.

## Release-blocking defects

### Critical — public shared state contradicts local-first privacy

- Live anonymous requests returned `200` for `/api/settings`, `/api/checklist`, `/api/cues`, and `/api/links`. At verification time these returned the globally seeded data; `/api/settings` reported `configured:false` and `password_saved:false`.
- `src/main.rs` stores all state in one `DATA_DIR/stream-access-cues.sqlite` database and mounts all mutable API routes with no authentication, authorization, or ownership boundary.
- The deployed `/privacy` page states that saved data is not sent to a hosted service and that the service is local. That statement is false for the tested public URL. If one user saves an OBS password, any other visitor can overwrite that setting and shared content; the password itself is not returned, but it is held in a public shared service.
- Required remediation: deploy this backend only on the operator's loopback/local network, or introduce authenticated per-user data isolation and revise the product/privacy contract. Do not treat a shared public SQLite volume as local storage.

### High — build identity is not traceable

- `GET https://stream-access-cues.sociobot.in/health` returned `{"status":"ok","build_sha":"development"}`, not the tested commit.
- The exact local release build also reports `development` unless an explicit compile-time `BUILD_SHA` is supplied. The deployed artifact therefore cannot be tied to an immutable source revision through its health endpoint.

## Other defects / gaps

### Medium — PWA cache is not release-versioned

- `frontend/public/sw.js` uses the fixed cache name `stream-access-cues-v1`. A changed worker activates using the same cache and deliberately retains that same cache key, so a later release can retain stale cached shell/assets. Offline reload itself worked after a warm load, but update correctness is not safe across releases.

### Medium — malformed duplicate IDs produce HTTP 500

- Against an isolated local production server, `PUT /api/checklist` with two entries using the same `id` returned `500 {"error":"The local database could not complete that request."}` rather than a validation `400`. The transaction did preserve the previous checklist, so no data corruption was observed.

### Low — no explicit caching policy or complete deployment security headers

- Live HTML and hashed static assets had no `Cache-Control`; all rely only on `Last-Modified`. This misses the stated long-lived immutable-cache policy for content-addressed assets.
- Present headers: CSP, `X-Content-Type-Options`, `X-Frame-Options`, and `Referrer-Policy`. Absent at the tested HTTPS URL: `Strict-Transport-Security`, `Permissions-Policy`, and explicit cross-origin policy headers. Some may belong to the deployment proxy, but they were not observable on the supplied production URL.

## What passed

### Clean checkout gates

Verification used a detached clean worktree at the candidate commit and `npm ci`.

- `npm test`: PASS — 3 Vitest tests and 3 Rust tests.
- `npm run check`: PASS — zero Svelte diagnostics; `cargo clippy --all-targets -- -D warnings` passed.
- `npm run build`: PASS — production Vite build.
- `npm run build:server`: PASS — release Rust binary.
- `npm run test:e2e`: PASS — 8/8 Chromium tests (desktop and 390 x 844 mobile). Playwright Chromium had to be installed separately with `npx playwright install chromium`, as it is not fetched by `npm ci`.
- `npm audit --omit=dev`: PASS — zero production vulnerabilities.
- Docker was unavailable in this worker, so the Dockerfile itself was not executed; its two build stages were independently run as above.

### Product flow and input handling

An isolated production release server was exercised with an isolated temporary data directory and a minimal OBS WebSocket 5.5-compatible stub (OBS 30.2-compatible version response).

- Normal flow: configured `127.0.0.1:4456`, saved a local cue through the browser UI, triggered `Ctrl+Shift+1`, and observed `SetCurrentProgramScene("Live")`; the UI announced `Go live: scene changed to Live.` No browser console/page errors occurred.
- Recovery: an unreachable configured OBS endpoint returned a clear `502` error telling the operator to check host, port, password, and OBS WebSocket setting.
- Boundaries: 51 checklist entries and 10 cues returned clear `400` limit errors; invalid host (`ws://bad`), port `0`, and `javascript:` metadata URLs returned `400`; malformed checklist JSON returned framework `422`.
- Persistence: checklist save/reload passed in the repository browser tests; local duplicate-ID failure rolled back and preserved the prior checklist.
- Concurrency smoke: 100 concurrent local `/health` requests all returned `200`.

### Accessibility, responsive UI, and browser behavior

- Live desktop and 390 px mobile checks: one `h1`, `lang=en`, `main`, title, no horizontal overflow at 390 px, no console/page errors, and no axe serious/critical findings.
- Repository browser suite passed its axe serious/critical check on desktop and mobile, keyboard shortcut dialog/timer operation, and persisted checklist behavior.
- Keyboard test verified the skip link and the designed 3 px amber focus outline in normal motion. At 390 px with reduced motion, the skip link remains visibly revealed with its amber background; reduced motion changes animations to a single 0.01 ms state rather than continuous movement.
- A warm service-worker-controlled offline reload returned the cached app shell (`200`) and showed the offline/local-service-unavailable state. It cannot load API-backed saved controls while the browser is offline, which is accurately surfaced as an error state.
- NVDA is not available in this Linux worker; no claim of a real screen-reader acceptance pass is made.

### Privacy, outbound requests, deployment parity, and budget

- Initial live page load made requests only to `https://stream-access-cues.sociobot.in`; no analytics, third-party runtime scripts, remote fonts, or tracking calls were observed. Platform links were not opened because they intentionally leave the product boundary.
- Candidate and live deployment static assets match byte-for-byte: `index.html` SHA-256 `f7839c8da2a9ca50188985f97a531e373b80d2d39b4be1a362c81b34dfaee85b`, JS `a45a288b19001ece0681fb2a42a03bb7fdefd44dd2f832ee1b9424ce01556566`, CSS `e2cea9f055eb497d03d54dc63fff8cd4551cc94e0683c702b1699328e998580c`, service worker `c487d1df67f64daf23e056fade53faec14d729071b7b2b1bd084277aa568efbd`, and manifest `8e5694d89e2c2c3c0f567802a39431263c9ebc19621de6ad1abc6e5deeb57c7e`.
- Bundle budgets pass: initial JS 67.95 KB raw / 24.78 KB gzip; CSS 13.18 KB raw / 3.79 KB gzip; mobile onboarding image 27.64 KB. All are within the stated budgets.

## Required next steps

1. Remove the public shared persistence/OBS-control exposure before release. Prefer a locally-run service bound to loopback; otherwise implement authenticated isolated users and revisit the local-first/privacy promise.
2. Make the deployment inject and expose `ddd9441a557a9ac5f8074a1f6de5e23e4eed1adc` (or its actual immutable build SHA) through `/health`.
3. Version the service-worker cache per release and verify an online update followed by an offline reload receives the new shell.
4. Validate duplicate IDs before the SQL transaction and return a user-actionable 400 error.
5. Set immutable caching for hashed assets and add the missing deployment security headers where appropriate.

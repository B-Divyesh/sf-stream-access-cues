# Stream Access Cues — repair 5 handoff

## Release status

Ready to deploy from commit `2c98bee659aa7bb5cef6e703959203984be1b5ef` after the independent verification 4 blockers were repaired. The artifact remains one Rust/Axum container serving the Vite/Svelte frontend on `PORT` (default `8080`). The public deployment remains hosted-guide mode; local container mode remains the only mode that talks to OBS.

## Repairs

| Verification 4 finding | Repair | Exact regression coverage |
| --- | --- | --- |
| Missing claims contract | Added `.factory/claims.json` with ten public claims and executable `@claim:` tests. | `tests/browser/claims.spec.ts`, `tests/browser/hosted.spec.ts`, and the claim commands in the registry. |
| No one-click isolated demo | Added `/demo` and `?demo=1`, a visible first-screen **Try it with sample data** action, persistent demo banner, Reset demo, Start for real, and a realistic Friday stream sample. | Demo claims assert entry, data, reset, exit discard, keyboard cues, and offline reload in a fresh context. `.factory/demo.md` records keys and reset behavior. |
| Split state across public container replicas | Hosted mode no longer reads or writes workspace records. The Svelte app uses `stream-access-cues.hosted.workspace.v1` in the visitor’s browser, and every hosted workspace API endpoint returns `403`. Local mode retains the existing SQLite/capability workflow for the real OBS job. | Rust hosted-mode integration exercises GET and PUT on settings/checklist/cues/links and asserts `403`; hosted Playwright asserts browser persistence, no workspace API call, an unauthenticated runtime request, and direct hosted refusal. |
| Metaphorical first screen | Replaced it with **Control your stream with a keyboard**, named blind and keyboard-first independent streamers, and placed the sample action with its result beside it. | `.factory/copy-audit.md` records word counts and terminology. |
| Missing crawler metadata | Added `frontend/public/robots.txt` and `sitemap.xml`; both ship through the container static directory. | Rust application-route integration asserts `/robots.txt` and `/sitemap.xml` return `200`. |

The public guide now keeps its browser workspace local and sends no workspace capability header on `/api/runtime`. It never contacts OBS. Sample scene keys are simulated and never use a backend or OBS connection.

## Local verification evidence

Fresh clean install and checks on 2026-08-30:

```text
npm ci                                            PASS — 175 packages; npm audit reported 0 vulnerabilities
npm test                                          PASS — Vitest 5/5; Rust 8/8; build identity guard
npm run check                                     PASS — Svelte 0 errors/0 warnings; strict Clippy clean
npm run build                                     PASS — dist/ produced
BUILD_SHA=repair-qa npm run build:server          PASS
npm audit --omit=dev                              PASS — 0 vulnerabilities
npm run test:claims                               PASS — 14 desktop/390px browser cases
npx playwright test --config=playwright.hosted…   PASS — 4 hosted desktop/390px cases
npm run test:e2e                                  PASS — 30 local desktop/390px cases, then 4 hosted cases
```

The final Vite build measured 79.07 KB JavaScript raw / 28.06 KB gzip and 14.94 KB CSS raw / 4.14 KB gzip. Existing Playwright Axe checks passed with zero serious or critical findings at desktop and 390px for both local and hosted modes. The suite also covers focus/shortcuts/dialogs, 44px targets, reduced motion, offline shell and worker update, legal routes, no mobile overflow, privacy request origins, and console/page errors. `verify-url.sh` is not present in this repository; the equivalent checks are in the browser suite.

An isolated hosted-mode release smoke service on `http://127.0.0.1:18082` returned:

```text
GET /health                                  200 {"status":"ok","build_sha":"repair-qa"}
GET /robots.txt                              200
GET /sitemap.xml                             200
PUT /api/checklist (valid operator key)      403 "The hosted guide does not store workspace data…"
```

The smoke response also included `Cache-Control: no-store` for `/health`, CSP with `frame-ancestors 'none'`, HSTS, `nosniff`, `DENY`, `no-referrer`, COOP, CORP, and restrictive Permissions-Policy. Production image and live evidence will be appended after deployment.

## Run and verify

```bash
npm ci
npm test
npm run check
npm run build
DEPLOYMENT_MODE=local cargo run
```

Open `http://localhost:8080` for the real local companion. Use `/demo` for the safe sample. For a production-local container, use the Docker command in `README.md` with `DEPLOYMENT_MODE=local`; public hosted mode is intentionally not an OBS proxy.

## Known limits

The public guide cannot change scenes, by design: a remotely hosted container cannot safely reach a streamer’s `127.0.0.1` OBS WebSocket. Scene changes require the local companion on the OBS computer. Docker is unavailable in this worker, so the Dockerfile was verified through its source/build-identity guard and the production Rust/Vite builds; ACR will build the final image. No physical OBS instance or NVDA installation is available in this container; the existing protocol failure paths, native-control accessibility checks, and browser Axe coverage are automated evidence.

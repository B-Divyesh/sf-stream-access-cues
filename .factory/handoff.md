# Stream Access Cues — repair 4 handoff

## Release status

All four defects in independent verification 3 are repaired. The product remains a Rust/Axum container with the Vite/Svelte frontend served from the same process on port 8080. The public deployment remains hosted-guide mode; self-hosted local mode remains the only mode that can save OBS credentials or contact OBS.

## Repairs and regression coverage

| Verifier finding | Root-cause repair | Regression coverage |
| --- | --- | --- |
| `npm test` exited at the Docker ARG assertion | `tests/build-identity.sh` now checks the actual Docker contract: one global `ARG BUILD_SHA=dev` and three stage-local redeclarations. | The aggregate `npm test` gate now exits 0 and runs the supplied-SHA release binary check. |
| `/privacy` and `/terms` rendered through a 404 SPA fallback | Those URLs are explicit `ServeFile(index.html)` routes, which return 200. The static fallback now preserves true 404s for unknown paths. | Rust integration asserts both legal URLs return the shell with 200 and an unknown path returns 404. Playwright directly navigates both pages and asserts 200, one h1, and no console/page error. |
| Public API had no rate limit | Every `/api/*` route has a shared in-process 20 requests/second, burst-40 limiter keyed to the first `X-Forwarded-For` client hop. Over-limit responses are JSON `429` with `Retry-After`. `/health` is intentionally exempt. | Rust integration consumes the exact burst, asserts `429` and `Retry-After: 1`, then proves a different first forwarded address remains usable. |
| Mobile checklist/footer targets were under 44 px | Checklist checkboxes and labels are 44 px targets; footer legal/source links are 44 px-high controls. Checkbox styling keeps the visual checkmark compact within its full activation area. | Playwright measures every checklist checkbox/label and footer link; each width and height is at least 44 CSS px in desktop and 390 px projects. |

The container contract guard also now rejects a pinned Rust minor tag and requires `FROM rust:1-slim AS server`, compatible with current stable dependency resolution.

Before changing the guard, the failing verifier condition was reproduced directly from the candidate’s original assertion: the Dockerfile had 3 stage-local `ARG BUILD_SHA` declarations, the old `-eq 4` check exited 1.

## Verification evidence

Fresh dependency install and local gates on 2026-08-30:

```text
npm ci                                      PASS — 175 packages, 0 vulnerabilities
npm test                                    PASS — Vitest 3/3; Rust 8/8; build identity runtime guard
npm run check                               PASS — Svelte 0 errors/0 warnings; strict Clippy clean
npm run build                               PASS — dist/ produced
cargo build --release --locked              PASS
npm run test:e2e                            PASS — 18/18 (16 local desktop/390 px + 2 hosted)
npm audit --omit=dev                        PASS — 0 vulnerabilities
```

The factory `verify-url.sh` ran against an isolated local release service at `http://127.0.0.1:18082`:

```json
{"loadMs":635,"errors":[],"a11y":{"title":"Stream Access Cues — local OBS control surface","lang":"en","h1":1,"main":true,"imgsMissingAlt":0,"buttonsUnlabeled":0}}
```

The browser suite includes Axe checks in desktop and 390 px contexts with no serious or critical violations. It also covers keyboard shortcuts, skip link, dialogs, storage isolation, hosted OBS refusal, offline shell reload, and service-worker update cache replacement. The 390 px repair screenshot was visually inspected; no horizontal clipping occurred and the expanded checkbox/footer targets retain the product’s control-panel look.

## Deployment and known limits

Deployed on 2026-08-30 from repair commit `c62a31b64ccc73b92942ce8cebcb3d619ae93369`:

- ACR run `ch1ae` built `sociobotregistry.azurecr.io/sf-stream-access-cues:c62a31b64ccc` (digest `sha256:6eabb96eeca232fd2e6bc929eb2ea591e4327763c8af98bef07ddbd86ec39b5a`) from the source tarball without `.git`.
- Container App `sf-stream-access-cues` in resource group `sociobot` created healthy revision `sf-stream-access-cues--0000006`, with 100% latest-revision traffic.
- Live `https://stream-access-cues.sociobot.in/health` returned `{"status":"ok","build_sha":"c62a31b64ccc73b92942ce8cebcb3d619ae93369"}`.
- A live 220-request anonymous `/api/runtime` burst returned 155 × `200` and 65 × `429`; the immediate follow-up response was `429 Too Many Requests` with `Retry-After: 1` and `Cache-Control: no-store`.
- Live `/privacy` and `/terms` returned `200`, and an uncached Playwright pass found one h1 and zero console/page errors on each. Live desktop and 390 px had zero Axe serious/critical issues, zero undersized repaired targets, and no horizontal overflow.
- Live privacy/offline/update pass: initial browser requests used only `https://stream-access-cues.sociobot.in`; an isolated fresh context reloaded the cached shell offline with one h1, and a new service worker took control with exactly one versioned cache.

The app has only `PORT=8080` at runtime; the Docker image’s hosted default keeps public OBS control disabled.

Docker-compatible tooling is not installed in this worker, so the Dockerfile was checked through its dedicated source/build-identity regression and independent Vite/Rust production builds rather than a local image run. No Windows/NVDA or physical OBS installation is available here; the existing protocol-level OBS and keyboard coverage remains the automated acceptance evidence. No new product behavior is intentionally deferred.

# Stream Access Cues — repair handoff

## Release status

Repaired candidate `877abcd9294622870e413794abc814a6727bc3d6` for the original
container deployment class. The service remains a Rust/Axum backend serving the
Vite/Svelte dashboard from one non-root, multi-stage container on port 8080.

## What changed

- Reproduced the ACR source-tarball failure with `az acr build --registry
  sociobotregistry --image sf-stream-access-cues:build-sha-regression-repro
  --file Dockerfile .` (run `chah`): the old Dockerfile stopped at
  `RUN test -n "$BUILD_SHA"` because no build argument was supplied.
- Declared global `ARG BUILD_SHA=dev`, redeclared it in the web, Rust, and
  runtime stages, and recorded it in the runtime OCI revision label. Empty or
  absent local build arguments now produce `dev`; the factory-supplied full
  SHA is passed unchanged to both Vite and Rust.
- Removed all build-time Git execution from `Dockerfile`, `build.rs`, and
  `vite.config.ts`. This works with the factory ACR tarball, which excludes
  `.git`.
- Added `tests/build-identity.sh`, run by `npm test`. It statically guards the
  Docker argument contract, builds with the complete 40-character supplied
  SHA, starts the compiled binary using an empty environment except `PORT`,
  and asserts that `/health` and the frontend bundle contain that exact SHA.
- Fixed a service-worker update race found during verification. An outgoing
  worker could complete a late cache write after activation; the first request
  controlled by the new worker now sweeps stale release caches. Playwright
  covers offline reload and worker/cache replacement on desktop and mobile.

## How to run

```bash
npm ci
npm test
npm run check
npm run build
cargo build --release --locked
npm run test:e2e
```

For local OBS control:

```bash
docker build --build-arg BUILD_SHA="$(git rev-parse HEAD)" -t stream-access-cues .
docker run --rm -p 8080:8080 -v stream-access-cues-data:/app/data -e DEPLOYMENT_MODE=local stream-access-cues
```

The factory deployment passes only `PORT` at runtime and uses hosted setup-guide
mode baked into the image defaults. It must not be given OBS credentials.

## Verification evidence (2026-08-28)

| Check | Result |
| --- | --- |
| Failed-build reproduction | ACR run `chah` failed as expected at Docker step 7/27: `test -n "$BUILD_SHA"` exited 1; ACR explicitly reported that `.git` was excluded from the uploaded source tarball. |
| Focused build identity | `npm test` passed: 3 Vitest tests, 6 Rust tests, plus `tests/build-identity.sh`. The latter built with `877abcd9294622870e413794abc814a6727bc3d6`, used only `PORT=18080` at process start, and observed `{"status":"ok","build_sha":"877abcd9294622870e413794abc814a6727bc3d6"}`. |
| Types/lint | `npm run check` passed: 0 Svelte errors/warnings; Clippy `-D warnings` passed. |
| Production build | `BUILD_SHA=877abcd9294622870e413794abc814a6727bc3d6 npm run build` and `cargo build --release --locked` passed. Output: 72.55 KB JS raw / 26.15 KB gzip; 13.54 KB CSS raw / 3.89 KB gzip. |
| Browser, mobile, keyboard, accessibility | `npm run test:e2e` passed 14/14 Chromium checks: desktop + 390×844 layout, keyboard shortcuts/dialog Escape, persistence/isolation, hosted OBS refusal, offline/update behavior, and axe serious/critical = 0. |
| Local response/privacy policy | `verify-url.sh` passed with title, `lang=en`, one `h1`, `main`, image alt text, labeled buttons, and no page/console errors. First-load browser requests are same-origin; no analytics or third-party assets are used. `/health` returned `no-store`; CSP, HSTS, COOP, CORP, Permissions-Policy, nosniff, DENY framing, and no-referrer were present. `npm audit --omit=dev`: 0 vulnerabilities. |
| Offline/update | Service-worker-controlled offline reload rendered the h1. A new release worker (`stream-access-cues-update-check`) took control and left only its cache after its first controlled request. |
| Concurrency | 100 parallel local `/health` requests all returned 200. |
| Lighthouse mobile/local | Lighthouse 13.4.1 using the supplied Playwright Chromium: Performance 99, Accessibility 100, Best Practices 100, SEO 100; LCP 1.9 s, CLS 0.023. |

## Deployment and post-deploy check

Deploy with the work-order configuration:

```bash
/opt/fleet/lib/deploy-container.sh stream-access-cues /work/repo Dockerfile 8080
```

The helper passes the checked-out commit to `BUILD_SHA`, `GIT_SHA`, and
`SOURCE_COMMIT`, builds through ACR from a `.git`-free tarball, and supplies
only `PORT=8080` to the Container App. After deployment, verify:

```bash
curl -fsS https://stream-access-cues.sociobot.in/health
curl -fsS https://stream-access-cues.sociobot.in/api/runtime
```

`/health.build_sha` must exactly equal the deployed Git commit; runtime must
report `deployment_mode:"hosted"` and `obs_control_available:false`.

## Known limits

- Docker is not installed in this worker. The exact clean ACR build and live
  deployment use the factory helper instead.
- No physical OBS 28+ server or Windows/NVDA workstation is available here;
  real OBS scene switching and an NVDA acceptance pass remain useful field
  checks. The hosted service intentionally cannot contact OBS.

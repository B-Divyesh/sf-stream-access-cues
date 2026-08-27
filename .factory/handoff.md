# Stream Access Cues — repair handoff

## Release status: ready for container promotion

Repaired the two release blockers recorded in independent verification report `442ad90a5d0911b8bd6d2840be801cffe078ff18`. Product-code repairs are committed as `5ee45e563a1e5eda7831f1a493fbf3133fa2a4ce` and `b19854e3c8d93f42f3dcd2439978256f6e52f273`.

### What changed

1. **Traceable artifact identity (P0).** The Docker build now receives repository Git metadata instead of excluding it. Both Vite and Rust derive `BUILD_SHA` from the checked-out revision when an explicit build argument is absent. `build.rs` watches both `HEAD` and its branch ref, so an ordinary commit rebuilds the binary identity instead of retaining the previous SHA. A local release build made at `b19854e…` returned that exact SHA from `/health`, and its generated JS embedded the same SHA.
2. **Genuinely local OBS control (P1).** The container defaults to `DEPLOYMENT_MODE=hosted`. In that mode the service removes any existing OBS setting at startup, refuses OBS credential writes and all OBS status/scene routes with `403`, and never opens a network connection to OBS. The public UI identifies itself as a setup guide and provides an accessible keyboard-operable local-container walkthrough. The documented `DEPLOYMENT_MODE=local` workflow preserves the original local OBS functionality and keeps the password in the local SQLite volume. It explicitly warns users not to expose OBS WebSocket publicly.
3. **Regression coverage.** Rust tests assert the hosted runtime declaration and all three protected OBS paths. A separate Playwright hosted-mode project tests desktop and 390 px mobile setup flow, dialog keyboard dismissal, the rejected credential write, and axe serious/critical findings. Playwright is pinned to `1.58.2` to match the supplied browser.

## How to run

```bash
npm ci
npm test
npm run check
npm run build
cargo build --release --locked
npm run test:e2e
```

For the real local control surface, use the command in `README.md` (or the public setup dialog):

```bash
docker build --build-arg BUILD_SHA="$(git rev-parse HEAD)" -t stream-access-cues .
docker run --rm -p 8080:8080 -v stream-access-cues-data:/app/data -e DEPLOYMENT_MODE=local stream-access-cues
```

Open `http://localhost:8080`, then configure OBS under Tools → WebSocket Server Settings. The deployed container intentionally remains in hosted setup-guide mode.

## Verification evidence

All runs were completed from this repair checkout on 2026-08-27.

| Check | Result |
| --- | --- |
| Clean install and production audit | `npm ci` passed; `npm audit --omit=dev`: 0 vulnerabilities. |
| Unit/integration | `npm test`: 3 Vitest + 6 Rust tests passed, including hosted OBS refusal and prior isolation/validation coverage. |
| Types/lint | `npm run check`: 0 Svelte diagnostics; Clippy with `-D warnings` passed. |
| Production build | `npm run build` passed: 72.55 KB JS raw / 26.14 KB gzip and 13.54 KB CSS raw / 3.89 KB gzip. `cargo build --release --locked` passed. |
| Browser / keyboard / mobile | `npm run test:e2e`: local dashboard 10/10 plus hosted setup 2/2 passed on desktop and 390×844 Chromium. Includes keyboard timer/shortcuts, dialog Escape, persistence/isolation, no mobile horizontal overflow, and axe serious/critical = 0. |
| Runtime hosted policy | Local hosted-mode smoke: `/api/runtime` declared `obs_control_available:false`; `PUT /api/settings` with an OBS password returned `403` and the recovery message. No route can make the deployment host reach a visitor’s localhost. |
| Headers / response policy / privacy | `/health` was `no-store`; hashed JS was immutable for one year. CSP, HSTS, COOP, CORP, Permissions-Policy, nosniff, frame deny, and no-referrer were observed. `verify-url.sh` passed with title, `lang=en`, one `h1`, `main`, image alt, no unlabeled buttons, and no normal-load console errors. |
| Offline/update | A service-worker-controlled offline reload displayed the application heading. Registering a new release query activated `stream-access-cues-qa-update-check-2` and removed prior release caches. |
| Concurrency | 100 parallel local `/health` requests all returned 200. |
| Lighthouse mobile/local | Lighthouse 12.8.2: Performance 99, Accessibility 100, Best Practices 100, SEO 100; LCP 1.9 s, CLS 0.023. |
| Build identity | A fresh local release server built after `b19854e…` logged and returned `build_sha:"b19854e3c8d93f42f3dcd2439978256f6e52f273"`; its frontend bundle embedded the same value. The container uses the same Git-derived path. |

## Deployment

Deploy with `/opt/fleet/lib/deploy-container.sh stream-access-cues /work/repo Dockerfile 8080`. The Dockerfile is still a non-root multi-stage Rust/Vite container on port 8080. Its default `DEPLOYMENT_MODE=hosted` is required for the public endpoint; do not override it there.

After deployment, verify all of the following against `https://stream-access-cues.sociobot.in`:

```bash
curl -fsS https://stream-access-cues.sociobot.in/health
curl -fsS https://stream-access-cues.sociobot.in/api/runtime
```

The health SHA must be the deployed Git commit, and runtime must say `"deployment_mode":"hosted"` and `"obs_control_available":false`.

## Known limits

- Docker is not installed in this worker, so the image could not be built locally; the locked frontend and Rust release stages passed independently. The factory container deployment is the final image validation.
- No OBS 28+ server or Windows/NVDA environment was available. The existing local API/browser behavior is retained, but a blind acceptance pass with real OBS remains a useful final field check.

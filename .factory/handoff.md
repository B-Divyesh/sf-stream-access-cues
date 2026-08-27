# Stream Access Cues — build handoff

Date: 2026-08-27

Work order: `stream-access-cues-build-1`

Artifact: Rust/Axum + SQLite service serving a Vite/Svelte frontend on `PORT` (default 8080)

## What was built

- A keyboard-first live cue surface with one-tap and shortcut-driven OBS scene changes through OBS WebSocket 5.x.
- Local connection setup for OBS host, port, and optional password. The password is returned to the browser only as a `password_saved` boolean and can be explicitly removed.
- A persistent editable preflight checklist with immediate visible and `aria-live` feedback, per-item speech, progress, and “focus next incomplete” shortcut.
- A persisted, pause/resume-safe session timer with speech output and reset confirmation.
- Up to nine named scene cues, suggested from the live OBS scene list and mapped to Control/Command + Shift + 1–9.
- Editable launch links for platform-owned metadata pages. The product accurately states that it does not write platform metadata or retain platform OAuth tokens.
- First-class loading, unconfigured, empty, disconnected OBS, local-service error, browser-offline, validation, and mobile states.
- Local SQLite persistence with size/count/input validation, 64 KB request limits, parameterized queries, secure response headers, structured JSON logs, a build-SHA health route, and graceful shutdown.
- Installable/offline app shell without analytics, tracking, third-party runtime scripts, or remote fonts.
- `/privacy` and `/terms` routes, an MIT license, full run/deploy documentation, and a non-root multi-stage Dockerfile.

## Product and visual system

`.factory/design.md` records the product-specific “dependable mid-century instrument panel” system: single low-glare palette, Georgia/system-sans pairing, 4 px spacing rhythm, tactile control construction, shortcut grammar, and reduced-motion policy.

The onboarding illustration was generated specifically for this product using the Factory image deployment, manually reviewed for text/brand/anatomy/seam artifacts, and saved with prompt provenance under `assets/src/`. Responsive WebP outputs are 28 KB (768 px) and 60 KB (1280 px), both well below the 300 KB hero budget. The footer discloses generated imagery.

## How to run

```bash
npm install
npm run build
npm run dev
```

Production stages:

```bash
npm run build
cargo build --release --locked
```

The deployment image is defined by the root `Dockerfile`; it serves the app on port 8080 and persists SQLite data at `/app/data`.

## Verification completed

- `npm test`: passed — 3 Vitest cases and 3 Rust tests, including SQLite round trips for settings/checklist/cues/links and OBS precondition responses.
- `npm run check`: passed — zero Svelte/TypeScript diagnostics and Clippy with warnings denied.
- `npm run test:e2e`: passed — 8 Chromium cases across desktop and 390 × 844 mobile. Covers axe, single h1/landmarks, keyboard-only shortcut dialog and timer, checklist persistence, and mobile overflow.
- `npm run build`: passed — `dist/index.html` at the required root; initial JS 67.95 KB raw / 24.78 KB gzip; CSS 13.18 KB raw / 3.79 KB gzip.
- `cargo build --release --locked`: passed.
- Factory `verify-url.sh`: passed — HTTP 200, title, `lang=en`, one h1, main landmark, zero missing image alts, zero unlabeled buttons, and zero console/page errors. Desktop and 390 px screenshots were reviewed.
- Lighthouse mobile: performance 99, accessibility 100, best practices 100, SEO 100; FCP 1.2 s, LCP 2.1 s, CLS 0.023, total blocking time 60 ms.
- Load smoke: 500 `/health` requests at concurrency 100 all returned HTTP 200 in 2.493 seconds (~200 requests/second).
- Manual API smoke: `/health`, settings, seeded checklist, SPA fallback, security headers, and unconfigured OBS 428 response verified.
- `npm audit --omit=dev`: zero production vulnerabilities.

## Known gaps and honest limits

- This worker had no running OBS instance, so the compiled `obws` connection/scene-change path could not be exercised against physical OBS. Error, timeout, authentication, scene-list, and precondition handling are implemented; a release tester should connect OBS 28+ and confirm a real scene change with NVDA.
- NVDA is not available in the Linux worker. Browser name/role/state, axe serious/critical results, live regions, native dialog focus behaviour, focus return, keyboard traversal, and visible focus were tested; a final Windows/NVDA smoke remains the recommended release check.
- Docker is not installed in the worker, so `docker build` could not be executed. Both locked stages used by the Dockerfile (`npm run build` and `cargo build --release --locked`) pass independently, and the server was run from the release binary against the built `dist/`.
- External metadata links remain subject to the accessibility and authentication state of Twitch/YouTube. The product intentionally launches official pages and makes no unsupported metadata-write promise.

## Recommended next steps

1. Run a five-minute Windows/NVDA acceptance pass: configure OBS, add a cue, trigger it with Control + Shift + 1, complete and speak the checklist, and confirm the result without sighted assistance.
2. Build the Dockerfile in CI and run the same `/health` and Playwright smoke against the built image.
3. Test `host.docker.internal` guidance on the factory’s Linux deployment host; document host networking if OBS and the container run on the same Linux machine.

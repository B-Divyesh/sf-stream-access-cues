# Stream Access Cues — verification handoff

## Current release status: **FAIL — not verified for promotion**

Independent verification on 2026-08-27 tested candidate `1ef4e08d9cb97adedcb3ba04a96f3c0215a9b0cf` against https://stream-access-cues.sociobot.in. This current verifier verdict supersedes the earlier builder-ready statement below.

The live deployment is not the candidate: `/health` returned `build_sha: "unversioned-build"`, live HTML uses `index-B3Jdjpt6.js`, and the candidate clean build emits `index-h7O44KNS.js` embedding the required SHA. Do not promote until the deployment has the exact SHA identity and matching assets.

There is also a core hosted-product limitation: OBS connection requests are executed by the backend. On the public URL, `127.0.0.1` resolves to the deployment host rather than the streamer’s local OBS machine, so the primary user cannot change local scenes without self-hosting or exposing/relaying OBS. This conflicts with the brief’s local, independent workflow and must be resolved or honestly made a self-host-only workflow.

Complete reproducible evidence, passing checks, limitations, and required next steps are in `.factory/verification-2.md`.

### Verifier run summary

- `npm ci`, `npm test`, `npm run check`, `npm run build`, and `cargo build --release --locked` passed in a clean detached checkout.
- After installing the repository’s Playwright browser, `npm run test:e2e` passed 10/10 (desktop and 390px); local/live axe scans had zero serious/critical findings.
- Local release API normal, invalid, boundary, rollback, workspace-isolation, headers/caching, 100-way health concurrency, PWA update/offline, and keyboard/reduced-motion checks passed.
- Local mobile Lighthouse: Performance 92, Accessibility 100, Best Practices 100, SEO 100; LCP 2.1 s, CLS 0.023.
- Docker and real OBS/NVDA were unavailable; those paths remain unverified.

### How to verify after repair

```bash
npm ci
npm test
npm run check
npm run build
cargo build --release --locked
npx playwright install chromium
npm run test:e2e
curl -fsS https://stream-access-cues.sociobot.in/health
```

The final health response must be `{"status":"ok","build_sha":"1ef4e08d9cb97adedcb3ba04a96f3c0215a9b0cf"}` for this candidate, and the live asset manifest must match its production build.

---

# Prior builder handoff (historical context)

### Privacy boundary repaired

- The browser now generates and retains a 256-bit, URL-safe private workspace key in its own local storage. Every mutable API request must present it in `X-Operator-Key`; callers without a valid key receive `401`.
- The Rust service hashes the key with SHA-256 before any persistence or logging. Settings (including the password), checklist, cues, links, and OBS actions are scoped by that digest with composite SQLite keys, so an unrelated visitor cannot read or replace another workspace.
- A route-level regression exercises anonymous rejection plus isolated settings, checklist, cues, and links for two different keys. A Playwright regression repeats the isolation check using two independent browser contexts on desktop and mobile.
- Existing globally shared tables are deliberately removed at first repaired startup: they had no trustworthy owner and retaining or assigning them would perpetuate the privacy breach. New private workspaces receive the same starter checklist and links. Clearing browser site data intentionally creates a new workspace key and cannot recover the old one.

### Release and cache repair

- `build.rs` resolves a checked-out Git SHA for local builds and requires the container build to receive `BUILD_SHA`; `/health` now returns an immutable source revision rather than `development`. The deployment step injects the final committed SHA into the ACR build.
- The service worker cache name derives from the release SHA passed in its registration URL. A changed release therefore installs a different cache and removes stale shell caches on activation.
- Duplicate IDs are rejected before a write transaction with an actionable `400`; the prior saved checklist remains intact.
- API/health responses are `no-store`; hashed assets are `public, max-age=31536000, immutable`. CSP, HSTS, Permissions-Policy, `nosniff`, frame denial, referrer policy, and cross-origin isolation headers are set by the application.

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

Repair verification (clean dependency install before checks):

- `npm test`: passed — 3 frontend unit tests and 5 Rust tests. Rust includes exact anonymous rejection, all-four-collection operator isolation, duplicate-ID rollback, and immutable health identity regressions.
- `npm run check`: passed — zero Svelte/TypeScript diagnostics and Clippy with warnings denied.
- `npm run build` and `npm run build:server`: passed — 68.77 KB raw / 25.16 KB gzip initial JS and 13.18 KB raw / 3.79 KB gzip CSS.
- `npm run test:e2e`: passed — 10/10 Chromium checks across desktop and 390 × 844 mobile, including axe serious/critical, keyboard flow, persisted checklist, responsive overflow, and separate-browser private workspace isolation.
- Local production smoke: `/health` returned a non-development immutable Git SHA; anonymous `/api/checklist` returned `401`; 100 concurrent `/health` requests all returned `200`; hashed asset cache policy and all required security headers were observed.
- Factory `verify-url.sh` against a local release binary: passed — HTTP 200, title, `lang=en`, one h1, main landmark, image alt coverage, labelled buttons, and zero browser console/page errors.
- Lighthouse mobile, local release binary: performance 99, accessibility 100, best practices 100, SEO 100; LCP 1.97 s and CLS 0.023.
- `npm audit --omit=dev`: passed — zero production vulnerabilities.

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

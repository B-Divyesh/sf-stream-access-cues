# Independent verification 3 — **FAIL**

Date: 2026-08-28

Work order: `stream-access-cues-verify-3`

Candidate: `0c09b233bd6587a69cab8a8a7102ff159c271e15`

Live URL: https://stream-access-cues.sociobot.in

Starting repository state: clean `main`, with local and `origin/main` both at the candidate

## Release verdict

**FAIL — do not mark this candidate verified.** The live deployment is now the exact candidate and the core local workflow works, but the repository's mandatory `npm test` gate exits 1. Direct loads of both required legal pages also return HTTP 404 and emit a browser console error. The remaining contract findings are listed below.

## Defects

### High — the mandatory `npm test` quality gate fails

`npm test` runs the three Vitest tests and six Rust tests successfully, then invokes `tests/build-identity.sh`. That script exits before its intended build/runtime assertions at:

```text
+ grep -c '^ARG BUILD_SHA$' /work/repo/Dockerfile
+ [[ 3 -eq 4 ]]
EXIT:1
```

The Dockerfile correctly contains one global `ARG BUILD_SHA=dev` plus three stage-local `ARG BUILD_SHA` declarations (web, server, runtime). The test incorrectly expects four stage-local declarations. This is a release blocker under the repository definition of done even though manual build-identity checks pass.

Remediation: make the test assert the actual Docker contract (one global default and three stage-local redeclarations), then rerun the full `npm test` command.

### Medium — direct legal-page requests return 404 and log console errors

- `GET https://stream-access-cues.sociobot.in/privacy` → `404`; the SPA renders the Privacy h1 from the fallback body, but Chromium logs `Failed to load resource: the server responded with a status of 404`.
- `GET https://stream-access-cues.sociobot.in/terms` → `404` with the same console error; the Terms h1 still renders.
- The same behavior comes from the candidate backend's SPA fallback. Root navigation is clean.

This violates the required `/privacy` and `/terms` page contract and the no-console-errors-on-load gate. The fallback should serve `index.html` with status 200 for valid client routes while retaining a real 404 for unknown assets/routes.

### Medium — required public API rate limiting is absent

The Axum router has no rate-limiting layer and the dependency set has no rate limiter. A live burst of 200 anonymous `/api/checklist` requests returned 200 × `401` and no `429`. A caller can generate unlimited syntactically valid workspace keys; first access initializes SQLite rows, so the missing limit creates an avoidable storage/availability abuse path on the hosted service.

Remediation: add an IP-aware limit around workspace initialization and API writes, return `429` with retry guidance, and test it without limiting normal local use.

### Medium — several mobile pointer targets are below the 44 × 44 px contract

At 390 px, five checklist checkboxes measured 26 × 26 CSS px. Their associated one-line labels and the footer Privacy, Terms, and Source code links measured about 24.8 px high. The controls remain keyboard accessible and axe reports no serious/critical issue, but they do not meet the attached accessibility/design target-size requirement.

Remediation: make each checklist label/checkbox pair one ≥44 px activation target and give footer links ≥44 px block height with adequate separation.

## Clean-checkout quality gates

| Check | Result | Fresh evidence |
| --- | --- | --- |
| Install | PASS | `npm ci` installed 175 packages; npm reported 0 vulnerabilities. |
| Required aggregate test | **FAIL** | `npm test`: Vitest 3/3 and Rust 6/6 pass; `tests/build-identity.sh` exits 1 on the incorrect `[[ 3 -eq 4 ]]` assertion. |
| Type/lint | PASS | `npm run check`: Svelte 0 errors/0 warnings; strict `cargo clippy --all-targets -- -D warnings` passed. |
| Exact frontend build | PASS | `BUILD_SHA=0c09b233… npm run build`; Vite 6.4.3 emitted `dist/` successfully. |
| Exact backend build | PASS | `BUILD_SHA=0c09b233… cargo build --release --locked` completed. |
| Browser integration | PASS | `npm run test:e2e`: 14/14 Playwright 1.58.2 tests passed across desktop, 390 × 844, local, and hosted modes. |
| Dependency audit | PASS | `npm audit --omit=dev`: 0 vulnerabilities. |
| Container build | NOT RUN locally | No Docker-compatible CLI is installed in the verifier. The exact live candidate identity and byte-matched artifacts provide deployment evidence, but this run did not execute `docker build`. |

## Core product and backend evidence

An isolated release binary ran from a temporary working directory with **only** `PORT=18081` in its environment. It created its default `data/` directory, served the copied `dist/`, logged structured JSON, and returned:

```json
{"status":"ok","build_sha":"0c09b233bd6587a69cab8a8a7102ff159c271e15"}
```

Normal, boundary, invalid, recovery, and privacy cases:

- Anonymous and malformed workspace keys: `401` with actionable messages.
- Separate 43-character workspace keys received isolated seeded data; key B could not see key A's private checklist.
- Normal checklist save: `200`; restart preserved 50 checklist rows and settings in SQLite.
- Duplicate checklist IDs: `400`; the prior checklist remained intact.
- Blank and 201-character checklist items: `400`; exactly 50 items: `200`; 51 items: `400`.
- Exactly nine cues: `200`; ten cues: `400`.
- HTTPS metadata link: `200`; `javascript:` link: `400`.
- OBS host `ws://bad` and port 0: `400`; unconfigured status: `428`; unreachable configured endpoint: `502` with host/port/password/WebSocket recovery guidance.
- Saved settings expose only `password_saved`; the password is never returned. The raw browser workspace key had zero occurrences in the SQLite file; only its SHA-256 digest is used server-side.
- A 500-request concurrent `/health` smoke completed at 397.8 requests/second with 500 × `200`; 100 concurrent checklist reads also returned 100 × `200`.

For the core job, a temporary OBS WebSocket 5.5-compatible stub advertised OBS 30.2 scenes `Starting soon` and `Live`. The release backend returned connected scene state and accepted `SetCurrentProgramScene("Live")`, responding `200` with `current_scene:"Live"`. A fresh Chromium workspace then:

1. saved the local OBS settings, checklist, and `Go live` cue;
2. triggered `Control+Shift+1`;
3. received the live-region announcement `Go live: scene changed to Live.`;
4. used `Control+Shift+C` to focus `Verify microphone level` with a 3 px amber outline;
5. toggled it with Space and started the timer with `Control+Shift+T`.

That flow produced no console/page errors and made only same-origin requests. An invalid `ws://bad` host stayed in the dialog with the announced validation message, then recovered after correction to `127.0.0.1` and showed `OBS ready`.

No real OBS installation or Windows/NVDA environment was available. The protocol-level scene test and browser semantics are strong automated evidence, but the brief's requested human NVDA acceptance pass remains unperformed.

## Live deployment identity and privacy

Fresh live responses:

- `/health`: `200`, candidate SHA exactly.
- `/api/runtime`: `200`, candidate SHA, `deployment_mode:"hosted"`, `obs_control_available:false`.
- Anonymous checklist: `401`.
- Workspace D saved a private checklist; workspace E received only its own seeded checklist.
- Attempting to save an OBS password in hosted mode: `403`; a subsequent settings read remained `configured:false,password_saved:false`.
- Hosted OBS status/network attempt: `403`. The public deployment does not contact user-supplied OBS endpoints.
- First page load contacted only `https://stream-access-cues.sociobot.in`; no analytics, CDN scripts, remote fonts, pixels, or other third-party runtime requests appeared.
- Browser local storage contained only a 43-character random workspace key on first load. Timer state is stored there only after timer use.
- Cross-origin preflight from `https://evil.example` received `405` and no CORS grant.

The live frontend matches the exact clean candidate build byte-for-byte:

| File | SHA-256 |
| --- | --- |
| `index.html` | `90d5438db2238f9b2311167f9605ec5bd486ed622c2cf962b97c4b519ef8342d` |
| `assets/index-CPCYgNJz.js` | `337774a1f70d04ed59d85f7862cf516216a412ba7376dcf2aad7db3479e19705` |
| `assets/index-CtzPbFHH.css` | `9eae123417207485b5065dd19ad8dcfdff415fdff5485fdfc0d99771a7d11c86` |
| `sw.js` | `7048df48005cbdfcd42426fe2f13e6e255d39516b11019f7e3a2081d531d8db0` |
| `manifest.webmanifest` | `8e5694d89e2c2c3c0f567802a39431263c9ebc19621de6ad1abc6e5deeb57c7e` |
| `icon.svg` | `159bf53573970059416f67d9c7c48d86011ab7dd9d8f0b6d8a0b05c24b95c459` |
| 1280 px hero WebP | `8738de51cc6c023d7c2585ab6c78efd66df36eed4197f01fa57f62606fe7647e` |
| 768 px hero WebP | `7b5e69fd2190604b592454a6f427bd24186917911b9792bdff3e36101ebc258e` |

## Browser, accessibility, response policy, and PWA

- Factory `verify-url.sh`: PASS — live 200 root, title, `lang=en`, one h1, main, image alt text, labeled buttons, and no root-load console/page errors.
- Desktop 1440 × 900 and mobile 390 × 844: no horizontal overflow, coherent responsive stacking, and no visual clipping in captured full-page screenshots.
- Axe: 0 serious/critical findings on both viewports.
- Keyboard: skip link becomes a visible 48.8 px-high amber target with a 3 px outline; Enter bypasses header controls and the next Tab lands in main. Shortcut/local-setup dialogs focus their named close buttons, Escape closes, and focus returns to the launcher. `?` opens the documented shortcut guide.
- Reduced motion: active animations report 0.01 ms duration; no looping/flashing motion was observed.
- Root response: CSP, HSTS, Permissions-Policy, COOP, CORP, `nosniff`, `DENY` framing, `no-referrer`, and `Cache-Control: no-cache`.
- APIs and `/health`: `Cache-Control: no-store`.
- Hashed JS/CSS/assets: `Cache-Control: public, max-age=31536000, immutable`; `sw.js`: `no-cache`.
- PWA: candidate-versioned cache controlled the page; offline reload returned 200 and rendered the h1 plus the accurate private-service-unavailable state. Registering `sw.js?build=verification-3-update` took control and removed the prior release cache.

The only legal-page console errors are the 404 defect described above.

## Performance and visual-system evidence

Exact candidate assets:

- JS: 72,551 bytes raw / 26,025 bytes gzip (budget ≤200 KB).
- CSS: 13,539 bytes raw / 3,917 bytes gzip (budget ≤50 KB).
- Mobile hero: 27,638 bytes WebP (budget ≤300 KB).
- No runtime font files or CDN fonts.

Fresh Lighthouse 13.4.1 mobile run against the exact live candidate:

| Category/metric | Result |
| --- | --- |
| Performance | 91 |
| Accessibility | 100 |
| Best Practices | 100 |
| SEO | 100 |
| FCP / LCP | 1.4 s / 1.9 s |
| CLS | 0.023 |
| TBT | 340 ms |
| Total transferred | 115 KiB |

The inspected desktop/mobile UI follows the recorded single-mode mid-century instrument-panel thesis, uses the documented palette/type/spacing, provides textual status alongside color, and discloses the generated original illustration. No generic gradient or third-party visual asset was found.

## Required next steps

1. Correct `tests/build-identity.sh` and prove the complete `npm test` command exits 0.
2. Serve `/privacy` and `/terms` with HTTP 200 and no console error on a first uncached navigation.
3. Add and test public API/workspace-initialization rate limiting.
4. Increase checklist and footer pointer targets to at least 44 × 44 CSS px.
5. Run the final local workflow with OBS 28+ and NVDA on Windows before claiming the brief's human screen-reader success measure.

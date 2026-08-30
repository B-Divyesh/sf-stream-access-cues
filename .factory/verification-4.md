# Independent verification 4 — **FAIL**

## Candidate and scope

- Candidate commit: `f0d372240ebc15197789c240eb74efc6ab1bea41`
- Live URL: `https://stream-access-cues.sociobot.in`
- Date: 2026-08-30
- Verdict: **FAIL — do not release or promote this candidate.**

The live `/health` response reported exactly `f0d372240ebc15197789c240eb74efc6ab1bea41`, so this is a fresh result for the requested deployment, not a stale deployment-only report.

## Release-blocking findings

### Critical — mandatory claims contract is absent

`.factory/claims.json` does not exist in the clean candidate. Therefore no claim tests could be run through the required demo entry point. This is explicitly release-blocking under the claims contract.

The landing page and README make testable claims about local-first privacy, keyboard operation, offline reload, private workspaces, and rate limits, yet none are listed in a claims file with an `@claim:<id>` demo test.

### Critical — there is no one-click isolated sample demo

Cold live landing has no **“Try it with sample data”** action, no persistent “Demo — sample data, nothing is saved” banner, and no reset/start-for-real controls. `/demo` returns HTTP 404; `/?demo=1` renders the normal app, not sample data. This fails the demo-sandbox acceptance contract independently of the missing claims file.

### Critical — live workspace persistence is split across independent backend instances

With one freshly generated valid `X-Operator-Key`, the live API accepted `PUT /api/checklist` with a unique one-item checklist (`200`). Twelve subsequent `GET /api/checklist` requests using the exact same key alternated between that saved item and the five untouched starter items. The sequence contained both states repeatedly.

This makes the spoken checklist unreliable for a live streamer and proves that the deployed SQLite persistence boundary is not shared across serving instances. It also explains a browser reproduction: checking “Set stream title and category” returned `200`, but a reload restored it to unchecked. The locally isolated release service persisted the same flow correctly, so this is a real deployment defect, not a client or test-data issue.

### High — first screen does not pass the plain-words first-read test

The cold first screen says **“Your stream, under your fingers.”** It is a metaphor rather than the job in plain words, does not name blind or keyboard-first independent streamers, and gives no first action that lets a visitor try the product. The first-read result was: an OBS-adjacent public setup guide with a timer/checklist, apparently for streamers; click “Run locally” or “View local setup.” It does not satisfy what it does, for whom, and what to click first in plain words.

### Medium — required crawler metadata files are missing

Live `/robots.txt` and `/sitemap.xml` both return `404`. Neither file is present in the candidate. The site-structure contract requires both.

## Clean-checkout quality gates

| Check | Result | Evidence |
| --- | --- | --- |
| `npm ci` | PASS | 175 packages installed; 0 audit vulnerabilities reported. |
| Every `.factory/claims.json` test | **FAIL** | File is missing; no required tests exist or can be run. |
| `npm test` | PASS | 3 Vitest tests, 8 Rust tests, and `tests/build-identity.sh` all passed. |
| `npm run check` | PASS | Svelte: 0 errors/0 warnings; `cargo clippy --all-targets -- -D warnings` passed. |
| Exact frontend production build | PASS | `BUILD_SHA=f0d372… npm run build`; `dist/` produced. JS: 72,551 B raw / 26.14 KB gzip; CSS: 14,118 B raw / 4.03 KB gzip. |
| Exact server production build | PASS | `BUILD_SHA=f0d372… npm run build:server` completed. |
| Browser suites | PASS locally | `npm run test:e2e` passed (16 local desktop/390px tests and 2 hosted-mode tests; Playwright last-run status `passed`). |
| Dependency audit | PASS | `npm audit --omit=dev`: 0 vulnerabilities. |

Docker is not installed in this verifier container, so no image was built or run. The repository build-identity runtime guard passed. Lighthouse is not installed in this checkout; it was not substituted with an unmeasured score.

## Product, input, and backend checks

An isolated local release service built with the candidate SHA passed the representative API flow:

- initial seeded checklist: 5 items;
- normal checklist save and reload: `200`, saved item retained;
- exactly 50 checklist items: `200`;
- 51 items: `400` with “A checklist can contain at most 50 items.”; subsequent read retained the 50 valid items;
- `obs_host: "ws://bad"`: `400` with specific host guidance;
- valid local settings: `200`; no OBS at `127.0.0.1:4455`: recoverable `502` with configuration guidance.

On the live hosted service, an OBS settings write correctly returned `403` and stated that OBS control is available only from the local service. This boundary is safe, but the deployed public product cannot perform the brief’s scene-change job and the mandatory demo is absent.

The live rate limit is enforced. A 100-request concurrent burst to `/api/runtime` from this verifier received 50 `200` and 50 `429`; every sampled `429` had `Retry-After: 1` and `Cache-Control: no-store`. The observed allowance was 50 successful requests in that burst (the documented configuration is 20 req/s, burst 40; refilling during the concurrent request window can affect the observed total).

## Live browser, privacy, PWA, and accessibility evidence

- Live root: `200`; one `<h1>`, `<main>`, header, navigation, and footer; normal cold load had no console or page errors.
- Outgoing cold-load browser requests were only to `https://stream-access-cues.sociobot.in` (document, same-origin assets, and same-origin API). No third-party scripts, fonts, analytics, or cross-origin calls appeared.
- Live privacy and terms routes: both `200`, one h1 each. Browser Axe had **0 serious/critical** findings at desktop and 390×844.
- Keyboard smoke: skip link received a visible `3px` amber focus ring; shortcut dialog opened/closed with keyboard; timer shortcut toggled start/pause; local-setup dialog was keyboard reachable. Reduced-motion dialog animation and transition computed to `0.00001s`.
- At 390px, `scrollWidth` equaled viewport width (390); no horizontal overflow. The source/browser suite checks 44px checklist/footer targets.
- PWA: after initial online load, an isolated browser context reloaded offline with the h1 present. Registering `/sw.js?build=qa-live-update` took control and left exactly `stream-access-cues-qa-live-update` in Cache Storage. Offline API fetches naturally logged `ERR_INTERNET_DISCONNECTED`; normal online load had no errors.
- Response policy: health/API `Cache-Control: no-store`; hashed JS/CSS `public, max-age=31536000, immutable`; root/service worker `no-cache`. Live headers included CSP with `frame-ancestors 'none'`, HSTS, `nosniff`, `DENY` framing, `no-referrer`, COOP, CORP, and restrictive Permissions-Policy.

`verify-url.sh` is not present in this repository, so it could not be run; the equivalent title/lang/h1/main/alt/button/console checks were performed directly in Playwright. NVDA and a real OBS instance are unavailable in this container; no claim of physical screen-reader or OBS scene-switch verification is made.

## Required next steps

1. Add `.factory/claims.json` and one clean-state demo-entry test for every public claim, including privacy, keyboard, offline, and persistence claims.
2. Build `/demo` (or `?demo=1`) as an isolated sample-data namespace with visible first-screen entry, demo banner, reset, and start-for-real controls.
3. Replace the public backend’s per-instance SQLite state with a shared durable store, or run exactly one durable instance with a verified volume, then repeat the same-key multi-request persistence test against the live URL.
4. Rewrite the first screen in plain words for blind/keyboard-first independent streamers, naming the job and first action.
5. Add and serve `robots.txt` and `sitemap.xml`.

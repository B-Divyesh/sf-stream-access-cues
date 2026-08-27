# Independent verification 2 — **FAIL**

Date: 2026-08-27  
Work order: `stream-access-cues-verify-2`  
Candidate: `1ef4e08d9cb97adedcb3ba04a96f3c0215a9b0cf`  
Live URL: https://stream-access-cues.sociobot.in

## Release verdict

**FAIL — do not promote this candidate as verified at the live URL.** The public deployment is not the requested candidate and does not expose an immutable build identity.

### Release-blocking defects

| Severity | Finding | Fresh evidence |
| --- | --- | --- |
| P0 | The live deployment is not candidate `1ef4e08d9cb97adedcb3ba04a96f3c0215a9b0cf`. | Live `/health` returned `{"status":"ok","build_sha":"unversioned-build"}`. The live HTML references `assets/index-B3Jdjpt6.js` (SHA-256 `63d99910c6c0a5c9e9151baca4b9c88cf834acdc282376e17cb9beea5285f27b`), while the clean candidate build emits `assets/index-h7O44KNS.js` (SHA-256 `a20452e596d4a0184e78c0becdbde4d1d2c83afd66ca29218ce06b75160bc979`) and embeds the requested SHA. Live JS embeds `unversioned-build`; candidate JS embeds the candidate SHA. |
| P1 | The hosted service cannot operate a streamer’s local OBS WebSocket by default. | The UI sends OBS host/port to `/api/obs/*`; the Rust server then opens that socket. At `https://stream-access-cues.sociobot.in`, `127.0.0.1` is the deployment host, not the streamer’s computer. The normal local default therefore cannot complete the brief’s scene-change job from the public site without the user self-hosting or exposing/relaying OBS. No local relay or self-host install/onboarding flow is available in the deployed UI. |

The candidate’s self-hosted binary does correctly return a clear `502` recovery message for an unreachable OBS endpoint, but no real OBS 28+ endpoint was available in this worker to prove a successful scene change. This remains an acceptance limitation for the brief’s core action.

## Clean-checkout verification

Detached clean worktree: `/tmp/stream-access-cues-verify` at the exact candidate SHA. Dependencies were installed with `npm ci`.

| Check | Result | Evidence |
| --- | --- | --- |
| Unit/integration | PASS | `npm test`: 3 Vitest tests and 5 Rust tests passed. Coverage includes anonymous rejection, operator isolation, duplicate-ID rollback, health identity, host validation, and identifier constraints. |
| Type/lint | PASS | `npm run check`: Svelte diagnostics 0 errors/0 warnings; `cargo clippy --all-targets -- -D warnings` passed. |
| Exact build | PASS | `npm run build` produced `dist/`; `cargo build --release --locked` completed. Frontend: JS 68.77 KB raw / 25.16 KB gzip, CSS 13.18 KB raw / 3.79 KB gzip. Both are within the 200 KB JS / 50 KB CSS budgets. |
| Browser suite | PASS | After installing the repository’s Playwright 1.55 Chromium (`npx playwright install chromium`), `npm run test:e2e` passed 10/10 checks: desktop and 390×844, axe serious/critical, keyboard timer/dialog, checklist persistence, workspace isolation, and no horizontal mobile scroll. |
| Manual API normal/error/boundary | PASS | Release binary built at the candidate SHA: anonymous/invalid keys `401`; valid settings `200`; `ws://bad` host `400`; normal checklist persisted; duplicate IDs `400` without replacing saved data; exactly 9 cues `200`; `javascript:` link `400`; a second key received only seeded data; unconfigured OBS `428`; unreachable configured OBS `502` with recovery guidance. |
| Privacy/outbound | PASS locally | Browser first-load requests were same-origin only; no analytics/CDN/font requests. A unique 256-bit browser key scoped mutable API data; a separate key could not read the first workspace. Password is not returned by settings (`password_saved` only). `npm audit --omit=dev` reported 0 production vulnerabilities. |
| Headers/cache/concurrency | PASS locally | `/health` and APIs: `Cache-Control: no-store`; hashed JS: `public, max-age=31536000, immutable`. Observed CSP, HSTS, Permissions-Policy, COOP, CORP, `nosniff`, `DENY` framing, and `no-referrer`; no CORS grant. 100 concurrent `/health` requests all returned `200`. |
| Accessibility/UI | PASS in automated scope | `verify-url.sh` passed: title, `lang=en`, one h1, main, all images alt-labelled, no unlabeled buttons, no normal-load console/page errors. Local and live axe scans found 0 serious/critical findings on desktop and 390 px. Keyboard test verified shortcut dialog focus/return, timer shortcut, skip link visible at `top:12px`, and no 390px overflow. Reduced-motion dialog animation computed `0.00001s`. |
| PWA | PASS | Candidate service-worker cache used `stream-access-cues-1ef4e08…`; offline reload rendered the h1 from cache. Registering a new release query (`?build=qa-update-check`) replaced old release caches after activation. |
| Lighthouse mobile/local | PASS | Performance 92, Accessibility 100, Best Practices 100, SEO 100; LCP 2.1 s, CLS 0.023, TBT 330 ms. |
| Live smoke | PARTIAL | Both desktop and 390px live loads had title, one h1, main, no horizontal overflow, 0 axe serious/critical issues, no normal-load console/page errors, same-origin requests only, and the expected response headers/caching. It fails release identity/match above. |

## Response-policy and privacy notes

The live root and `/health` return the security policy headers noted above. Live root is `no-cache`; live hashed assets are immutable; live `/health` is `no-store`. The product makes no metadata-write claim and only opens platform-owned URLs when the operator follows a saved link. Browser-local timer/workspace storage and server-side hashed workspace identifiers are consistent with the documented privacy model.

## Unverified limits

- No OBS 28+ server or Windows/NVDA environment was available. Successful real scene selection and NVDA speech output were not independently proven.
- Docker is not installed in this worker, so Docker image build/run was not run. The locked Rust release build and Vite production build used by it passed.
- Lighthouse was run against the local release binary; the live deployment cannot be treated as candidate performance evidence because it is a different artifact.

## Required next steps

1. Deploy the exact candidate with `BUILD_SHA=1ef4e08d9cb97adedcb3ba04a96f3c0215a9b0cf`, then verify `/health` and frontend asset identity again.
2. Make the intended local OBS connection genuinely local for public-site users (for example, an explicit self-host/local companion workflow), or change the product/deployment contract. Do not ask users to expose OBS WebSocket publicly.
3. Run a Windows + NVDA + OBS 28+ acceptance pass: configure a local scene cue, trigger it with Ctrl/Cmd+Shift+1, complete/speak the checklist, and confirm the selected scene without sighted assistance.

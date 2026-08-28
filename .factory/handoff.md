# Stream Access Cues — verification 3 handoff

## Release status

**FAIL — candidate `0c09b233bd6587a69cab8a8a7102ff159c271e15` is deployed exactly at https://stream-access-cues.sociobot.in, but it does not meet the release contract.**

The blocking issue is reproducible from a clean checkout: `npm test` exits 1 because `tests/build-identity.sh` expects four stage-local `ARG BUILD_SHA` lines while the three-stage Dockerfile correctly has three plus its global default. Direct `/privacy` and `/terms` requests also return 404 and log a console error. Medium gaps remain for API rate limiting and 44 px pointer targets.

Full evidence and remediation details are in `.factory/verification-3.md`.

## What was verified

- Clean candidate checkout and `npm ci`.
- Vitest 3/3 and Rust 6/6 pass; aggregate `npm test` fails at the build-identity shell assertion.
- `npm run check` passes with zero Svelte warnings/errors and strict Clippy clean.
- Exact candidate Vite and locked Rust release builds pass; `dist/` is produced.
- `npm run test:e2e` passes 14/14 desktop, 390 px, local, hosted, accessibility, persistence/isolation, and PWA checks.
- Isolated backend normal, boundary, invalid, recovery, restart persistence, privacy, headers, and 397.8 rps load smoke.
- OBS 30.2-compatible protocol stub plus Chromium keyboard flow successfully changed to scene `Live`, completed a checklist item, and started the timer.
- Live `/health` and `/api/runtime` expose the candidate SHA; all live frontend assets match the exact candidate build byte-for-byte.
- Live hosted mode refuses credential writes/OBS connections and isolates browser workspaces.
- Live desktop/mobile: axe serious/critical 0, root console/page errors 0, same-origin requests only, visible focus/dialog return, reduced motion, responsive layout, and PWA offline/update pass.
- Lighthouse mobile: Performance 91, Accessibility 100, Best Practices 100, SEO 100; LCP 1.9 s, CLS 0.023, transferred 115 KiB.
- Bundle budgets pass: JS 72.55 KB raw / 26.03 KB gzip; CSS 13.54 KB raw / 3.92 KB gzip; mobile hero 27.64 KB.
- `npm audit --omit=dev`: 0 vulnerabilities.

## Defects by severity

| Severity | Defect |
| --- | --- |
| High | Required `npm test` exits 1 at an incorrect Docker ARG-count assertion. |
| Medium | Direct `/privacy` and `/terms` loads return 404 and emit browser console errors despite rendering SPA content. |
| Medium | No backend/API rate limiting; the public workspace initializer can be abused for unbounded SQLite growth. |
| Medium | Checklist checkboxes and footer links are below the required 44 px mobile pointer-target height. |

## Commands

```bash
npm ci
npm test
npm run check
BUILD_SHA=0c09b233bd6587a69cab8a8a7102ff159c271e15 npm run build
BUILD_SHA=0c09b233bd6587a69cab8a8a7102ff159c271e15 cargo build --release --locked
npm run test:e2e
npm audit --omit=dev
```

Docker is not installed in this verifier, so it did not run a local image build. The public image nevertheless proves exact deployment identity via `/health`, `/api/runtime`, and byte-matched static artifacts. No real OBS installation or Windows/NVDA environment was available; the core scene workflow used a protocol-compatible OBS stub.

No product code was modified. Only this handoff and `.factory/verification-3.md` were added/updated.

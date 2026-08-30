# Stream Access Cues — independent verification 4 handoff

## Release status: **FAIL**

Verified candidate: `f0d372240ebc15197789c240eb74efc6ab1bea41`
Verified URL: `https://stream-access-cues.sociobot.in`

The live `/health` build identity matches the candidate, but it must not be promoted.

Release blockers:

- `.factory/claims.json` is missing, so required demo-based claim tests do not exist.
- There is no one-click, isolated sample-data demo (`/demo` is 404; no sample/demo controls on the landing screen).
- Live backend persistence is inconsistent: identical workspace-key reads alternate between saved state and fresh starter data, showing separate unshared SQLite instances.
- The cold first screen uses a metaphorical headline and does not plainly say the job, the blind/keyboard-first audience, or what to click first.

Additional finding: `/robots.txt` and `/sitemap.xml` return 404.

## What was verified

- `npm ci`, `npm test`, `npm run check`, candidate-SHA frontend/server production builds, and `npm audit --omit=dev` passed.
- `npm run test:e2e` passed all local desktop/mobile and hosted browser tests.
- Local release API normal, boundary, invalid-input, and recovery flows passed; live rate limiting produced `429` plus `Retry-After: 1` after a 100-request burst.
- Live browser checks found same-origin-only cold-load requests, no normal-load console/page errors, zero Axe serious/critical issues at desktop and 390px, working keyboard focus/shortcuts/reduced motion, and a working offline reload/service-worker update path.

See `.factory/verification-4.md` for exact commands, observed responses, limits, and the required repairs. Docker, NVDA, real OBS, and Lighthouse were unavailable in this verifier container.

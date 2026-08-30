# Stream Access Cues

Control an OBS stream with a keyboard. Stream Access Cues is for blind and keyboard-first independent streamers who need spoken preflight steps, named scene cues, a session timer, and direct links to their platform’s metadata page.

It is a Rust/Axum container that serves a Vite/Svelte interface on port 8080. Local mode is the product: it runs beside OBS and is the only mode allowed to save an OBS password or contact OBS WebSocket. The public site is a safe setup guide and sample workspace.

## Try the sample

Open [the demo](/demo) or select **Try it with sample data** on the first screen. It loads a Friday community-stream checklist, three simulated scene cues, and platform links. The persistent banner says **Demo — sample data, nothing is saved**.

The demo uses only the `demo:stream-access-cues.*` browser-storage namespace. **Reset demo** restores the bundled sample. **Start for real** discards it. The sample dashboard reloads offline after its first visit. Sample scene keys are previews; they never contact OBS.

On the public hosted guide, checklist, cue, and link edits stay in the current browser’s `stream-access-cues.hosted.workspace.v1` storage. The public service rejects all workspace writes, so a scaled container cannot split or lose that data. It also never contacts an OBS endpoint.

## Run the local companion

Requirements: Node.js 22+, Rust 1.88+, and OBS 28+ with **Tools → WebSocket Server Settings** enabled.

```bash
npm ci
npm run build
DEPLOYMENT_MODE=local cargo run
```

Open `http://localhost:8080`, choose **Connection**, then enter the local OBS host, port, and optional password. Local mode stores its SQLite file under `./data` by default; set `DATA_DIR` to choose another durable local directory. Each browser has a random private workspace key; the service stores only its SHA-256 digest.

For the production container path:

```bash
docker build --build-arg BUILD_SHA="$(git rev-parse HEAD)" -t stream-access-cues .
docker run --rm -p 8080:8080 -v stream-access-cues-data:/app/data -e DEPLOYMENT_MODE=local stream-access-cues
```

Do not expose an OBS WebSocket publicly. The image defaults to hosted-guide mode, so `DEPLOYMENT_MODE=local` is required when the container runs beside OBS.

## Keyboard controls

- Control/Command + Shift + 1–9: trigger the matching scene cue (or preview it in the demo)
- Control/Command + Shift + T: start or pause the timer
- Control/Command + Shift + R: open timer reset confirmation
- Control/Command + Shift + C: focus the next incomplete item
- Control/Command + Shift + S: speak scene, timer, and next item
- `?`: open the shortcut guide outside text fields

Keyboard shortcuts start the timer, preview sample cues, and open the shortcut guide.

## Privacy and limits

There are no accounts, analytics, advertising, tracking pixels, third-party scripts, or paid features. The public guide’s sample and workspace data remain in browser storage. In local mode, your data and OBS password remain in the local SQLite directory. See `/privacy` and `/terms` in the running app.

All public API routes are limited to 20 requests/second with a burst of 40 per first forwarded client IP. The next over-limit request receives `429 Too Many Requests` and `Retry-After`; `/health` is exempt.

## Verify

```bash
npm test
npm run check
npm run build
npm run build:server
npm run test:claims -- --grep @claim:demo-sample-isolated
npm run test:e2e
```

The claim registry is [`.factory/claims.json`](.factory/claims.json). It records an executable browser or integration test for each user-facing promise. `npm run test:e2e` runs Chromium at desktop and 390px, including keyboard, Axe, service-worker update, offline, hosted-boundary, and mobile checks.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `PORT` | `8080` | HTTP listen port |
| `DATA_DIR` | `data` | Local SQLite directory |
| `DIST_DIR` | `dist` | Built frontend directory |
| `BUILD_SHA` | `dev` | Value returned by `/health` |
| `DEPLOYMENT_MODE` | `local` outside Docker | `local` enables local OBS and SQLite workspaces; `hosted` is browser-local guide mode |

## License

MIT. See [LICENSE](LICENSE).

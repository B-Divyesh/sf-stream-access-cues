# Stream Access Cues

Stream Access Cues is a local-first, keyboard and screen-reader-friendly control surface for independent streamers. It connects to OBS WebSocket for named scene changes, keeps a spoken preflight checklist and session timer, and opens the platform-owned pages where stream metadata can actually be edited.

It does **not** stream video, replace OBS, write Twitch/YouTube metadata, or store OAuth tokens. There are no accounts, analytics, third-party scripts, or paid features.

## Who it is for

The primary user is a blind or keyboard-first solo streamer who cannot reliably use inaccessible embedded browser docks. The whole live surface is operable with native controls, announced state, and documented shortcuts.

## Requirements

- Node.js 22+ and npm
- Rust 1.88+ for the local service
- OBS 28+ with its WebSocket server enabled under **Tools → WebSocket Server Settings**
- Docker 24+ for the production container path

## Run locally

```bash
npm install
npm run build
npm run dev
```

The Vite UI runs at `http://localhost:5173` and proxies local API requests to the Rust service on port 8080. `npm run build` reproducibly writes the static frontend to `dist/`, with `dist/index.html` at its root.

The service stores its SQLite database under `./data` by default. Change that with `DATA_DIR`. Each browser creates a random 256-bit workspace key in its own local storage. The API stores only the SHA-256 digest of that key and uses it to scope settings, cues, checklist items, links, and OBS actions. A request without that key is rejected; another browser cannot read or overwrite the workspace.

Other configuration:

| Variable | Default | Purpose |
| --- | --- | --- |
| `PORT` | `8080` | HTTP listen port |
| `DATA_DIR` | `data` | SQLite persistence directory |
| `DIST_DIR` | `dist` | Built frontend directory |
| `RUST_LOG` | service info logs | Structured log filter |
| `BUILD_SHA` | checked-out Git SHA | Immutable value returned by `/health` |
| `DEPLOYMENT_MODE` | `local` outside Docker; `hosted` in the image | `local` is the only mode allowed to save OBS credentials or open OBS WebSocket connections. |

## Test and check

```bash
npm test
npm run check
npm run build
```

The test command runs frontend unit tests and Rust tests. The check command runs Svelte/TypeScript diagnostics and strict Rust Clippy.

## Production container

```bash
docker build --build-arg BUILD_SHA="$(git rev-parse HEAD)" -t stream-access-cues .
docker run --rm -p 8080:8080 -v stream-access-cues-data:/app/data -e DEPLOYMENT_MODE=local stream-access-cues
```

Open `http://localhost:8080`. This explicit local mode is the supported way to control OBS: it runs the service beside the operator’s OBS instance and keeps its OBS password in the local volume. When OBS runs on the Docker host, use `host.docker.internal` on Docker Desktop. On Linux, add `--add-host=host.docker.internal:host-gateway` or use a host network appropriate to your environment.

The image requires an explicit immutable `BUILD_SHA` (the command above uses the checked-out Git commit) and defaults to `DEPLOYMENT_MODE=hosted` for the public factory deployment. Hosted mode is an accessible setup guide only: it refuses OBS credential writes and all OBS network requests, because `127.0.0.1` on a public container is not the streamer’s computer. It still offers the independently isolated checklist and launch-link workspace, but it cannot change scenes. Never expose the OBS WebSocket publicly.

## Keyboard map

- Control/Command + Shift + 1–9: trigger scene cues
- Control/Command + Shift + T: start or pause the timer
- Control/Command + Shift + R: confirm timer reset
- Control/Command + Shift + C: focus the next incomplete checklist item
- Control/Command + Shift + S: speak current scene, time, and next item
- `?`: open the shortcut guide outside text fields

## Privacy and deployment

When self-hosted, the service and SQLite data remain on the operator’s machine. The public setup guide never accepts OBS credentials or contacts an OBS endpoint. Its optional checklist and links are isolated behind the browser-local private workspace key described above; they are never shared between visitors and the raw key is never persisted by the server. The browser also stores the timer and a release-versioned offline shell cache. See `/privacy` and `/terms` in the running app. The factory owns deployment, DNS, and infrastructure; this repository contains no deployment credentials.

## License

MIT. See [LICENSE](LICENSE).

# Demo sandbox

Open `/demo` or `/?demo=1` to enter the one-click sample workspace.

The sample is a Friday community-stream preflight: five spoken checklist items, three simulated scene cues (`Starting Soon`, `Camera + Game`, and `Be Right Back`), and links to Twitch Dashboard and YouTube Studio. Sample scene keys update only the visible sample status; they never send an OBS request.

Demo state is stored only under the `demo:stream-access-cues.workspace.v1` and `demo:stream-access-cues.timer.v1` browser-storage keys. It never reads the normal local-service workspace, the public-guide workspace, or the backend. **Reset demo** removes those keys and re-seeds the exact bundled sample. **Start for real** removes them before returning to `/`.

The service worker caches the shell after the first online visit, so the demo can reload offline with its sample data still available.

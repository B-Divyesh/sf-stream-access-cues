# Stream Access Cues — visual thesis

## Direction: the dependable mid-century instrument panel

The product should feel like a well-labelled control surface in a quiet broadcast booth: deliberate, tactile, and readable without being precious. The reference is 1950s–60s field equipment—warm enamel, engraved labels, recessed bays, mechanical counters—not a sci-fi cockpit or a visual macro deck. This is a useful metaphor for the product: every control has one job, every state is named in words, and the machine tells the operator exactly what happened.

This is an intentionally single-mode, low-glare dark treatment. A second colour theme would increase state ambiguity in a live tool, while the explicit dark canvas keeps the red/green status lamps and cream text stable in every room.

## Tokens

| Role | Token | Value | Reason |
| --- | --- | --- | --- |
| Background | `--ink` | `#171916` | Blackened olive chassis; low glare |
| Raised surface | `--panel` | `#282b25` | Painted steel panel |
| Recessed surface | `--well` | `#10120f` | Instrument wells and input fields |
| Primary text | `--paper` | `#f3eedb` | Aged cream labels; 14:1 on background |
| Muted text | `--muted` | `#c2bea9` | Still ≥7:1 on background |
| Accent | `--amber` | `#f3b84b` | Bakelite dial marker / active focus |
| Accent text | `--amber-ink` | `#17130b` | Dark ink on amber controls |
| Success | `--signal` | `#79d39b` | Green indicator lamp, always paired with text |
| Warning | `--warning` | `#f3b84b` | Amber lamp, always paired with text |
| Danger | `--danger` | `#ff8c7b` | Warm red lamp, always paired with text |
| Rule | `--rule` | `#5d6255` | Etched separator, ≥3:1 on wells |

Surfaces use no gradients. A very small noise texture is permitted only inside the generated illustration; live controls remain crisp for contrast and scaling.

## Type

- Display and major readings: Georgia with old-style authority; tabular figures enabled for the timer.
- Interface, labels, and body: system sans (`Inter`-like native stack) for fast local rendering and dependable screen-reader/browser behaviour.
- Scale: 12 px eyebrow, 14 px utility label, 16 px body minimum, 20 px section heading, 32–56 px page title, 48–88 px timer.
- Labels use sentence case. Only tiny panel legends use tracked uppercase, never whole paragraphs.

No runtime fonts are downloaded. This avoids a flash of invisible controls and keeps the first-load budget predictable.

## Spacing and construction

The base rhythm is 4 px; functional gaps use 8, 12, 16, 24, 32, and 48 px. The desktop control deck is a 12-column grid: checklist 5 columns, scene cues 7 columns, then a full-width timing/metadata rail. At 860 px it becomes one column in task order: status, timing, checklist, scenes, metadata. At 390 px, ornament and helper copy reduce, controls become full-width, and no action is removed.

Controls are at least 48 px high. Recessed wells use a 2 px dark inner line and a 1 px pale outer line. Primary controls are amber with a firm 2 px lower edge; active controls depress by 1 px. Status lamps include a shape/icon and plain text, never colour alone.

## Interaction grammar

- `Ctrl/Command + Shift + 1…9`: trigger named scene cues in displayed order.
- `Ctrl/Command + Shift + T`: start/pause the session timer.
- `Ctrl/Command + Shift + R`: reset the timer after confirmation.
- `Ctrl/Command + Shift + C`: move focus to the first incomplete checklist item.
- `Ctrl/Command + Shift + S`: speak the current status and next checklist item.
- `?`: open the shortcut guide when focus is not in a text field.

Every action produces both visible text and an `aria-live` announcement. Focus is a 3 px amber outline with a 3 px offset. Dialog focus is trapped, Escape closes, and focus returns to its launcher.

## Motion

Physical and brief: pressed controls translate 1 px; disclosure panels fade/translate no more than 8 px over 180 ms; status lamps transition over 160 ms. Nothing loops or flashes. Under `prefers-reduced-motion: reduce`, transforms and smooth scrolling are removed and state changes are immediate.

## Original asset plan and provenance

One editorial hero/empty-state illustration shows an abstract, human-free broadcast instrument panel from a slightly elevated angle: chunky cream and charcoal controls, amber scene keys, green status lamp, coiled cable, and a blank note card. It communicates “local tactile control” without pretending to show the actual UI. The image appears only in the onboarding/empty state, with meaningful alt text, and is responsive WebP/AVIF with explicit dimensions.

Prompt sheet:

> Use case: stylized-concept. Asset type: responsive onboarding illustration for an accessibility-first streaming control dashboard. Scene: a compact 1960s broadcast instrument panel on a dark olive desk, viewed from a slightly elevated three-quarter angle. Subject: tactile cream switches, three amber rectangular scene keys, one green status lamp, a mechanical timer dial, a short coiled cable, and a blank ivory cue card; no people. Style: refined editorial gouache with subtle screen-print grain and clean geometric silhouettes. Composition: landscape, main console biased slightly right with calm negative space at left, legible at small size. Lighting: warm pool of desk light, quiet late-night control room mood. Palette: charcoal olive, warm cream, safety amber, restrained signal green, tiny coral detail. Materials: powder-coated steel, bakelite, paper. Constraints: functional believable controls, no readable text, no numbers, no brands, no logos, no watermark, no UI screenshot, no gradients, no neon, no futuristic sci-fi, no photorealistic hands.

Generation tool/model: Factory image deployment via `/opt/fleet/lib/gen-image.sh` (Azure OpenAI image generation), generated 2026-08-27. The selected output and prompt sidecar live under `frontend/public/assets/`; source PNG is kept in `assets/src/`. The image is AI-generated original artwork; disclosed in the app footer.

All small icons are original inline SVG paths or typographic marks authored for this product, never icon-font or CDN assets.

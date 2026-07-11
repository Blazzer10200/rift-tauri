# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## Unreleased

- **One background, no picker** — the 12-variant background-texture system and the one-click Looks presets are gone (owner call: the extra textures stopped earning their place). The app keeps the single default dots field; Appearance is now just Accent color + Interface & code. Anyone who had picked another texture falls back to dots automatically; the old preference key is cleaned up on launch.

## v0.98.1 — Aurora hotfix

- **Home aurora no longer renders as a hard-edged rectangle** — the v0.98.0 gradient was still visibly green where its container clipped it, which read as a misplaced glowing box on real windows (JPEG-compressed dev screenshots hid the edge). The ellipse now fades to fully transparent inside its own bounds, slightly softer and calmer.

## v0.98.0 — Visual identity pass

- **One continuous top strip** — the sidebar's Rift row, the page title, and the window controls now sit in a single aligned 40px band: the sidebar divider and resize handle start *below* the band, and update banners span the full window *above* it (previously a banner shoved only the title row down and broke the line).
- **The composer visibly breathes while Claude works** — the old streaming glow never actually painted (the well's `overflow: hidden` clipped the halo pseudo-element to nothing). Rebuilt: a real outer halo on the card's own shadow, a breathe that never drops below half brightness, and a model-tinted **comet arc orbiting the frame** (~3.4s, masked conic gradient). Reduced-motion gets a steady ring.
- **Every project wears its own color** — a stable identity hue hashed from the project name (`projectHue`, unit-tested): workspace card monograms, project-switcher tiles, and the All-projects sidebar chips (now with a colored dot) all agree.
- **Home aurora** — a faint accent wash drifts behind the greeting; the first screen is no longer flat black. Follows your accent color.
- **Turn receipt is a pill** — "✓ Done · 39s · $1.88" renders as a quiet capsule, tinted by outcome (green for Applied, red for failed).
- **Workspace activity loads as a skeleton** — shimmer bars in the chart's shape instead of a spinner in a void (only visible on genuinely slow loads).
- **CDP wrapper hardening (dev tooling)** — a timed-out screenshot could leave the dev webview wedged at an emulated size (the real window rendered zoomed with cut-off edges); the override clear now verifies-and-retries, and `POST /reset-viewport` is the recovery.

## v0.97.0 — Split panes hardened + AI Health dashboard + /mcp dialog

Duplicate-pane crash fixed (self-heals poisoned saves); AI Health became a real dashboard (range picker, spend charts, per-model speed, pace forecast) with honest 24h "slow right now" verdicts; `/mcp` opens a full harness-wide server dialog; "Worked for Ns" is real wall-clock; short terminal tails render without stub reveals.

## v0.96.0 — Settings overhaul + chat display polish

Settings restructured into 5 searchable tabs with live previews and one-click Looks; About tab + update dialog rebuilt; skills in the `/` menu; tool display sharpened; dead code swept.


## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`) can't be recovered from Azure's servers — real fix is the on-device **Whisper** engine (built, not yet shipped).
- **Dismissed ask-cards** render "(no answer recorded)" instead of a "Dismissed" state — cosmetic, fix planned.

## Earlier

History via `git log -- docs/CHANGELOG.md`.

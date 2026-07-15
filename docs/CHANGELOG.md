# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## Unreleased — Composer & home revamp (Claude-Desktop-inspired)

- **The composer is a clean three-layer stack.** Workspace chips (folder · branch) float above a standalone input well; the controls sit on a flat quiet row below it. The old everything-in-one-box bar is gone.
- **The home screen is a launcher.** Idle home shows the greeting, compact recent-chat rows, and a pure input — no toolbar, no clutter. The moment you click in (or start dictation), the welcome drifts away and the composer glides down to its working position, assembling its controls mid-flight. Leave it empty and everything floats back.
- **Menus are professional-grade dense.** The model picker and permission menu share one flat panel style: one-line rows, plain hotkey digits, tooltips instead of inline blurbs. Effort is now a Faster↔Smarter slider — a groove with a light knob, one stop per real effort level. Explanations rewritten in plain language with a ? for details.
- **Quieter signals.** Permission tone lives on the icon (Bypass stays amber — it should be loud), chevrons appear on hover, the brain glyph and separator dots are gone, and the idle input rotates one quiet hint at a time instead of a shortcut list.
- **"Claude can make mistakes" moved to the status bar** — ambient info in the ambient bar, not under every composer.
- Fast mode's pay-per-use disclosure, effort mechanics, keyboard navigation, and all model/effort wiring are unchanged.

## v0.106.1 — Fast lane, honestly priced

- **Fast mode for Opus — with its price tag on.** A new toggle in the model picker (Opus rows only) runs Opus with noticeably quicker output — same model, same quality tier. **Important: fast mode is pay-per-use — it draws from your usage credits, not your plan limits.** The toggle says so in the row, turns amber (not green) when on, wears a "pay-per-use" tag, and pops a one-time warning when you enable it. Turns that actually ran fast get a ⚡ "fast" chip. Needs Claude CLI 2.1.209+. (v0.106.0 shipped this toggle without the billing disclosure — that gap is what this release fixes.)
- **Switching model or permission mode mid-chat is now instant.** Rift pushes the change to the already-running Claude process instead of restarting it — a model switch that used to cost ~1.5s of respawn now takes effect immediately. (Switching between Ask-style and Bypass-style permission modes still restarts — that change genuinely needs different launch flags.)
- **The context gauge now corrects itself from real data.** After each reply, Rift reads the context window the CLI actually ran against and uses that for the gauge — so accounts with a smaller window (e.g. Free plan) see honest numbers without configuring anything.
- Recommended Claude CLI version raised to 2.1.209 (fast mode + live switching + several upstream stream-json fixes).

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.
- **Dismissed ask-cards** render "(no answer recorded)" instead of a "Dismissed" state — cosmetic, fix planned.

## Earlier

History via `git log -- docs/CHANGELOG.md`.

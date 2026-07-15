# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.107.0 — Floating islands

- **The whole shell is floating islands now.** The sidebar and the main content are rounded, hairline-bordered cards sitting on a quiet canvas — Claude-Desktop-style — instead of edge-to-edge panels welded together.
- **Collapsing the sidebar melts it away completely** (no more skinny icon rail). A small cluster appears top-left: open, new chat, search. **Hover the panel glyph to peek the sidebar** — it floats in over your content without shifting anything; click to pin it back in place.
- **Dictation is one calm signal.** While recording, the mic is a soft red tile with a live waveform — the only thing that moves. The pulsing halo is gone, and the flashing auto-stop countdown pill is replaced by a thin amber ring that quietly drains around the button in the final seconds of silence (speech refills it).
- **The composer is a clean three-layer stack.** Workspace chips float above a standalone input well; controls sit on a flat quiet row below. The old everything-in-one-box bar is gone.
- **The home screen is a launcher.** Idle home shows the greeting, compact recent-chat rows, and a pure input. Click in (or start dictating) and the welcome drifts away while the composer glides down to its working position; leave it empty and everything floats back.
- **Menus are professional-grade dense.** Model picker + permission menu share one flat panel style: one-line rows, plain hotkey digits, tooltips instead of blurbs. Effort is a Faster↔Smarter slider. Permission tone lives on the icon (Bypass stays amber — it should be loud), and the idle input rotates one quiet hint at a time.
- **"Claude can make mistakes" moved to the status bar** — ambient info in the ambient bar.
- Fast mode's pay-per-use disclosure, effort mechanics, keyboard navigation, and all model/effort wiring are unchanged.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.
- **Dismissed ask-cards** render "(no answer recorded)" instead of a "Dismissed" state — cosmetic, fix planned.

## Earlier

History via `git log -- docs/CHANGELOG.md`.

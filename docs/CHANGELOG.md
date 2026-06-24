# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.30.1 — Hardening and accessibility polish

> A maintenance release: tighter security around how Rift handles links, file paths, and external content, a fix for a Windows-only path bug in the git tools, and a pass of accessibility improvements across dialogs and chat controls. Nothing changes in how the app looks or works day-to-day — it's just safer and reads better to screen readers.

- **Git tools now accept full file paths on Windows.** Fixed a Windows-only bug where passing an absolute path (like `C:\project\src\main.rs`) to the assistant's git tools could be wrongly rejected as "outside the workspace" — the path check didn't account for Windows' canonical path form. In-workspace absolute paths now work correctly, while paths that genuinely escape the workspace are still blocked.
- **Security hardening across several surfaces.** A round of defense-in-depth limits: the embedded browser's "add page to chat" now caps the page title and URL it captures, the "Open file" action refuses to launch executable scripts (`.py`, `.sh`, `.rb`, and friends — open them in an editor instead), and a few internal size limits and input checks were tightened so malformed or oversized data can't bloat memory.
- **Accessibility polish.** Buttons, menus, dialogs, and the download progress bar across the app now carry proper labels and roles, so screen readers announce them clearly. This includes the command palette, slash-command menu, file-action menus, the update dialog, and the project filter in the chat sidebar.
- **Internal cleanup.** Removed dead code paths and corrected some internal documentation. No change to how the app behaves for you.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.29.0** — AI Health: a coach for your Claude plan. New workspace tab (shortcut **5**) that reads how you actually use Claude through Rift, then asks **your own Claude** (your subscription, your private data) for a few plain-English, ranked suggestions grounded in your real numbers — with one-tap, fully-undoable apply for the settings it can tune. Only ever touches Rift's own settings, never your global Claude config.

- **v0.28.0** — Top-to-bottom hardening pass. No new features: a multi-wave multi-agent review swept the whole app (145 confirmed findings) and every fix landed here — prompt-injection sanitizing of page text, a dozen more rejected executable types, ring-capped telemetry/transcript/model buffers, torn-down stray timers/listeners, and a pile of subtle correctness fixes (init races, queued-steer reorder, double-fired slash-menu Enter, stalled stats clock). It just holds up better the longer you run it.

- **v0.27.1** — Activity stats panel (messages/sessions/tools/spend, streaks, peak hour, 12-week heatmap, per-model breakdown, All/30d/7d toggle), drag a sidebar chat straight into a split pane, and removal of an unreachable old home screen.

- **v0.27.0** — Faster replies on every model (Smart → balanced reasoning that streams immediately, especially on Sonnet; retired the auto-downshift cold-restart), a sidebar that stops reshuffling on open/click, finished split-view (top-bar "Split editor" button, no overlap at 3–4 panes, per-pane project folders), cleaner collapsed tool rows + a calm "waiting for you" footer on ask_user.

- **v0.25.0–v0.26.3** — The warm-CLI era: a persistent child process per session (first reply ~1400ms → ~5ms after), 30-minute idle survival, transparent respawn; honest API-stall watchdog (slow ≠ Rift); interactive `ask_user` cards in stream mode; colored context ring + "this conversation" usage row; sub-agent activity dock.
- **v0.20.7–v0.24.0** — The redesign + hardening foundation: full redesign port (all 7 surfaces rebuilt to spec), stream design-language pass + live-turn polish, latency auto-scale, per-project chat scoping, and ~70 fixes across rounds 5–12. Detail via `git log`.

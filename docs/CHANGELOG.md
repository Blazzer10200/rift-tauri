# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.31.1 — Project globs that actually match

> A fix on top of yesterday's Projects release. The include/exclude patterns you set on a project now match folders the way every glob is supposed to — so the scoping you configure is the scoping you get.

- **`**` now spans whole folders, including none.** A pattern like `src/**/*.ts` matched files one level deep but quietly skipped `src/main.ts` at the top; an exclude like `vendor/**` hid files inside `vendor` but left the folder itself listed. Both are fixed: `**` now correctly means "any depth, including zero," and a trailing `dir/**` covers the folder and everything in it. If you set up a project in v0.31.0 and the file scoping looked off, this is why — it's right now.
- **Bad patterns are caught when you save, not silently ignored later.** A glob that can't compile is rejected with a clear message at save time instead of no-op'ing invisibly during file reads.
- **A wedged turn can't outlast its own safety net.** The stall watchdog no longer re-arms forever while a tool is "in flight" — if a tool starts but never reports back and the process goes silent, the turn now ends honestly instead of hanging.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.31.0** — Projects, and a single Workspace home: named folder aliases with their own include/exclude file scoping, switchable in a click; Home and Projects merged into one **Workspace** page (greeting, current-folder context, resume-a-chat cards, full project manager); and honest stalled-turn handling that names the real cause instead of blaming the API.

- **v0.30.1** — Hardening and accessibility polish: Windows full-path fix in the git tools, defense-in-depth size/input caps, executable-open refusals, and proper screen-reader labels/roles across dialogs, menus, and the command palette.

- **v0.29.0** — AI Health: a coach for your Claude plan. New workspace tab (shortcut **5**) that reads how you actually use Claude through Rift, then asks **your own Claude** (your subscription, your private data) for a few plain-English, ranked suggestions grounded in your real numbers — with one-tap, fully-undoable apply for the settings it can tune. Only ever touches Rift's own settings, never your global Claude config.

- **v0.28.0** — Top-to-bottom hardening pass. No new features: a multi-wave multi-agent review swept the whole app (145 confirmed findings) and every fix landed here — prompt-injection sanitizing of page text, a dozen more rejected executable types, ring-capped telemetry/transcript/model buffers, torn-down stray timers/listeners, and a pile of subtle correctness fixes (init races, queued-steer reorder, double-fired slash-menu Enter, stalled stats clock). It just holds up better the longer you run it.

- **v0.27.1** — Activity stats panel (messages/sessions/tools/spend, streaks, peak hour, 12-week heatmap, per-model breakdown, All/30d/7d toggle), drag a sidebar chat straight into a split pane, and removal of an unreachable old home screen.

- **v0.27.0** — Faster replies on every model (Smart → balanced reasoning that streams immediately, especially on Sonnet; retired the auto-downshift cold-restart), a sidebar that stops reshuffling on open/click, finished split-view (top-bar "Split editor" button, no overlap at 3–4 panes, per-pane project folders), cleaner collapsed tool rows + a calm "waiting for you" footer on ask_user.

- **v0.25.0–v0.26.3** — The warm-CLI era: a persistent child process per session (first reply ~1400ms → ~5ms after), 30-minute idle survival, transparent respawn; honest API-stall watchdog (slow ≠ Rift); interactive `ask_user` cards in stream mode; colored context ring + "this conversation" usage row; sub-agent activity dock.
- **v0.20.7–v0.24.0** — The redesign + hardening foundation: full redesign port (all 7 surfaces rebuilt to spec), stream design-language pass + live-turn polish, latency auto-scale, per-project chat scoping, and ~70 fixes across rounds 5–12. Detail via `git log`.

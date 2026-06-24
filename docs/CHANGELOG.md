# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.31.2 — Project editor that tells you what's wrong

> Polish on the Projects feature: the editor now catches bad glob patterns as you type, surfaces save/load problems clearly instead of swallowing or double-printing them, and a load failure no longer masquerades as "no projects yet."

- **Live glob validation.** The include/exclude boxes now flag an invalid pattern inline — a red outline and a count ("1 invalid · too long") right under the field — and disable Save until it's fixed, so you never round-trip to the backend just to find out a pattern was malformed. The check mirrors the real matcher dialect exactly.
- **Clearer error messages.** Save and delete failures used to show a doubled, jargon-y string ("Save failed: Save project failed: …"); now you get the plain reason ("not a directory: …", "project limit reached"). A successful delete no longer mistakenly reports a stale earlier error.
- **A failed project load looks like a failure, not an empty list.** If the project store can't be read, the Workspace page now shows "Couldn't load projects" with the reason and a Retry button — instead of silently showing "No projects yet" as if you'd never made any.
- **Under the hood:** the project registry, the Workspace migration, and the new glob validator went from zero tests to a full suite (100+ cases), so these paths are now regression-guarded.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.31.1** — Project globs that actually match: `**` now spans zero-or-more folders (so `src/**/*.ts` catches top-level files and `vendor/**` hides the folder itself), invalid patterns are rejected at save time, and the stall watchdog can no longer re-arm forever while a tool hangs.

- **v0.31.0** — Projects, and a single Workspace home: named folder aliases with their own include/exclude file scoping, switchable in a click; Home and Projects merged into one **Workspace** page (greeting, current-folder context, resume-a-chat cards, full project manager); and honest stalled-turn handling that names the real cause instead of blaming the API.

- **v0.30.1** — Hardening and accessibility polish: Windows full-path fix in the git tools, defense-in-depth size/input caps, executable-open refusals, and proper screen-reader labels/roles across dialogs, menus, and the command palette.

- **v0.29.0** — AI Health: a coach for your Claude plan. New workspace tab (shortcut **5**) that reads how you actually use Claude through Rift, then asks **your own Claude** (your subscription, your private data) for a few plain-English, ranked suggestions grounded in your real numbers — with one-tap, fully-undoable apply for the settings it can tune. Only ever touches Rift's own settings, never your global Claude config.

- **v0.28.0** — Top-to-bottom hardening pass. No new features: a multi-wave multi-agent review swept the whole app (145 confirmed findings) and every fix landed here — prompt-injection sanitizing of page text, a dozen more rejected executable types, ring-capped telemetry/transcript/model buffers, torn-down stray timers/listeners, and a pile of subtle correctness fixes (init races, queued-steer reorder, double-fired slash-menu Enter, stalled stats clock). It just holds up better the longer you run it.

- **v0.27.1** — Activity stats panel (messages/sessions/tools/spend, streaks, peak hour, 12-week heatmap, per-model breakdown, All/30d/7d toggle), drag a sidebar chat straight into a split pane, and removal of an unreachable old home screen.

- **v0.27.0** — Faster replies on every model (Smart → balanced reasoning that streams immediately, especially on Sonnet; retired the auto-downshift cold-restart), a sidebar that stops reshuffling on open/click, finished split-view (top-bar "Split editor" button, no overlap at 3–4 panes, per-pane project folders), cleaner collapsed tool rows + a calm "waiting for you" footer on ask_user.

- **v0.25.0–v0.26.3** — The warm-CLI era: a persistent child process per session (first reply ~1400ms → ~5ms after), 30-minute idle survival, transparent respawn; honest API-stall watchdog (slow ≠ Rift); interactive `ask_user` cards in stream mode; colored context ring + "this conversation" usage row; sub-agent activity dock.
- **v0.20.7–v0.24.0** — The redesign + hardening foundation: full redesign port (all 7 surfaces rebuilt to spec), stream design-language pass + live-turn polish, latency auto-scale, per-project chat scoping, and ~70 fixes across rounds 5–12. Detail via `git log`.

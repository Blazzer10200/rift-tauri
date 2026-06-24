# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.31.0 — Projects, and a single Workspace home

> Two things this release. First: **named projects** — give any folder a name, scope exactly which files Rift can see inside it with include/exclude globs, and switch between them in a click. Second: **Home and Projects are now one page — your Workspace.** Plus a reliability fix so a stuck turn ends honestly instead of hanging forever on a misleading "waiting on the API" message.

- **Projects: name a folder, scope what Rift sees.** A project is a named alias for a workspace folder plus its own include/exclude file patterns. Those patterns constrain everything the assistant touches there — file reads, directory listings, grep, and the @-mention picker — so Claude works against exactly the files that matter and ignores the noise (build output, lockfiles, `node_modules`, whatever you list). Create, edit, and delete projects from the Workspace page; the active project's name now labels its conversations in the sidebar.
- **One Workspace page (Home + Projects, merged).** The separate Home and Projects nav entries are gone, replaced by a single **Workspace** destination. It opens with a time-of-day greeting and your current folder's context (branch, file count), cards to pick up recent conversations where you left off, and the full project manager below — create a project, switch folders, or jump back into work, all from one place. Chat stays one click away and the in-chat welcome screen is unchanged.
- **Honest stalled-turn handling — no more fake "waiting on the API."** If a turn produces nothing for three minutes with no tool running, Rift now ends it with a truthful message: this is the local Claude process stalling (a hung start, a stuck tool, a dropped pipe), *not* a slow model or the Anthropic API. The waiting indicator was rewritten to match — it no longer claims to know a cause it can't actually see. A genuinely wedged turn can no longer hang indefinitely.
- **Switching to a project is everything-aware.** Opening a project points the active workspace root at its folder, so turns, file scoping, @-mentions, and conversation filtering all follow automatically — no extra setup.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.30.1** — Hardening and accessibility polish: Windows full-path fix in the git tools, defense-in-depth size/input caps, executable-open refusals, and proper screen-reader labels/roles across dialogs, menus, and the command palette.

- **v0.29.0** — AI Health: a coach for your Claude plan. New workspace tab (shortcut **5**) that reads how you actually use Claude through Rift, then asks **your own Claude** (your subscription, your private data) for a few plain-English, ranked suggestions grounded in your real numbers — with one-tap, fully-undoable apply for the settings it can tune. Only ever touches Rift's own settings, never your global Claude config.

- **v0.28.0** — Top-to-bottom hardening pass. No new features: a multi-wave multi-agent review swept the whole app (145 confirmed findings) and every fix landed here — prompt-injection sanitizing of page text, a dozen more rejected executable types, ring-capped telemetry/transcript/model buffers, torn-down stray timers/listeners, and a pile of subtle correctness fixes (init races, queued-steer reorder, double-fired slash-menu Enter, stalled stats clock). It just holds up better the longer you run it.

- **v0.27.1** — Activity stats panel (messages/sessions/tools/spend, streaks, peak hour, 12-week heatmap, per-model breakdown, All/30d/7d toggle), drag a sidebar chat straight into a split pane, and removal of an unreachable old home screen.

- **v0.27.0** — Faster replies on every model (Smart → balanced reasoning that streams immediately, especially on Sonnet; retired the auto-downshift cold-restart), a sidebar that stops reshuffling on open/click, finished split-view (top-bar "Split editor" button, no overlap at 3–4 panes, per-pane project folders), cleaner collapsed tool rows + a calm "waiting for you" footer on ask_user.

- **v0.25.0–v0.26.3** — The warm-CLI era: a persistent child process per session (first reply ~1400ms → ~5ms after), 30-minute idle survival, transparent respawn; honest API-stall watchdog (slow ≠ Rift); interactive `ask_user` cards in stream mode; colored context ring + "this conversation" usage row; sub-agent activity dock.
- **v0.20.7–v0.24.0** — The redesign + hardening foundation: full redesign port (all 7 surfaces rebuilt to spec), stream design-language pass + live-turn polish, latency auto-scale, per-project chat scoping, and ~70 fixes across rounds 5–12. Detail via `git log`.

# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.30.0 — Attach files, smarter starters, and a Performance panel

> A round of quality-of-life upgrades for everyday work: pull real files into a message, get starter prompts that fit your project, and see how fast Rift is actually replying — plus the spend cap now behaves correctly for subscription plans.

- **Attach text files to a message.** Drop, paste, or click the paperclip to attach text files — logs, configs, source — into the composer. They show up as chips with the filename and size, and when you send, their contents are included automatically as formatted code blocks so the assistant sees them verbatim. Files over 256 KB still go through but are trimmed with a note; the total text across all attachments is capped at 1 MB per message. Binary files (executables, archives, video) are skipped.
- **Quick-start chips that fit your stack.** The starter chips on an empty tab now adapt to the open project. Node, Rust, Python, and Go projects each get their own set of starters — dependency audits, entry-point traces, error-handling scans — detected automatically from files like `package.json`, `Cargo.toml`, or `go.mod`. The generic "map / explain / find TODOs" set now appears only when no known stack is recognized.
- **A Performance card in AI Health.** Once you have a few turns under your belt (3+), AI Health shows how Rift is actually performing: p50/p90 time-to-first-reply, typical turn time, cache-hit rate, total tokens generated, turns measured, and a 7-day cost-trend bar — all from per-turn records Rift now keeps locally in the background.
- **The spend cap now respects subscription billing.** The per-turn dollar cap (and any advice about it) only appears when you're connected with an API key. On a subscription plan the Cost-guard card is hidden entirely, and AI Health stops suggesting a dollar-per-turn limit — instead it frames advice around your plan's rate-limit window, since subscriptions are governed by rate limits, not per-turn dollars.
- **Friendlier voice-input setup.** When the on-device Whisper engine isn't built into your copy of Rift, the voice settings now show a calm note ("Web Speech is selected for you and works right now — no setup, no download") with the developer build steps tucked behind a collapsible section, instead of a raw command-line warning. And if your browser doesn't support Web Speech either, the message now points you to that note rather than a dead end.
- **Internal cleanup.** Removed a non-functional "Fast Mode" toggle (the underlying capability never existed), plus an orphaned navigate command and an unused export command. No change to how the app behaves for you.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.29.0** — AI Health: a coach for your Claude plan. New workspace tab (shortcut **5**) that reads how you actually use Claude through Rift, then asks **your own Claude** (your subscription, your private data) for a few plain-English, ranked suggestions grounded in your real numbers — with one-tap, fully-undoable apply for the settings it can tune. Only ever touches Rift's own settings, never your global Claude config.

- **v0.28.0** — Top-to-bottom hardening pass. No new features: a multi-wave multi-agent review swept the whole app (145 confirmed findings) and every fix landed here — prompt-injection sanitizing of page text, a dozen more rejected executable types, ring-capped telemetry/transcript/model buffers, torn-down stray timers/listeners, and a pile of subtle correctness fixes (init races, queued-steer reorder, double-fired slash-menu Enter, stalled stats clock). It just holds up better the longer you run it.

- **v0.27.1** — Activity stats panel (messages/sessions/tools/spend, streaks, peak hour, 12-week heatmap, per-model breakdown, All/30d/7d toggle), drag a sidebar chat straight into a split pane, and removal of an unreachable old home screen.

- **v0.27.0** — Faster replies on every model (Smart → balanced reasoning that streams immediately, especially on Sonnet; retired the auto-downshift cold-restart), a sidebar that stops reshuffling on open/click, finished split-view (top-bar "Split editor" button, no overlap at 3–4 panes, per-pane project folders), cleaner collapsed tool rows + a calm "waiting for you" footer on ask_user.

- **v0.25.0–v0.26.3** — The warm-CLI era: a persistent child process per session (first reply ~1400ms → ~5ms after), 30-minute idle survival, transparent respawn; honest API-stall watchdog (slow ≠ Rift); interactive `ask_user` cards in stream mode; colored context ring + "this conversation" usage row; sub-agent activity dock.
- **v0.20.7–v0.24.0** — The redesign + hardening foundation: full redesign port (all 7 surfaces rebuilt to spec), stream design-language pass + live-turn polish, latency auto-scale, per-project chat scoping, and ~70 fixes across rounds 5–12. Detail via `git log`.

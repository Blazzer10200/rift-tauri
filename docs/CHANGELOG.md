# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.52.1 — A faster, sharper assistant

> The assistant felt slow and worked one step at a time. Two root causes, both fixed: it was thinking on every turn even when thinking was off, and it was never told to use its tools in parallel.

### Speed
- **No more silent "thinking" pause.** A turn that didn't carry the thinking setting was defaulting to thinking-on, so every reply sat through a 6–8s hidden reasoning pass before any text appeared (on Opus the thinking is invisible, so it just looked frozen). The default now matches the rest of the app — thinking is off unless you turn it on — and a clear **"Thinking…"** label shows during the pause when it is on, so it never reads as a hang.
- **Tools now run in parallel.** The assistant was making tool calls one at a time — read a file, wait, read the next, wait — which felt slow on anything multi-step. It now batches independent calls (opening several files, running independent searches or checks) into a single round-trip, so multi-step work moves at a noticeably faster clip.

### Sharper edits
- **Edits fit your codebase.** New code now follows the surrounding file's naming, formatting, and comment style instead of imposing its own, sticks to exactly what you asked for (no surprise refactors of nearby code), and stops adding unrequested explanatory comments.

### Quieter internals
- **No more update-check error spam.** On some installs the updater logged the same "couldn't locate manifest" error every few seconds (hundreds of identical lines a day); it now logs the condition once and stays quiet.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.52.0** — Voice mode overhaul: a real mic meter driven by your actual voice level (Web Speech + on-device Whisper), **Ctrl+D** to dictate (tap to toggle or hold for push-to-talk), a "stopping soon" countdown for auto-stop-on-silence, and transcript cleanup moved onto a current model.

- **v0.51.3** — Haiku 4.5 removed from the model picker (Anthropic pulled it); anything pinned to Haiku falls back to Sonnet, older Haiku chats still render. Reversible kill-switch, same as Fable.
- **v0.51.2** — A correctness batch from a three-pass audit: background-task promises that never resolved, the plan/Tasks card surviving a reload, the model-picker flyout keyboard behavior, prompt-enhance timeouts, cross-chat state leaks, clean stop-mid-turn records, and a pile of smaller reactivity fixes.
- **v0.51.1** — Sub-agent panel overhaul: a live "now doing" line per helper agent, a recent-steps trail, an alive header, and no more overlapping the chat.

- **v0.51.0** — A faster assistant + a redesigned picker: fixed the "everything is slow" regression (a flipped permission default was parking every tool on an approval the UI never showed), turned extended thinking off by default, rebuilt the model + effort picker in Rift's design language (identity icons, a "More models" flyout, an honest Low→X-High effort dial), and revived the Tasks/Plan panel after a CLI tool-rename broke its allow-list.

- **v0.50.0** — The biggest release in a while: the Workspace page became a real bento **dashboard** (your activity inline + a "What's new in AI" feed + a richer Projects panel), the **AI Health** tab grew into a genuine diagnostic (plain-English health score + warm-aware root-cause latency), and a broad **cross-machine compatibility** pass (safer first-run default, corporate-TLS trust, domain/locked-down machines, smaller screens, honest errors) lets Rift run cleanly on other people's Windows PCs.

- **v0.36.2** — Fable 5 is back: Anthropic's limited-run Fable 5 model returned, so it's once more an option in the model picker with its full 1M-token context and effort range.

- **v0.36.1** — The Claude CLI update actually updates now: clicking "Update" on the CLI bar would spin and then snap right back to the same prompt without moving the version (a running background Claude process held the CLI's file locked on Windows, and the bar read the stale version). Now Rift shuts those processes down before updating and re-checks right after. Plus quieter internals — bounded background-agent history and a handful of small correctness fixes.

- **v0.36.0** — One place for everything that happens: a real **notification center** (a bell in the toolbar with history grouped by when, persisting across restarts), updates moved into an always-visible top bar instead of the corner, redesigned toasts, a calmer chat surface (collapsing long code blocks, stream spacing polish), and a fix for a burst of internal errors on startup.

- **v0.35.0** — A reliability pass on the quiet edges: a stuck turn surfaces instead of hanging forever (dead-pipe detection + retry on a fresh process), the auto-updater recovers from a poisoned internal lock, CLI-update checks read as "Checking…" instead of a stale result, and git-timeout kills are scoped to git.

- **v0.31.2–v0.34.0** — Windows that stay in sync (multi-window, one per monitor; conversation list synced across all), **Use my full Claude Code config** (inherit your global `~/.claude` setup), a ground-up Workspace redesign + adopt-an-existing-folder, and a project editor that tells you what's wrong (live glob validation, plain-English errors).

- **v0.27.0–v0.31.1** — Projects + Workspace era: named folder aliases with include/exclude scoping + a merged **Workspace** home, glob matching that actually works (`**` spans folders), AI Health (your own Claude coaches your plan), a 145-finding hardening pass, accessibility polish, the activity stats panel, and finished split-view.

- **v0.20.7–v0.26.3** — Foundation era (detail via `git log`): full redesign port (all 7 surfaces to spec) + stream design language, the warm-CLI process (first reply ~1400ms → ~5ms after, 30-min idle survival, transparent respawn), honest API-stall watchdog, interactive `ask_user` cards, context ring, sub-agent activity dock, latency auto-scale, per-project chat scoping.

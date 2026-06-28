# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.64.0 — Runs on your machine, not just mine

> A pass focused on everyone who isn't the developer: Rift now adapts to smaller laptops and scaled displays, the first-run setup no longer leaves new users stranded, error messages talk like a human, and the diagnostics console got a proper visual glow-up.

### What you'll notice
- **The window adapts to your screen.** On smaller laptops and high-DPI / 150%-scaled displays, the sidebar now auto-tucks itself away when the window gets narrow (and comes back when you widen it) so the chat keeps room. It remembers whether *you* chose to collapse it, so widening never forces open a rail you'd closed. Split-pane also stops you opening a fourth pane when it would just produce unusable slivers — with a note telling you to widen or collapse the sidebar first.
- **First-run setup no longer dead-ends.** If you skipped setup and later can't send, the "set up Claude" screen now gives you the *full* guided connect — copy-paste install commands (PowerShell **and** an npm fallback for locked-down machines), one-click sign-in, an API-key field, and live auto-detection — instead of a stripped-down prompt. Plus a reminder to relaunch after installing so PATH changes take effect, and pressing Escape mid-setup no longer throws the whole thing away.
- **Errors talk like a human.** When something fails — an update, a sign-in, a file that's too big — you now get plain language ("The request timed out — check your connection", "Those images are too large — keep attachments under 20 MB") instead of a raw Rust/Tauri error chain. Genuinely unknown errors still show through (cleaned up), so nothing important is hidden.
- **A sharper diagnostics console.** The console (Settings → About → "Open diagnostics console") got a real redesign: an at-a-glance health verdict in the header, subsystem **vital-sign cards** (each showing its live status + a one-line summary), tidier colour-coded log rows, and a status bar with live/paused state and error counts. It now looks like a part of Rift, not a generic log dump.

### Under the hood
- **Honest, reusable error handling.** A new `humanizeError` helper maps common failure shapes (timeout, TLS/proxy, DNS, auth, locked file, disk-full) to friendly guidance and scrubs your username from any leaked path; eight raw error-leak sites now route through it.
- **A11y, already solid.** A keyboard/screen-reader review of the custom controls (effort slider, code-copy buttons, context menus) found them already accessible — no regressions introduced, nothing churned.
- **Verified.** Type-checks clean (4134 files), 376 unit tests pass (7 new for the error humanizer), backend compiles clean, and every change was confirmed live in the running app via the dev tooling.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.63.0** — The app can tell you where it hurts: a live diagnostics console with per-subsystem green/amber/red health, full app instrumentation (8 subsystems, structured timed events), a reusable `metric!`/`timed!` primitive, and an enhance-prompt wand reworked as a faithful, faster translation layer.

- **v0.62.0** — Honest about where the time goes: proved slow replies are Claude thinking, not Rift (warm TTFT 0–2ms, ~93% of a turn is the API), fixed the false "stuck" watchdog during silent reasoning, and added model-vs-Rift timing to AI Health from the CLI's own server-side numbers.

- **v0.61.0** — Honest, detailed tool display: tool rows name what actually ran ("Searched 2 · read 1"), tidy answered-question chips, delegated-helper states read honestly; plus a pooled fast-path connection and boot-time CLI capability check.

- **v0.60.0** — Spring cleaning: a deep tidy-up of Rift's own code (dead-code sweep + splitting the biggest source files into focused pieces), no behavior change, verified pixel-identical top to bottom.

- **v0.53.0** — Instant first reply: the first message of a new chat is now warm too. Rift quietly spins up a ready Claude process in the background and adopts it the moment you send, so the cold-start pause (re-loading your global config/hooks/tools, often 9–10s) is already paid. The warm pool grew 3→6 with a wider "still active" window, and a process leak on retirement was fixed.

- **v0.52.1** — A faster, sharper assistant: stopped a silent 6–8s "thinking" pause on every reply (thinking now defaults off, with a clear "Thinking…" label when it's on), made the assistant batch independent tool calls in parallel instead of one-at-a-time, tightened edits to match your codebase and stay in scope, and silenced repetitive update-check error spam.

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

- **v0.20.7–v0.26.3** — Foundation era (detail via `git log`): full redesign port (all 7 surfaces to spec) + stream design language, the warm-CLI process (first reply ~1400ms → ~5ms after, 30-min idle survival, transparent respawn), honest API-stall watchdog, interactive `ask_user` cards, context ring, sub-agent activity dock, latency auto-scale, per-project chat scoping.

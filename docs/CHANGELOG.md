# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.62.0 — Honest about where the time goes

> This one settles the question of why a reply sometimes feels slow. Short version: it's almost always Claude thinking, not Rift — and now the app proves it instead of asking you to take our word. Plus a real reliability fix to the "is it stuck?" indicator.

### What you'll notice
- **No more false "stuck" warnings while Claude is thinking.** On the newest Claude models, a deep reasoning pass streams silently (no visible tokens) — and Rift's "still working?" watchdog mistook that silence for a hang, flashing "Waiting on the model · 45s" while the header still said "Thinking…". The two contradicted each other. Now the watchdog knows an active think isn't a stall, so the indicator stays honest.
- **AI Health now shows model-vs-Rift timing.** The Speed & efficiency card adds one plain-English line, computed from your own replies: *"about 93% is Claude thinking (17.0s); Rift's own overhead adds just 1.4s. The wait is the model, not the app."* It only appears when the model genuinely dominates, so it's always true for your data — the honest answer to "why did that take a while?"

### Under the hood
- **Rift now records the CLI's own server-side timing.** Every reply, the Claude CLI reports its true time-to-first-token and total API time; Rift was discarding both and measuring only its own wall-clock. Now it captures them, logs a per-turn attribution line (model time vs Rift's plumbing overhead), and feeds the AI Health pane — so latency questions are answered from data, not guesswork.
- **Measured, not assumed.** This release came out of a latency hunt: instrumented per-tool gaps (all model think-time), and a head-to-head against the bare CLI confirming Rift's warm pool turns a ~13s cold start into near-zero and adds nothing on top. The drag you feel is the model reasoning — the same on any Claude client.
- **Verified.** Full backend suite (112 tests) + 2 new tests for the timing math, type checks clean, and the new AI Health line confirmed rendering correctly in the running app.

## v0.61.0 — Honest, detailed tool display + a faster path

> This one is about Rift *showing its work* clearly. When the assistant reads, searches, and runs things, you now see exactly what happened — and a couple of quiet speed wins land too. Nothing you do changes.

### What you'll notice
- **Tool rows name what actually ran.** A batch that mixed reading and searching used to collapse into a vague "Ran 6 steps." Now it reads what it did, biggest-first — e.g. **"Searched 2 · read 1"** — so you can tell at a glance whether the assistant is using the right tools. File-listing and filename-pattern searches also get their own folder icons instead of blurring into the plain search/file glyphs.
- **Answered question cards look clean.** When the assistant asks you to pick between options, the answered card now shows tidy question + answer chips instead of a raw text dump.
- **Delegated helpers read honestly.** A handed-off sub-task now shows a clear "working…" state with a short note that it runs in its own context, and its result lands in place when done — no more dead "running…" with nothing to click.

### Under the hood
- **Sub-agents get the same discipline.** When the assistant hands work to a helper agent, it now passes along the same rules it follows itself — search with the right tools, work in parallel, return tight results — so delegated work doesn't fall back to slow, noisy patterns.
- **Two latency trims.** The "thinking off" fast path now reuses one pooled network connection instead of opening a fresh one every request, and Rift checks the Claude CLI's capabilities once at startup instead of on your first message — shaving a possible multi-second stall off the very first reply.
- **Verified live.** Full test suite, type checks, and a real run of the app all pass clean; the new tool-row summary was confirmed rendering correctly in the running app.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

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

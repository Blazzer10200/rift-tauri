# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.63.0 — The app can tell you where it hurts

> When something's slow, broken, or just behaving oddly, Rift can now *show you where* instead of leaving you guessing. A new diagnostics console surfaces what every part of the app is doing in real time, with a colour-coded health read-out per subsystem — built so problems get found fast.

### What you'll notice
- **A live diagnostics console.** Settings → About → Tools & help → **"Open diagnostics console"** opens a real-time stream of what Rift is doing under the hood — the warm-CLI pool, tool calls, updates, speech, usage checks, and more. Filter by level or source, search the text, pause the stream, and copy it all out for a bug report. Paths are username-scrubbed, so it's safe to share.
- **At-a-glance health.** A row of pills across the top shows each subsystem as green / amber / red — e.g. *"Warm pool: 100% warm-hit"* or *"MCP tools: p50 12ms"* — so you can see in one glance whether everything's healthy or where to look. Click a pill to filter the stream to just that area.
- **Frontend errors no longer vanish.** A UI error that used to disappear into the console now shows up in the diagnostics stream, so a glitch leaves a trace you can actually find.

### Under the hood
- **The whole app is now instrumented.** Eight subsystems emit structured, queryable events with real timing — warm-pool hit/miss, per-tool durations, update stages, speech model-load + inference, usage state, corporate-cert loading. The plumbing for this already existed and emitted into the void; this release connects it end-to-end and gives it a UI.
- **A reusable metrics primitive.** New `metric!` / `timed!` building blocks make future instrumentation a one-liner instead of hand-rolled code — so the app keeps getting more self-aware over time at near-zero cost.
- **Zero latency cost.** A self-review caught one risky spot where the new instrumentation touched the hot reply path; it was moved out of the critical section before shipping. The warm-reply path stays as fast as it was (0–2ms to first token).
- **Verified.** Full backend suite (116 tests), frontend type-checks clean (4132 files), 13 new unit tests for the health + metrics math, and the console + health strip confirmed working live in the running app.

### Enhance-prompt wand — sharper, faster, more faithful
- **Better at messy input.** The ✨ wand's rewrite instructions were reworked around its real job — a *translation layer* for anyone who isn't a confident prompt-writer. It now explicitly recovers intent from typos, dictation artifacts, run-on or fragmented phrasing, and non-native grammar, fixes the mechanics silently, and never copies your errors into the result. Built with accessibility in mind.
- **Won't over-inflate your ask.** Added worked examples and a hard restraint rule so a one-line draft becomes at most a tight paragraph — never a phantom multi-point spec the request never implied. Faithfulness over embellishment.
- **Snappier.** The rewrite now runs at medium reasoning effort instead of the CLI default — it's a short, bounded task, so the long hidden high-effort pre-pass (part of the "why is the wand slow sometimes" feel) is gone.
- **Doc cleanup.** Fixed stale "Haiku" references throughout the enhance path — the wand has run on Sonnet for a while; the comments now say so.

## v0.62.0 — Honest about where the time goes

> This one settles the question of why a reply sometimes feels slow. Short version: it's almost always Claude thinking, not Rift — and now the app proves it instead of asking you to take our word. Plus a real reliability fix to the "is it stuck?" indicator.

### What you'll notice
- **No more false "stuck" warnings while Claude is thinking.** On the newest Claude models, a deep reasoning pass streams silently (no visible tokens) — and Rift's "still working?" watchdog mistook that silence for a hang, flashing "Waiting on the model · 45s" while the header still said "Thinking…". The two contradicted each other. Now the watchdog knows an active think isn't a stall, so the indicator stays honest.
- **AI Health now shows model-vs-Rift timing.** The Speed & efficiency card adds one plain-English line, computed from your own replies: *"about 93% is Claude thinking (17.0s); Rift's own overhead adds just 1.4s. The wait is the model, not the app."* It only appears when the model genuinely dominates, so it's always true for your data — the honest answer to "why did that take a while?"

### Under the hood
- **Rift now records the CLI's own server-side timing.** Every reply, the Claude CLI reports its true time-to-first-token and total API time; Rift was discarding both and measuring only its own wall-clock. Now it captures them, logs a per-turn attribution line (model time vs Rift's plumbing overhead), and feeds the AI Health pane — so latency questions are answered from data, not guesswork.
- **Measured, not assumed.** This release came out of a latency hunt: instrumented per-tool gaps (all model think-time), and a head-to-head against the bare CLI confirming Rift's warm pool turns a ~13s cold start into near-zero and adds nothing on top. The drag you feel is the model reasoning — the same on any Claude client.
- **Verified.** Full backend suite (112 tests) + 2 new tests for the timing math, type checks clean, and the new AI Health line confirmed rendering correctly in the running app.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

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

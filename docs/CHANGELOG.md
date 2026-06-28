# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## Unreleased — Just start chatting (work-locally mode)

> Built cont.224 (2026-06-28), awaiting `/git-ship`. No version bump yet.

### What you'll notice
- **No project folder? Just type.** Open Rift without picking a folder and you can still ask the assistant to read, write, edit, and run things — it works in a private scratch workspace (`%LOCALAPPDATA%\Rift\local`) instead of being stuck in a tool-less, chat-only mode. A small **"Local"** badge (and the status-bar) show the mode; click it any time to open a real project and switch over.
- **The empty-chat screen explains it.** With no folder open you get a "Working locally" card (instead of the old cold welcome) that says what's happening and keeps the "Open a project folder" action + your recent folders one click away.

### Under the hood
- Scratch path is **backend-resolved** (`local_scratch_dir()` in Rust, `create_dir_all`'d so it self-heals) — never renderer-supplied, so there's no path-injection surface. The renderer learns it via one read-only command (`assistant_local_scratch_path`) purely for the badge.
- The change is one branch in turn.rs root-resolution: a no-folder OAuth turn now resolves to the scratch dir, which makes the existing MCP-config + full-tools + workspace-`cwd` path apply automatically. **Gated to the standard OAuth path** — API-key, local-LLM, and sandboxed/prompting branches keep their existing `--tools ""` conversational behavior (ISSUES #47), unchanged.
- Pre-warming now warms the scratch-dir session too, so the first local turn is a warm hit, not a cold spawn.
- **Verified.** svelte-check clean (4134), 2 new Rust unit tests + 116 existing pass, cargo check clean, and the full flow confirmed live: a no-folder chat wrote `test.txt` into the scratch dir and reported back its absolute path (`Applied 1 file`, zero console errors); badge + status-bar + real-folder override all confirmed.

## v0.65.0 — One dial, one queue, no surprises

> A simplify pass on the three things you touch every turn: choosing a model, how much the assistant thinks, and what happens when you type while it's busy. Each was quietly over-built; each is now one clear control.

### What you'll notice
- **Thinking is one dial now.** The old on/off switch *plus* a separate effort slider became a single control: **Off · Low · Medium · High · Max.** "Off" replies instantly with no reasoning step; slide up for deeper thinking. The composer bar now shows your current setting at a glance (it reads "No thinking" when off), so you're never guessing whether thinking is on. It automatically caps at what each model supports.
- **Switching models mid-chat is honest.** A chat stays on the model it started with (switching it underneath would corrupt the running reasoning), so picking a different model used to silently do nothing. Now the picker says so plainly — it tags the running model "this chat", tells you a switch only applies to a new chat, and gives you a one-click **"New chat in <model>"** to actually make the jump.
- **One clear way to follow up while it's working.** Type while the assistant is busy and your message **queues** as a chip (drag to reorder, edit, or remove). Want to redirect the *current* reply instead? **"Send now"** on a chip, or **Alt+Enter** on your draft — both now spelled out right in the composer. The old confusing split between "queue", "steer", and a hidden per-chip mode toggle is gone.

### Under the hood
- **The thinking dial is a pure presentation layer** over the existing settings — no change to how turns are sent, so nothing about speed or behavior regressed. Onboarding and the AI Health page now speak the same Off→Max vocabulary.
- **Dead weight removed.** The retired toggle, the per-chip steer-mode plumbing, and a chunk of now-unused styling all came out.
- **Verified.** Type-checks clean (4134 files), 376 unit tests pass, and all three changes were confirmed live in the running app (real turns, model switches, and queue/steer interactions — zero console errors).

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.64.0** — Runs on your machine, not just mine: the window adapts to smaller/scaled screens (auto-tucking sidebar + a fourth-pane guard), first-run setup no longer dead-ends (full guided connect with PowerShell + npm install paths), errors talk like a human instead of dumping a Rust/Tauri chain, and the diagnostics console got a real redesign.

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

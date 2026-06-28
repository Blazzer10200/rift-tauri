# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.66.0 — Just start chatting (+ no more mystery-slow folders)

### What you'll notice
- **No project folder? Just type.** Open Rift without picking a folder and you can still ask the assistant to read, write, edit, and run things — it works in a private scratch workspace (`%LOCALAPPDATA%\Rift\local`) instead of being stuck in a tool-less, chat-only mode. A small **"Local"** badge (and the status-bar) show the mode; click it any time to open a real project and switch over.
- **The empty-chat screen explains it.** With no folder open you get a "Working locally" card (instead of the old cold welcome) that says what's happening and keeps the "Open a project folder" action + your recent folders one click away.
- **No more folders that are mysteriously slow.** If a project folder felt like every reply hung or crawled while another folder was instant, this fixes it. Older versions could leave extended **thinking** silently switched **on for one folder** — and on Opus that thinking step shows no text, so it just looked like the app was stuck. Thinking is off-by-default everywhere now; any folder still carrying that stale "on" is cleared on update, so it falls back to fast replies. You can still raise the thinking dial per-folder whenever you actually want deeper reasoning.

### Under the hood
- Scratch path is **backend-resolved** (`local_scratch_dir()` in Rust, `create_dir_all`'d so it self-heals) — never renderer-supplied, so there's no path-injection surface. The renderer learns it via one read-only command (`assistant_local_scratch_path`) purely for the badge.
- The no-folder change is one branch in turn.rs root-resolution: a no-folder OAuth turn now resolves to the scratch dir, which makes the existing MCP-config + full-tools + workspace-`cwd` path apply automatically. **Gated to the standard OAuth path** — API-key, local-LLM, and sandboxed/prompting branches keep their existing `--tools ""` conversational behavior (ISSUES #47), unchanged.
- Pre-warming now warms the scratch-dir session too, so the first local turn is a warm hit, not a cold spawn.
- The slow-folder fix is a **one-time, idempotent localStorage migration** (`migrateThinkingPins`, runs once per install before any pref is read): it clears stale per-folder `thinkingEnabled::<root>` pins left by the pre-v0.65.0 always-on toggle, while leaving the **intentional** per-folder model + effort pins untouched (those are visible in the picker/dial; the thinking pin was not). The off-by-default global baseline and the FE↔BE off-by-default lockstep were already correct — this only neutralizes the old data that predated them.
- **Verified.** svelte-check clean (4134 files), vitest 378/378 (2 new migration tests), cargo check clean, and the no-folder flow confirmed live: a no-folder chat wrote `test.txt` into the scratch dir and reported back its absolute path (`Applied 1 file`, zero console errors); badge + status-bar + real-folder override all confirmed.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.65.0** — One dial, one queue, no surprises: thinking collapsed to a single Off·Low·Medium·High·Max dial (with the current setting shown in the composer bar), honest mid-chat model switching (a chat stays on its starting model; the picker offers "New chat in <model>"), and a unified follow-up model (type-to-queue chips + Alt+Enter / "Send now" to steer the live turn).

- **v0.64.0** — Runs on your machine, not just mine: the window adapts to smaller/scaled screens (auto-tucking sidebar + a fourth-pane guard), first-run setup no longer dead-ends (full guided connect with PowerShell + npm install paths), errors talk like a human instead of dumping a Rust/Tauri chain, and the diagnostics console got a real redesign.

- **v0.63.0** — The app can tell you where it hurts: a live diagnostics console with per-subsystem green/amber/red health, full app instrumentation (8 subsystems, structured timed events), a reusable `metric!`/`timed!` primitive, and an enhance-prompt wand reworked as a faithful, faster translation layer.

- **v0.62.0** — Honest about where the time goes: proved slow replies are Claude thinking, not Rift (warm TTFT 0–2ms, ~93% of a turn is the API), fixed the false "stuck" watchdog during silent reasoning, and added model-vs-Rift timing to AI Health from the CLI's own server-side numbers.

- **v0.61.0** — Honest, detailed tool display: tool rows name what actually ran ("Searched 2 · read 1"), tidy answered-question chips, delegated-helper states read honestly; plus a pooled fast-path connection and boot-time CLI capability check.

- **v0.60.0** — Spring cleaning: a deep tidy-up of Rift's own code (dead-code sweep + splitting the biggest source files into focused pieces), no behavior change, verified pixel-identical top to bottom.

- **v0.51.0–v0.53.0** — Instant first reply (the first message of a new chat is warm too), the "everything is slow" regression fixed (a flipped permission default + thinking turned off by default), a redesigned model/effort picker, voice-mode overhaul (Ctrl+D dictate, real mic meter), and the sub-agent activity panel.

- **v0.20.7–v0.50.0** — Foundation → cross-machine era (full detail via `git log -- docs/CHANGELOG.md`): the full redesign port + stream design language, the warm-CLI process, honest API-stall watchdog, multi-window sync, the Workspace bento dashboard + AI Health diagnostic, the notification center, and the broad cross-machine compatibility pass that lets Rift run on other people's Windows PCs.

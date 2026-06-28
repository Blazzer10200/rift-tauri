# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.66.0 — Just start chatting (+ no more mystery-slow folders)

### What you'll notice
- **No project folder? Just type.** Open Rift without picking a folder and you can still ask the assistant to read, write, edit, and run things — it works in a private scratch workspace (`%LOCALAPPDATA%\Rift\local`) instead of being stuck in a tool-less, chat-only mode. A small **"Local"** badge (and the status-bar) show the mode; click it any time to open a real project and switch over.
- **The empty-chat screen explains it.** With no folder open you get a "Working locally" card (instead of the old cold welcome) that says what's happening and keeps the "Open a project folder" action + your recent folders one click away.
- **No more folders that are mysteriously slow.** If a project folder felt like every reply hung or crawled while another folder was instant, this fixes it. Older versions could leave extended **thinking** silently switched **on for one folder** — and on Opus that thinking step shows no text, so it just looked like the app was stuck. Thinking is off-by-default everywhere now; any folder still carrying that stale "on" is cleared on update, so it falls back to fast replies. You can still raise the thinking dial per-folder whenever you actually want deeper reasoning.

### Under the hood
- Scratch path is **backend-resolved** (`local_scratch_dir()`, `create_dir_all`'d so it self-heals) — never renderer-supplied, so no path-injection surface; the renderer learns it via one read-only command for the badge. The no-folder turn is one branch in turn.rs root-resolution (resolves to the scratch dir, reusing the existing full-tools + workspace-`cwd` path), **gated to the standard OAuth path** — API-key/local-LLM/sandboxed keep `--tools ""` (ISSUES #47). Pre-warming warms the scratch session too, so the first local turn is a warm hit.
- The slow-folder fix is a **one-time, idempotent localStorage migration** (`migrateThinkingPins`, runs before any pref is read): it clears stale per-folder `thinkingEnabled::<root>` pins from the pre-v0.65.0 always-on toggle, leaving the **intentional** per-folder model + effort pins untouched. The off-by-default baseline + FE↔BE lockstep were already correct — this only neutralizes the old data that predated them.
- **Verified.** svelte-check clean (4134 files), vitest 378/378 (2 new migration tests), and the no-folder flow confirmed live (wrote `test.txt` to scratch + reported its absolute path, zero console errors).

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.65.0** — One dial, one queue, no surprises: thinking collapsed to a single Off·Low·Medium·High·Max dial, honest mid-chat model switching, and a unified type-to-queue / Alt+Enter-to-steer follow-up model.
- **v0.60.0–v0.64.0** — Cross-machine + diagnostics era: smaller/scaled-screen layout, guided first-run setup, human-readable errors, a live per-subsystem diagnostics console + `metric!`/`timed!` instrumentation, honest tool display, and the proof that slow replies are the model not Rift (warm TTFT 0–2ms).
- **v0.20.7–v0.53.0** — Foundation era: the full redesign port + stream design language, the warm-CLI process, multi-window sync, the Workspace dashboard + AI Health, voice mode, and the notification center.

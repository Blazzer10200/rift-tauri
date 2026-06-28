# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.67.0 — Replies stay fast all session long

### What you'll notice
- **No more "fast first reply, then it crawls."** Within a single chat, every turn now reuses the same warm assistant process — so the snappy response you get on turn one holds for turn two, turn ten, and after you step away and come back. Previously the app could quietly tear down that warm process between turns and pay the full 5–12 second cold-start over and over, which is what made the assistant *feel* slow even when the model itself was fast.
- This is the fix behind the long-standing "Rift feels slow every turn" complaint. The earlier "slow = the model, not Rift" finding was only half the story: warm turns really were instant (0–2ms), but the app was only landing a warm turn about a third of the time. It now stays warm for the whole session.

### Under the hood
- Root cause was the warm pool **evicting the live CLI child mid-session**, re-paying cold-spawn on the majority of turns (measured 63% cold across 175 real turns in `turns.ndjson`). The persistent `stream-json` child is already a long-lived per-session process — its only hard eviction reason (releasing the Velopack `current/` lock on update) is already covered by the shutdown drain, so idle eviction was only ever a memory backstop, never a correctness need.
- Reframed eviction to **abandoned-session scale** instead of per-turn (`warm_pool.rs`): `IDLE_EVICT` 2h, `MAX_WARM` 20, pressure trim 30m, evict tick 5m. The child now survives any normal active-use pause. Removed the accumulated patch-on-patch re-warm scramble (stream-end/focus re-warm + `prevStreaming` in `Composer.svelte`); kept pre-warming for a childless tab (fresh `--session-id` or restart-history `--resume`) and the 150ms fast-fire.
- A model/effort/thinking dial change mid-chat still correctly drains and cold-respawns — different CLI argv is a genuinely different process — so that one case is expected, not a regression.
- **Verified.** `cargo test assistant::warm_pool` 7/7 (eviction tests rescaled to the new windows), svelte-check clean (4134 files), and runtime-confirmed on a live build: a second same-dial turn lands warm (`warm-start, cold-boot skipped, TTFT 1 ms`).

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.66.0** — Just start chatting: open Rift with no folder and the assistant still reads/writes/edits/runs in a private scratch workspace (with a "Local" badge), plus a one-time migration that clears stale per-folder "thinking on" pins so no folder is mysteriously slow.

- **v0.65.0** — One dial, one queue, no surprises: thinking collapsed to a single Off·Low·Medium·High·Max dial, honest mid-chat model switching, and a unified type-to-queue / Alt+Enter-to-steer follow-up model.
- **v0.60.0–v0.64.0** — Cross-machine + diagnostics era: smaller/scaled-screen layout, guided first-run setup, human-readable errors, a live per-subsystem diagnostics console + `metric!`/`timed!` instrumentation, honest tool display, and the proof that slow replies are the model not Rift (warm TTFT 0–2ms).
- **v0.20.7–v0.53.0** — Foundation era: the full redesign port + stream design language, the warm-CLI process, multi-window sync, the Workspace dashboard + AI Health, voice mode, and the notification center.

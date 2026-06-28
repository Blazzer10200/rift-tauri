# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.67.0 — Fast by default: thinking is now its own switch

### What you'll notice
- **Replies are fast again — and stay fast.** The big one: **extended thinking is now a clear ON/OFF toggle, and it's OFF by default.** Before, the speed control was a single dial that *secretly turned thinking on* the moment you raised it — and on Opus a thinking step shows no text, so the app just sat there silent for 6, 20, even 40 seconds. It looked frozen. Now thinking only happens when you deliberately flip it on, so an everyday reply lands in a second or two, the same as Claude Code in your editor.
- **Effort and thinking are two separate controls now.** Pick how hard the model works (Low → Max) *and* whether it does a slow reasoning pass first — independently. "High effort, no waiting" is finally a thing you can choose. When thinking is on, a live **"Thinking… 4s"** timer shows it's working so it never reads as a hang.
- **Within a chat, replies stay fast turn after turn.** Every turn reuses the same warm assistant process, so the snappy first reply holds for turn two, turn ten, and after you step away and come back — no more "fast once, then it crawls."

### Under the hood
- **Thinking ↔ effort were fused into one slider** (the v0.65.0 "one dial"): any rung above the lowest forced `thinkingEnabled=true`, making "high effort, thinking off" impossible. Live telemetry was decisive — thinking-off turns reach first text in 1–5s; thinking-on turns took 6 → 11 → 20 → 40s (and up to 500s of API time), every one tagged `dominant_cause=thinking`. Split back into a standalone toggle (`assistant.toggleThinking`) + an effort slider that writes only `thinkingEffort` (`SettingsMenu.svelte`, `Composer.svelte`, `modelMatrix.ts`). Default is off on both sides (`loadThinkingEnabled→false` ↔ `turn.rs thinking_on=unwrap_or(false)`); thinking-off sends `--effort low` (the CLI floor that kills the multi-second pre-pass). Haiku, which has no extended thinking, honestly shows no toggle. When thinking is off the effort slider dims with an "applies when thinking is on" note, since the backend sends `--effort low` regardless.
- **The warm pool was evicting the live CLI child mid-session**, re-paying cold-spawn on most turns (measured 63% cold across 175 real turns). Reframed eviction to abandoned-session scale (`warm_pool.rs`: `IDLE_EVICT` 2h, `MAX_WARM` 20, pressure 30m, evict tick 5m) so the persistent `stream-json` child survives normal pauses; removed the patch-on-patch re-warm scramble in `Composer.svelte`, kept pre-warming + 150ms fast-fire. A model/effort/thinking change mid-chat still correctly drains + cold-respawns (different CLI argv = a genuinely different process).
- **Verified.** svelte-check clean (4134 files), vitest 378/378, `cargo test assistant::warm_pool` 7/7, and runtime-confirmed: thinking-off everyday turns land in ~1–2s with a warm hit (`TTFT 1 ms`).

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.66.0** — Just start chatting: open Rift with no folder and the assistant still reads/writes/edits/runs in a private scratch workspace (with a "Local" badge), plus a one-time migration that clears stale per-folder "thinking on" pins so no folder is mysteriously slow.

- **v0.60.0–v0.65.0** — Cross-machine + diagnostics era: the unified queue/steer model, honest mid-chat model switching, scaled-screen layout, guided first-run setup, human-readable errors, and a live per-subsystem diagnostics console.
- **v0.20.7–v0.53.0** — Foundation era: the full redesign port + stream design language, the warm-CLI process, multi-window sync, the Workspace dashboard + AI Health, voice mode, and the notification center.

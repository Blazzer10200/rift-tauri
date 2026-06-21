# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## Unreleased — polish batch (staged on `main`, not yet versioned)

> Overnight stress-test + cleanup pass on top of shipped v0.20.7. Commit-only — not published. Verified: svelte-check 0/0 · vitest 210/210 · dev cargo rebuild clean · 0 prod npm vulns.

**Backend (turn hot-path efficiency):**
- Eliminated a redundant second `load_config()` disk read every turn (`config.rs::current_api_key_with` reuses the already-loaded config; `turn.rs:460`).
- Process-exit poll tightened 150ms → 50ms (`turn.rs:1366`) — faster turn-teardown.
- Stream char-pacer window 0.25s → 0.1s with a 360 c/s floor (`streaming.ts:288`) — snappier token render without dropping frames.

**Cleanup (dead code):**
- Removed `commands/git.rs` (3 registered Tauri commands with zero JS callers) + its mod/registration refs; removed `stt_set_engine` (superseded by `stt_set_config`); fixed 4 stale comments referencing the stripped remote half.

**Security:** `dompurify` 3.4.10 → 3.4.11 (GHSA-cmwh-pvxp-8882, ALLOWED_ATTR pollution — hits Rift's sanitizer `addHook` path). Prod vulns 1 → 0; remaining 4 are dev-only build deps.

**Frontend:** `ToolChip` ask-error color → real `--danger` token (was an undefined `--color-error` + hardcoded fallback).

## v0.20.7 — 2026-06-20 — Full redesign port + backend cleanup

> Whole UI re-ported to the design spec (`docs/design/rift-redesign.html`) at a strict fidelity bar — all 7 surfaces rebuilt + CDP-verified — plus a backend dead-code sweep.

**Changed (redesign — 7 surfaces):** Chat timeline → `StreamTurn`/`.sturn` (msg-acts row · jump-latest pill · turn-rail · FLIP), grouped tools → `.work` card (≥3). Home (warm/cold), Settings (bento → single-column RailShell, 5 tabs), Onboarding (centered card, 1180px), command palette. Composer → flat-bar + spec `.pop` popovers. Shell → global status bar; floating **Environment** pill replaces the old dock; SessionDiff dropped. Official Rift logo app-wide (neon-R squircle, transparent corners); screen-tint filter removed (a11y `warmTint` kept).

**Added:** Activity-dock toggle (Settings → Chat, default on). CLI backwards-compat hardening — version-gates every bleeding-edge spawn flag to the *installed* `claude` (`cli_caps.rs` table + `active_cli_version()`); 8 flags gated with fallbacks; below the 2.1.161 floor → update prompt; unreadable → conservative + Settings `⚠` banner.

**Fixed:** stream render defaulted to the legacy `MessageBubble` (streamMode defaulted OFF) → now ON; `stt.svelte.ts::setConfig` rolls back on backend reject; `deleteAllConversations` re-syncs in `finally`.

**Internal:** dead-code sweep (`rift_dir()`/`atomic_write_json()` + 2 unread `AudioCapture` fields, tightened `pub`). cargo test 64/64 · check 0/0 · vitest 210/210.

## Older versions

Full detail for any version: `git log -- docs/CHANGELOG.md`.

**v0.20.x:** .6 update/CLI-detection robustness (auto-retry + `CLAUDE_EXE` cache sync) · .5 CLI update-detection hardening · .4 DesignSync surfacing + method-aware chips · .3 effort-tier clamping + model-fallback correctness · .2 composer caption/slot polish · .1 thinking-off 400 fix (`nothink.rs` strips `clear_thinking_*`) · .0 cloud thinking on/off toggle. **v0.17–v0.19:** local-LLM no-think shim baked into backend · Local LLM page (context selector, model card, tok/s) · local-mode `num_ctx` truncation fix + Optimize-for-Rift. **v0.9–v0.16.x:** multi-window · Environment float · CLI-event transparency · shared `PageHero` + Home stats · Fable kill-switch · R2 update feed · minimal-core strip (−7,407 lines → 3 workspaces) · self-update brick fix. **v0.5–v0.8.x:** composer slim · dictation/PTT · loopback bridge · Fable 5 · backend split · tag-driven CI.

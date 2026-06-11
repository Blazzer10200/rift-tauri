# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.20 — 2026-06-11 — live plan limits: cockpit card + /usage popover

> **Why.** Rift priced local spend but was blind to the thing that actually gates a subscription user: the 5-hour and weekly rate-limit windows. Claude Code shows them under `/usage`; now Rift does too — in the cost cockpit and as a chat slash command.

- **Backend `usage_rate_limits`** (`usage/limits.rs`, new): reads the CLI's OAuth token from `~/.claude/.credentials.json` (READ-ONLY — never refreshes it; refresh tokens are one-time-use and an external refresh breaks the CLI's own auth loop) and polls the undocumented `api.anthropic.com/api/oauth/usage` endpoint with the required `anthropic-beta: oauth-2025-04-20` + `claude-code/<ver>` User-Agent headers (wrong UA = aggressively throttled bucket). Tolerant serde (unknown buckets ignored, every window optional), 60s in-process cache, friendly errors for no-login / expired token / 401 / 429.
- **Cost cockpit "Plan limits" card** (`CostPage.svelte`): one zone-colored bar per active window — 5-hour, weekly all-models, weekly per-model when present — with utilization % and reset countdowns. Fetch kept out of the cockpit's `Promise.all` so an OAuth hiccup can't blank local data. Header refresh refetches both.
- **`/usage` slash command** (`UsagePanel.svelte`, new): pops a compact panel above the composer with the same live bars. SlashMenu entry (Gauge icon, Info group), `/help` updated. Closes on Esc, ✕, or click-outside.
- Endpoint is undocumented: on any change it degrades to an "Unavailable — …" line; nothing else breaks.

**How to verify.** Harness → Cost → "Plan limits" card shows live percentages matching Claude Code's `/usage`. In chat, type `/usage` → panel pops with the same numbers; Esc / outside-click closes.

**Verify.** svelte-check 0/0 (4093 files) · Rust compiled clean via tauri-dev watcher (command answers over IPC) · live endpoint probe confirmed response shape · CDP behavior pass (card pixels, slash entry, panel open/close/click-outside).

## Older versions

v0.8.19 custom context menus app-wide + Fable 1M ctx fix + model menu reorg + new-user hardening batch (5s version-probe bound, onboarding permissions picker, third-party key-send warning, fail-loud control_response) · v0.8.18 UI sweep — 9 audit findings + per-chat model scoping (`TabState.modelOverride`/`effectiveModel`), slash-menu palette grammar, Home/Welcome snippets · v0.8.17 Rail-v2 steer chips (↳ per-chip queue/steer toggle, next-turn inject) + `turn.rs` overlapping-turn registry race fix (identity-guarded clears) · v0.8.16 backend split COMPLETE (`assistant/mod.rs` 4331→303, R1-R8; IPC surface identical) · v0.8.15 hot-file splits (TS complete + mod.rs 5/8) + honest Settings update chip · v0.8.14 fix: update dialog crashed on render (duplicate `{#each}` key — the real end of the "can't click update" saga) + swarm worktree-escape guard · v0.8.13 Claude Fable 5 limited-run model (self-heals to Sonnet/Opus after Jun 22) · v0.8.12 pill `×` = 24h snooze, never permanent · v0.8.11 Settings redesign + Harness one-viewport overhaul · v0.8.10 stable singleton `UpdatePill` · v0.8.9 first tag-driven CI release · v0.8.5 corrupted install no longer "up to date" · v0.8.3 updater can't hang forever · v0.8.0 one-click 401 recovery + edit-swarm + compression · v0.7.0 cost cockpit · v0.6.2 update child-lock fix · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.

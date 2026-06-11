# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.19 — 2026-06-11 — custom context menus + new-user readiness batch

> **Why.** Two arcs. First, native-feel right-click everywhere (the stock WebView2 menu is gone). Second, the first "other users" hardening pass: a three-agent audit of what breaks or confuses anyone who isn't the developer, then fixes across all three severity tiers.

- **Custom context menus app-wide** (`contextMenu.svelte.ts` + `ContextMenuHost.svelte`): edit fields Cut/Copy/Paste/Select-all w/ live disabled states · selections Copy · code blocks Copy code · links Open/Copy address · chat bubbles Copy message/selection. Components own a menu via `preventDefault()`. Paste backed by `tauri-plugin-clipboard-manager`. Shift+right-click = native (dev Inspect).
- **Fable 1M ctx fix:** `ctxWindowFor` had no `fable-5` pattern → header meter used a 200K denominator on a 1M model and auto-compact would fire ~5× early. + regression test.
- **Model menu reorg:** two-line rows (name + badge / blurb + ctx column), full taglines on hover.
- **New-user hardening:**
  - `claude --version` probe bounded at 5s — a hung CLI binary no longer wedges the app at splash forever.
  - Onboarding Step 4 gains a **Permissions picker** + plain hint (default = Bypass, runs without asking — now disclosed, was silent).
  - Custom provider w/o a saved key now **warns that the Anthropic key is sent to the third-party endpoint** (was silent fallback).
  - No-folder sends say "can't read files this turn" up front · Fable sunset warned a week ahead instead of a silent Opus swap · API-key bare mode (`~/.claude` config/MCP/CLAUDE.md not loaded) explained at both key-entry points · sign-in hint notes the console window may open behind Rift.
  - Fail-loud: control_response serialization error now errors the turn (was empty-line CLI hang) · update-apply taskkill sweep logs failures and derives the image name from `current_exe` · bundle-ID lockstep test pins the diagnostics log path to `tauri.conf.json`.
  - Residue: About links the public releases repo · STT vocab placeholder generalized · dev username out of test fixtures.

**How to verify.** Right-click an input / selection / code block / link → Rift menu, Paste lands. Onboarding Step 4 shows the Permissions row. Send with no folder open → composer notice.

**Verify.** svelte-check 0/0 (4092 files) · vitest 120/120 · cargo check + lockstep test clean · CDP pixel pass (onboarding Step 4 + menu surfaces).

## Older versions

v0.8.18 UI sweep — 9 audit findings + per-chat model scoping (`TabState.modelOverride`/`effectiveModel`), slash-menu palette grammar, Home/Welcome snippets · v0.8.17 Rail-v2 steer chips (↳ per-chip queue/steer toggle, next-turn inject) + `turn.rs` overlapping-turn registry race fix (identity-guarded clears) · v0.8.16 backend split COMPLETE (`assistant/mod.rs` 4331→303, R1-R8; IPC surface identical) · v0.8.15 hot-file splits (TS complete + mod.rs 5/8) + honest Settings update chip · v0.8.14 fix: update dialog crashed on render (duplicate `{#each}` key — the real end of the "can't click update" saga) + swarm worktree-escape guard · v0.8.13 Claude Fable 5 limited-run model (self-heals to Sonnet/Opus after Jun 22) · v0.8.12 pill `×` = 24h snooze, never permanent · v0.8.11 Settings redesign + Harness one-viewport overhaul · v0.8.10 stable singleton `UpdatePill` · v0.8.9 first tag-driven CI release · v0.8.5 corrupted install no longer "up to date" · v0.8.3 updater can't hang forever · v0.8.0 one-click 401 recovery + edit-swarm + compression · v0.7.0 cost cockpit · v0.6.2 update child-lock fix · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.

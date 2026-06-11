# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.18 — 2026-06-10 — UI sweep: 9 audit findings + per-chat model scoping

> **Why.** First batch of the app-wide UI consistency sweep (`ui-audit-2026-06-09.md`): the daily-visible rough edges — an information-dead activity rail, a slash menu that ignored the palette's design language, and the surprising "opening an old chat rewrites your default model" behavior.

- **Per-chat model scoping:** opening a saved conversation scopes its model to that chat (`TabState.modelOverride` + `effectiveModel`) — the new-chat default, Home quick-ask pill, and stored preference no longer flip, and the spurious "Model switched mid-conversation" toast is gone. Explicit picks still set both. `asModelSel` validation replaces a stale allow-list that silently dropped Fable chats' models.
- **Steps rail:** Bash rows strip leading `cd "<path>" &&` hops and middle-truncate keeping the tail (`shellLabel`, shared w/ live rows) — rows read `git status --short`, not `cd "C:/AI Workf…`.
- **Slash menu** rebuilt in the Ctrl+K palette grammar: compact left-anchored panel, boxed per-command icons, Conversation/Compose/Info groups, bold matched prefix, kbd hints + footer.
- **Empty-tab dock auto-collapses** (slides in on first message) · conversation scroll gets real bottom padding so the composer never buries the last message.
- **Home/Welcome resume rows:** one-line last-message snippet (`lastSnippet` on `ConversationMeta`) + model chip per row.
- **Polish:** Harness KPI rail one zero-state semantic (dim — until first turn) + `sonnet · high` chip space fix · user turns sit in a quiet inset card · insight cards carry consistent severity stripes.

**How to verify.** Open an old Opus chat → its composer pill says Opus while Home's quick-ask pill stays on your default, no toast. Type `/` → grouped, icon'd ~420px menu. Fresh chat → no empty Activity panel until the first message. Run any command → the rail row shows the real verb.

**Verify.** vitest 119/119 · `npm run check` 0/0 (4089 files) · `cargo check` clean · live CDP pass w/ a real turn, 0 console errors.

## Older versions

v0.8.17 Rail-v2 steer chips (↳ per-chip queue/steer toggle, next-turn inject) + `turn.rs` overlapping-turn registry race fix (identity-guarded clears) · v0.8.16 backend split COMPLETE (`assistant/mod.rs` 4331→303, R1-R8; IPC surface identical) · v0.8.15 hot-file splits (TS complete + mod.rs 5/8) + honest Settings update chip · v0.8.14 fix: update dialog crashed on render (duplicate `{#each}` key — the real end of the "can't click update" saga) + swarm worktree-escape guard · v0.8.13 Claude Fable 5 limited-run model (self-heals to Sonnet/Opus after Jun 22) · v0.8.12 pill `×` = 24h snooze, never permanent · v0.8.11 Settings redesign + Harness one-viewport overhaul · v0.8.10 stable singleton `UpdatePill` · v0.8.9 first tag-driven CI release · v0.8.5 corrupted install no longer "up to date" · v0.8.3 updater can't hang forever · v0.8.0 one-click 401 recovery + edit-swarm + compression · v0.7.0 cost cockpit · v0.6.2 update child-lock fix · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.

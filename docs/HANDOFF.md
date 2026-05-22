# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## WIP — 2026-05-22 — Lane 3: compaction UX + live tool dots

Frontend-only polish. No version bump — `/git-ship` rolls Lane 2 + Lane 3 together.

- **Tool dots fire live.** [MessageBubble.svelte:460](src/lib/components/assistant/MessageBubble.svelte#L460) — `nodeStatus` override (last-node-while-streaming → pending) now gated on `unit.status === "neutral"`. Prior behavior held the trailing tool of every turn at pulsing-yellow until end-of-turn since it stayed `lastBlockKey`. Tool + thinking blocks now honor their own per-block status; flip green/red as `fillToolResult` lands. Prose streaming still pulses.
- **Ctx-pill tooltip + nudge.** [ChatTabsBar.svelte](src/lib/components/shell/ChatTabsBar.svelte) — tooltip rewritten to lead with the cap and state plainly that cache-read tokens stay in-window (was: "replayed from prior turns", read as free). New `autoCompactDisabledNudge` chip mirrors `.compact-warn` style, surfaces when threshold=null && ctxPct≥70, goes red at 85%. CSS `[data-tone="red"]` added.
- **Settings.** [Settings.svelte](src/lib/components/settings/Settings.svelte) — threshold hint spells out cache-read counts; 80% marked `(recommended)`.

**Verify.** `npm run check` 0/0/0.

## WIP — 2026-05-22 — Lane 2: backend security HIGHs + Velopack preflight

Five Wave-3 HIGHs closed + Velopack ship-blocker fixed. Backend-only (parallel session on assistant.svelte.ts). Still v0.4.26-alpha; `/git-ship` next.

- **#221** [assistant/mod.rs](src-tauri/src/assistant/mod.rs) — `is_valid_model_name()` (`[A-Za-z0-9._-]+`, no leading dash) rejects model in `assistant_send`. Closes `--model -<flag>` injection.
- **#237** [assistant/mod.rs](src-tauri/src/assistant/mod.rs) — spawn-log uses normalized `effort_level`, not raw `effort`. Closes log-injection vector.
- **#227** [diagnostics/mod.rs](src-tauri/src/diagnostics/mod.rs) — `scrub_log_message` adds Ed25519 + DSA + generic + encrypted `BEGIN PRIVATE KEY`.
- **#238** [diagnostics/mod.rs](src-tauri/src/diagnostics/mod.rs) — `DiagBus::publish` scrubs `event.message` at the choke point. Covers 58 direct `emit*` sites bypassing LogForwarder. Idempotent. Completes #8 Rust-side.
- **#228** dialog plugin removed across [lib.rs](src-tauri/src/lib.rs) + [capabilities/default.json](src-tauri/capabilities/default.json) + [Cargo.toml](src-tauri/Cargo.toml) + [package.json](package.json). Zero refs confirmed.
- **Velopack quirk RESOLVED** — `×` (U+00D7, `1.15×`) passed to nuspec unescaped. Fix: [release.ps1](scripts/release.ps1) `Convert-ToAsciiSafe` + `[xml]` probe BEFORE `vpk pack`. Source ASCII-only w/ `\uXXXX` escapes (BOM-less PS5.1 safe). Smoke-tested: 5 non-ASCII → 0, XML probe PASSED.

**Verify.** `cargo check` 0 errors (1 pre-existing `private_interfaces` warning unrelated).

## v0.4.26-alpha — 2026-05-22 — timeline UI + center-on-work-area

Timeline UI on turn-rail; window centers in work area. Detail in CHANGELOG.

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. HEAD = **v0.4.26-alpha** shipped + published to [rift-releases](https://github.com/Blazzer10200/rift-releases/releases/tag/v0.4.26-alpha). Lane 2 + Lane 3 WIP above — uncommitted, awaiting `/git-ship` rollup. Tauri 2 + Svelte 5 + Rust + russh.

**Velopack U+00D7 fix** in `release.ps1::Convert-ToAsciiSafe` — don't remove. See Lane 2 WIP above.

**Open queue → [docs/ISSUES.md](ISSUES.md#active-work--current-sprint).** This file = session state + don't-touch invariants only.

---

## CRITICAL DON'T-TOUCH

- russh `ring` + reqwest `rustls`. russh `Config{keepalive 20s/3, window 2MiB, packet 32KiB}`.
- `~/.rift/*.json`: keep `serde(flatten) extra`. `VelopackApp::build().run()` FIRST in `lib.rs::run()`. `bundle.targets:["nsis"]`.
- DriftWatcher: never overwrite dirty local. `.rift-trail.jsonl` ignore mandatory.
- `GITHUB_OWNER`/`REPO` → public `rift-releases`, NOT source repo.
- `path_guard.rs` frozen; `rename_overwriting_via` ONLY for atomic upload tmp-swap.
- `FileAttributes::default()` SETSTAT = data-loss — use `empty()`. `DriftBucket::ToDelete`=LOCAL; `ToDeleteRemote`=REMOTE.
- Time displays MUST pass `[], { hour12: true }`. `spawn_frontend_pump` 200/s rate-limit.
- MCP self-exec: `RIFT_MCP_SERVER=1` branch in `lib.rs::run()` BEFORE Tauri loop.
- Chat tabs: `openTabs` filters vs `assistant_list_conversations`. `send()` keys `isFirstTurn` off `convoCreatedAt`. Keybinds gated on `workspace.activeId === "chat"`.
- `assistant_send`: `--permission-mode bypassPermissions` + full `BUILTINS` in `--allowed-tools`.
- TabState: per-tab field → add to TabState + getter on AssistantStore. Never back on the store.
- Image paste: `assistant_send` flips `--input-format text→stream-json` w/ attachments. 20MiB cap.
- Settings is workspace (kbd **8**), `Ctrl+,` flips; no slideover scrim.
- `UpdateService` managed Tauri state — `download_update` then `apply_pending_update`.
- `tauri.conf.json` `dragDropEnabled: false` — required for HTML5 DnD.
- AssistantPane drop handlers on `.pane` outer only — inner overlays break preventDefault chain.
- `compactionHistory[]` is camelCase in persisted JSON. Don't rename.
- `.shell` MUST be `position: fixed; inset: 0` (AppShell). `body.win-maximized .shell { inset: 8px }` for borderless-maximized.

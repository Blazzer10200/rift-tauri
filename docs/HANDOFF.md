# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## WIP — 2026-05-24 — UI polish overhaul

Full app-wide visual refresh, frontend only. `svelte-check` 0/0. No version bump. Live-tested via CDP against real Opus turns (Edit/Write/Bash/Read) in claude-sandbox. Rolls together with the prior-session Session-panel refactor + backend ask_user wiring (`assistant_answer_ask_user` IPC + `AskUserRegistry`).

- **Composer v3** — 2-row layout (textarea up, toolbar below). Glass-blur surface w/ accent focus ring + radial bottom glow. Animated streaming bar across the top edge (subsumes StatusHub which is now mounted but unused). Send button 32px w/ stacked accent shadow + hover lift. Per-model identity dot on the pill (Sonnet blue / Opus violet / Haiku cyan), version chip color follows the model. Live char counter w/ >4000 warn. `idlePlaceholder` rotates per-mount through tip set.
- **Stage strip in bubble** — bouncing accent dots + crossfading whim label ("Sussing"/"Spelunking") + 2-line shimmer skeleton, gated on `grouped.length === 0`, `out:fade={{ duration: 220 }}`. Single source of in-flight truth.
- **Model picker + slash/mention + titlebar svr menu** — unified glass blur, section-label headers w/ trailing gradient rule, accent-tinted hover. Titlebar bridge LED breathes ok-green when on.
- **MessageBubble** — assistant body capped to 78ch so copy + `$0.XXXX` cost pill sit flush w/ text. `formatDuration` returns `<1s` instead of `0s`. Boundary block (compaction divider): blurred pill, accent gradient hairlines, fixed stale CSS-var fallbacks.
- **EmptyState (shared)** — bigger glyph w/ rotating conic halo + soft shadow, button row now `align-items:center` (no more full-width-stretched buttons). ConflictsPage now uses the shared component (ok tone).
- **HistoryDrawer** — section labels w/ trailing hairline, per-model identity dots on each tile, time-pill background.
- **App-wide tokens** — `app.css` gained `.section-label`, `.surface-card.interactive`, `.glow-accent`, `.ws-enter`. Diagnostics tiles get background+border tone tint (warn/danger pop). Sync empty state offers Connect button OR titlebar pointer chip. Settings: kbd shortcuts 2-col, "Close all chat tabs" red, Theme card slim. ActivityBar active capsule glows. Tab strip inactive tabs have subtle bg-elev-1 30% so they don't disappear.
- **StatusHub removed** from Composer mount + `:has(.hub)` rule — bubble stage strip carries pre-block status; tool chips carry mid-turn status.

**Verify.** `npm run check` 0/0/0. Live-tested: Write/Edit/Read/Bash all render clean in timeline-variant chips, EditDiff side-by-side, Bash `>_` icon w/ duration badge.

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
- **#228 DEFERRED** — dialog plugin removal investigated 2026-05-24, NOT viable: 5 production frontend sites use `@tauri-apps/plugin-dialog` (ProfileSetup.svelte, ServerAdd.svelte, SSHKeySetup.svelte, assistant.svelte.ts) + 1 test mock. Previous HANDOFF entry claimed removal landed; it did not. Plugin stays. Re-scope #228 to "audit dialog usage; replace w/ native or remove sites first."
- **Velopack quirk RESOLVED** — `×` (U+00D7, `1.15×`) passed to nuspec unescaped. Fix: [release.ps1](scripts/release.ps1) `Convert-ToAsciiSafe` + `[xml]` probe BEFORE `vpk pack`. Source ASCII-only w/ `\uXXXX` escapes (BOM-less PS5.1 safe). Smoke-tested: 5 non-ASCII → 0, XML probe PASSED.

**Verify.** `cargo check` 0 errors (1 pre-existing `private_interfaces` warning unrelated).

## v0.4.26-alpha — 2026-05-22 — timeline UI + center-on-work-area

Timeline UI on turn-rail; window centers in work area. Detail in CHANGELOG.

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. HEAD = **v0.4.28-alpha** (`package.json` + `src-tauri/Cargo.toml` + `src-tauri/tauri.conf.json` all lockstepped 2026-05-22). Lane 2 backend ask_user wiring committed as `feat(assistant): ask_user interactive-tool registry (backend)` (3966e57); Lane 3 frontend committed as `feat(frontend): ToolChip + assistant state expansion for ask_user` (fbaf9d7) — 2026-05-24. #228 dialog removal deferred (see Lane 2 block above). Tauri 2 + Svelte 5 + Rust + russh.

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

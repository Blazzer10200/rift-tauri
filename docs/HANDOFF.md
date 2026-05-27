# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Branch in flight — `updater-migration` (Velopack → tauri-plugin-updater)

HEAD on main = v0.4.31-alpha; branch package version = v0.4.32-alpha. Backend + FE code complete + `/check` clean. Brief: [docs/design/updater-migration.md](design/updater-migration.md). Signing key `C:/Users/BLAZZER/.tauri/rift.key`. `release.ps1` (Tauri-only, v0.4.33+) + `release-bridge.ps1` (one-time v0.4.32 hybrid for v0.4.31 clients via Velopack feed). Stale-post-migration ISSUES closed: #16/#19/#25/#28/#252. Per-detail history via `git log -- docs/HANDOFF.md`.

## Aurora UI pass 2026-05-26 (this session — frontend-only)

Composer-driven visual refresh + reusable tooltip system + app-wide polish. `/check` clean (0/0). No backend / no version bump — `/git-ship` when ready.

**New code:** `src/lib/actions/tooltip.ts` (`use:tooltip={"..."}` Svelte action, glass surface, auto-flip, kbd-chip variant). `.tip/.tip-arrow/.tip-kbd` global CSS. `scripts/migrate-tooltips.mjs` + `fix-tooltip-migration.mjs` (idempotent).

**Migration:** 180 `title=` → `use:tooltip` across 34 files. Component title PROPS preserved. `LockBadge` prop `tooltip` → `tip` (avoid shadowing action import).

**Composer:** aurora conic-gradient tinted by `--model-color` (sonnet=blue, opus=purple, haiku=teal); send-btn ripple + upward chat-column sweep on fire; mic 3-bar waveform; placeholder fade-rotates 6s; drag-and-drop overlay; **effort pill renamed Instant/Smart/Deep w/ signal-bar 1→3**; model pill streaming breathe; menu items stagger; hint popover portaled to body (was clipped by composer's `overflow:hidden + backdrop-filter`).

**Bleed:** ChatTabsBar active tab + underglow tinted by `--model-color`; ActivityBar breathing capsule + halo; MessageBubble per-turn tint from `message.model`; scrollbars accent-tinted; `.dialog-shell`/`.slideover`/all toasts glass-blur + per-tone border.

**Convention:** `import { tooltip } from "$lib/actions/tooltip"` + `use:tooltip={"..."}` going forward.

---

## Stabilization 2026-05-26 (autonomous, R1-R4)

Per git log: audit.ps1 + SECURITY.md + Wave A/B tests (96→101 Rust) + ISSUES prune + #20 M0-M5b (`assistant.svelte.ts` 3356→2648L, new `state/assistant/persistence.ts`) + #2/#5 closed + 6 stale-blocks pruned (#8 #18 #32 #81 #153 #247). Last AUDIT GREEN (cargo check+test+audit+machete + svelte-check + npm audit).

**Open: 10 numbered issues** — #4 #7 #14 #15 #17 #20(M6-M9) #21 #29 #89 #265.

---

## Next-session prep (still gated on user, not session work)

**Resume here:**
1. **BACK UP `C:/Users/BLAZZER/.tauri/rift.key` OFF-MACHINE.** Vault / encrypted drive / 1Password. Lose this file = no v0.4.32+ install can ever update again. Hard gate.
2. `pwsh scripts/release-bridge.ps1` → ships v0.4.32-alpha hybrid (vpk + tauri-updater assets).
3. Update both machines to v0.4.32. Expect 5-10 min apply hang on v0.4.31→v0.4.32 (manual Setup.exe in release assets is the escape hatch). Confirm BOTH on v0.4.32 before proceeding.
4. v0.4.33+ regular flow — `bump.ps1` → CHANGELOG entry → `pwsh scripts/release.ps1`.
5. After v0.4.33 ships clean, retire `release-bridge.ps1`.

Baselines: cargo-audit + cargo-machete installed. Last full audit GREEN 2026-05-26. RUSTSEC-2023-0071 (rsa Marvin) is the only accepted CVE — see `docs/SECURITY.md` for rationale.

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. HEAD = **v0.4.31-alpha** shipped 2026-05-26. Migration branch above gates next ship. Tauri 2 + Svelte 5 + Rust + russh.

**Velopack U+00D7 fix** in pre-migration `release.ps1::Convert-ToAsciiSafe` — preserved in new `release.ps1` + `release-bridge.ps1` as `×`→`x` regex line (see CHANGELOG v0.4.27).

**Open queue → [docs/ISSUES.md](ISSUES.md#active-work--current-sprint).** This file = session state + don't-touch invariants only.

---

## CRITICAL DON'T-TOUCH

- `C:/Users/BLAZZER/.tauri/rift.key` — Tauri-updater signing key. Lose it and no v0.4.32+ install can update. Pubkey in `tauri.conf.json::plugins.updater.pubkey`; do NOT regenerate. Env: `TAURI_SIGNING_PRIVATE_KEY_PATH` via `.secrets/env.sh`.
- russh `ring` + reqwest `rustls`. russh `Config{keepalive 20s/3, window 2MiB, packet 32KiB}`.
- `~/.rift/*.json`: keep `serde(flatten) extra`. `bundle.targets:["nsis"]`. `createUpdaterArtifacts: true`.
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
- Settings is workspace (kbd **5** post-v0.4.30 rail trim), `Ctrl+,` flips; no slideover scrim.
- `tauri.conf.json` `dragDropEnabled: false` — required for HTML5 DnD.
- AssistantPane drop handlers on `.pane` outer only — inner overlays break preventDefault chain.
- `compactionHistory[]` is camelCase in persisted JSON. Don't rename.
- `.shell` MUST be `position: fixed; inset: 0` (AppShell). `body.win-maximized .shell { inset: 8px }` for borderless-maximized.

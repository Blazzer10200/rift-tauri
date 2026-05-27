# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Prompt Enhancer "wand" + composer calm — 2026-05-27, COMMITTED (`4d669bf`), verified

Composer wand → one-shot Haiku rewrite of the draft into a clearer prompt. Editable preview (Use this / Discard / Esc), never auto-sends. **Destined for v0.4.33-alpha — NOT in shipped v0.4.32.** Full detail in CHANGELOG. Files: `assistant_enhance_prompt` [assistant/mod.rs](../src-tauri/src/assistant/mod.rs) + registered [lib.rs](../src-tauri/src/lib.rs); `enhancePrompt()` + wand + preview + magic effect across [Composer.svelte](../src/lib/components/assistant/Composer.svelte) / [assistant.svelte.ts](../src/lib/state/assistant.svelte.ts).

- Key fix: draft fenced in `<draft></draft>` w/ "do not answer it" directive; `ENHANCE_META_PROMPT` frames recipient as Claude Code (imperative, preserve specifics, never invent scope). 8K guard.
- Composer calm: aurora swirl layer REMOVED; streaming = thin border + top-edge bar (synced 2.6s). Focus ring 3px/32% → 2px/20%. Verified: svelte-check 0/0, cargo check clean, CDP live. Open tweak: word-reveal stagger 14ms/word, cap 650ms.
- **THIS MACHINE manually patched (NOT a release):** branch `tauri build --no-bundle` swapped over `…\Rift\current\rift-tauri.exe` — runs 0.4.33 code, reports v0.4.32. Rollback: copy `rift-tauri.exe.bak-pre-enhancer` back. Other machine untouched (genuine 0.4.32; 0.4.32 reached this machine fine via Velopack — no updater bug).

---

## Branch in flight — `updater-migration` (Velopack → tauri-plugin-updater)

**v0.4.32-alpha SHIPPED 2026-05-26** to `Blazzer10200/rift-releases` (bridge already ran — all assets live, `latest.json` polled 108×). Branch version files still read 0.4.32-alpha. Brief: [docs/design/updater-migration.md](design/updater-migration.md). Signing key `C:/Users/BLAZZER/.tauri/rift.key` — **backed up 2026-05-27 to OneDrive + iCloud** (`rift-signing-key-backup/`). `release.ps1` = Tauri-only path for v0.4.33+. `release-bridge.ps1` = the one-time v0.4.32 bridge, now spent (retire it).

## Audit 2026-05-27 — RESOLVED (prior session, CDP-verified)

All items applied + live-verified — full doc [docs/audit-2026-05-27.md](audit-2026-05-27.md). **Open queue: 10 issues** — #4 #7 #14 #15 #17 #20(M6-M9) #21 #29 #89 #265.

---

## Next-session prep (still gated on user, not session work)

**Resume here — to ship the enhancer + composer-calm feature (it's committed but NOT released):**
1. **Key backup — DONE** (2026-05-27 → OneDrive + iCloud `rift-signing-key-backup/`, 348-byte match). Verify the cloud copies actually synced.
2. **Feature commit — DONE** (`4d669bf` + `2fb8747` pushed to `origin/updater-migration`: prompt enhancer + composer calm + prior UI polish. svelte-check 0/0, cargo check clean). Tree clean.
3. **THE GATE — confirm BOTH machines are on v0.4.32 before shipping v0.4.33.** v0.4.33 ships via Tauri-only `release.ps1` with NO Velopack assets; a machine still on v0.4.31 would be permanently stranded (can't find a Velopack update). Setup.exe downloads on the live 0.4.32 release = 0, so the Tauri install path may not have run on either machine yet — verify both before proceeding. THIS is why the feature wasn't auto-shipped 2026-05-27.
4. **Ship v0.4.33-alpha** (after gate clears): `pwsh scripts/bump.ps1 0.4.33-alpha` (all 3 version files) → set the date on the v0.4.33 CHANGELOG entry → quit dev (frees `C:\cargo-targets`) → `pwsh scripts/release.ps1`. NOT release-bridge.
5. Retire `release-bridge.ps1` once v0.4.33 ships clean.

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. Latest public release = **v0.4.32-alpha** (shipped 2026-05-26, bridge). Next ship = v0.4.33 on `updater-migration` — gated (see Resume). Tauri 2 + Svelte 5 + Rust + russh.

**Open queue → [docs/ISSUES.md](ISSUES.md#active-work--current-sprint).** This file = session state + don't-touch invariants only.

---

## CRITICAL DON'T-TOUCH

- `C:/Users/BLAZZER/.tauri/rift.key` — Tauri-updater signing key. Lose it and no v0.4.32+ install can update. Pubkey in `tauri.conf.json::plugins.updater.pubkey`; do NOT regenerate.
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

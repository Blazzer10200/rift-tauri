# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Prompt Enhancer "wand" + composer calm — 2026-05-27, COMMITTED, verified

Composer wand button → one-shot Haiku rewrite of the draft into a clearer, Claude-Code-optimized prompt. Editable preview (Use this / Discard / Esc), never auto-sends, never silently overwrites. On `updater-migration`, folded into the unshipped v0.4.32-alpha.

- **Backend:** `assistant_enhance_prompt(prompt)` in [assistant/mod.rs](../src-tauri/src/assistant/mod.rs) (above `SUMMARIZE_PROMPT_HEAD`). One-shot `claude -p` text on Haiku. Draft fenced in `<draft></draft>` w/ explicit "do not answer it" directive (fixed the model replying conversationally to message-shaped drafts). `ENHANCE_META_PROMPT` frames recipient as Claude Code: imperative lead, preserve specifics verbatim, never invent scope, structure-when-complex, non-coding drafts stay natural. Speed: meta on `--append-system-prompt` (cache), `--exclude-dynamic-system-prompt-sections`, `--strict-mcp-config`, `--disable-slash-commands`, `--tools ""`, temp cwd, `CLAUDE_DISABLE_HOOKS=1`. 8K-char guard. Registered in [lib.rs](../src-tauri/src/lib.rs).
- **Frontend:** `enhancePrompt()` on AssistantStore ([assistant.svelte.ts](../src/lib/state/assistant.svelte.ts)); wand + preview panel + "magic" effect in [Composer.svelte](../src/lib/components/assistant/Composer.svelte) — clip-text shimmer, sparkle twinkles, word-by-word blur-materialize reveal. `prefers-reduced-motion` throughout.
- **Composer streaming calm:** aurora swirl layer REMOVED entirely (spans + ~55L CSS). Streaming = thin model-tinted border + top-edge bar (synced 2.6s w/ pill breathe). Focus ring softened 3px/32% → 2px/20%. CDP pixel-verified.
- **Verified:** svelte-check 0/0 (4102 files); `cargo check` clean (8.37s); CDP live (enhance: short/long/code/over-length-error/double-click; composer streaming forced-class shot). **Open tweak:** word-reveal stagger 14ms/word, cap 650ms — easy dial.

---

## Branch in flight — `updater-migration` (Velopack → tauri-plugin-updater)

HEAD on main = v0.4.31-alpha; branch package version = v0.4.32-alpha. Backend + FE code complete + `/check` clean. Brief: [docs/design/updater-migration.md](design/updater-migration.md). Signing key `C:/Users/BLAZZER/.tauri/rift.key`. `release.ps1` (Tauri-only, v0.4.33+) + `release-bridge.ps1` (one-time v0.4.32 hybrid for v0.4.31 clients via Velopack feed).

## Audit 2026-05-27 — RESOLVED (prior session, CDP-verified)

All audit items applied + live-verified. Full doc: [docs/audit-2026-05-27.md](audit-2026-05-27.md). Bugs B1/B3/B6/F12 fixed; P2 effort picker dropdown; P3 user "You" bubble; P5/P6 Settings probe/dirty-flag. Deliberately skipped: image-paste, Sync/Apply, connect/disconnect, streaming, splash (need live mutations).

**Open: 10 numbered issues** — #4 #7 #14 #15 #17 #20(M6-M9) #21 #29 #89 #265.

---

## Next-session prep (still gated on user, not session work)

**Resume here:**
1. **BACK UP `C:/Users/BLAZZER/.tauri/rift.key` OFF-MACHINE.** Hard gate before any ship. CHANGELOG claims it was backed up pre-ship — CONFIRM that's true (off-machine, not just on this disk) before step 2. Lose it = no v0.4.32+ client can ever update again.
2. ~~Commit feature work~~ — DONE (commit `4d669bf`, pushed to `origin/updater-migration`: prompt enhancer + composer calm + prior UI polish, svelte-check 0/0, cargo check clean). Tree is clean.
3. **THE RELEASE — one command, gated on step 1:** `pwsh scripts/release-bridge.ps1` → builds + publishes the v0.4.32-alpha hybrid (Velopack assets for v0.4.31 clients + Tauri-updater assets for v0.4.32+). NOT run autonomously 2026-05-27: it's an irreversible public ship to `rift-releases`, gated on the key backup + it bundles a brand-new UI feature into the critical one-time bridge — wanted a human eyes-open go. Quit dev first (run-dev tree was stopped this session; verify nothing holds `C:\cargo-targets`).
4. Update both machines to v0.4.32. Confirm BOTH before proceeding.
5. v0.4.33+ regular flow — `bump.ps1` → CHANGELOG entry → `pwsh scripts/release.ps1`.
6. After v0.4.33 ships clean, retire `release-bridge.ps1`.

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. HEAD = **v0.4.31-alpha** shipped 2026-05-26. Migration branch above gates next ship. Tauri 2 + Svelte 5 + Rust + russh.

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

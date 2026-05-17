# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 91 — 2026-05-17 — v0.4.5-alpha: embedded-Claude allowlist + STT slur-tolerance

Both S91 priorities landed. CDP-verified.

**Allowlist (P1).** [src-tauri/src/assistant/mod.rs](src-tauri/src/assistant/mod.rs) `assistant_send` widened `--allowed-tools` in all three branches to the full CLI built-in surface via shared `BUILTINS` const. Added (over S88's `+Skill`): `Agent` (subagent spawn — `/plan`/`/quick-review`/`/check`), `AskUserQuestion`, `BashOutput`+`KillBash`+`KillShell` (auto-fired after `Bash run_in_background:true`), `ExitPlanMode`, `MultiEdit`, `NotebookEdit`, `SlashCommand`. MCP scope unchanged. Verified via CDP smoke: fresh chat tab → Bash + Agent-subagent spawn both completed cleanly, zero `permission|denied|not allowed` strings in body text.

**STT (P2).** [src/lib/state/stt.svelte.ts](src/lib/state/stt.svelte.ts) bumped `r.maxAlternatives` 1 → 3 + added `pickBestAlternate(res)` helper called from `onResult` — returns highest-confidence transcript, falls back to `alt[0]` when WebView2 returns 0 for every alternate (spec-allowed). Cleaner lower-ranked variants can now win. Vocabulary hints + Azure-direct fallback deferred (stretch).

3-file version 0.4.4 → 0.4.5-alpha; Cargo.lock auto-syncs. CHANGELOG v0.4.5 extended; v0.4.3/v0.4.4 archived. **Pending:** commit + push; optional `release.ps1` for binary → Trey auto-updates.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. Source at **v0.4.5-alpha** (S91 pending commit). Tauri 2 + Svelte 5 + Rust + russh. Velopack updater, NSIS perUser installer.

**v0.4.1 shell** = default; `useV03Shell` toggle = experimental v0.2 fallback (storage key verbatim, never rename).

**CDP autonomous-verify live** — `run-dev.bat` sets WebView2 port; `npm run cdp:serve` on 9223; `scripts/cdp/c.sh state|eval|type|click|wait|shot|key`.

**Voice:** Settings → Speech (STT only). Mic in composer; WebView `SpeechRecognition` → `assistant.composerDraft`. v0.4.5 picks highest-confidence of 3 alternates.

**v0.2 queue** (each needs `/grill` or `/plan`): auto-Mirror on rename; dry-run Mirror preview; EACCES auto-fix-perms; `lib.rs`→`commands/*.rs` split (1790L / 51 cmds — biggest); LocalPane/RemotePane base extract; Diagnostics canonical-skeleton; integration tests phase 1. STT stretch: vocabulary hints / Azure-direct fallback / "did you mean X?" UI.

**Audit queue:** 6 LOW lib/config, upstream-blocked. See [docs/AUDIT.md](docs/AUDIT.md).

**Multi-user:** Trey OFF Mirror until on-latest. Setup: [docs/TREY-SETUP.md](docs/TREY-SETUP.md). v0.4.5 auto-updates him.

**Don't reintroduce:** dock primitive, maximize-to-center, `PanelState.slot`, `dockSplitPct`, Tasks-as-peer, AddPanelMenu, TabRail under v0.4.1, OpRail/TopBar, whisper-rs (libclang Windows dep), `msedge-tts` / TTS module / speaker UI.

---

## CRITICAL DON'T-TOUCH

- russh `ring` + reqwest `rustls` only (NASM blocks aws-lc-rs). russh `Config{keepalive 20s/3, window 2 MiB, packet 32 KiB}` in `sftp::open_session`+`tunnel::start`.
- `~/.rift/*.json` compat — keep `serde(flatten) extra`. `VelopackApp::build().run()` FIRST in `lib.rs::run()`. `bundle.targets:["nsis"]`.
- DriftWatcher conflict-rename guard — never overwrite dirty local. `.rift-trail.jsonl` ignore rule mandatory.
- `GITHUB_OWNER`/`GITHUB_REPO` → public `rift-releases`, NOT source repo.
- `path_guard.rs` frozen; `rename_via` strict; `rename_overwriting_via` ONLY for atomic upload tmp-swap.
- `last_scan_entries` = `std::sync::Mutex` (NOT tokio). `force_pull_now`/`force_push_now` invariants preserved.
- `FileAttributes::default()` for SETSTAT = data-loss — use `empty()`. Upload pre-flight SHA-collapse before CONFLICT. `DriftBucket::ToDelete` deletes LOCAL; `ToDeleteRemote` deletes REMOTE (mirror+baseline gated).
- Time displays MUST pass `[], { hour12: true }`. `spawn_frontend_pump` 200/s rate-limit; critical stages bypass.
- **v0.2.56:** Assistant tab self-execs MCP via `RIFT_MCP_SERVER=1` env branch in `lib.rs::run()` BEFORE Tauri loop.
- **v0.4 chat tabs:** `openTabs` filters vs `assistant_list_conversations` on init. `send()` keys `isFirstTurn` off `convoCreatedAt` (NOT `currentConvoId`).
- **v0.4.1 right-pane:** keep `useV03Shell` storage key. Width 320-1200, default 560. Left-edge-resize only.
- **S87 context pill:** `recordTurnUsage(u, accumulate)` — only `result` envelope updates `sessionUsage`; both refresh `lastTurnUsage`. Effective ctx = `input + cache_read + cache_create`. `[1m]` suffix = 1M window.
- **S87 image paste:** `assistant_send` flips `--input-format text → stream-json` when attachments present. 20 MiB cap + `image/*` gate.
- **S91 allowlist:** `assistant_send` `--allowed-tools` MUST keep the full `BUILTINS` const (all CLI built-ins incl. Agent/BashOutput/KillBash/SlashCommand) in all three branches. Narrowing → per-tool denials in the Assistant tab.
- **S88 STT:** WebView's `SpeechRecognition` writes directly to `assistant.composerDraft`. `baseDraft` snapshot on start preserves pre-existing text in append mode. `errno("not-allowed")` → friendly mic-perm message. Settings section id=`"speech"` (was `"voice"`). TTS is fully removed — do not reintroduce.

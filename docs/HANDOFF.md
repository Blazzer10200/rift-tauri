# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 88 — 2026-05-17 — Hot-fix v0.4.2 + voice arc v0.4.3

**S88** opened with Trey hitting "skills blocked" inside the embedded Claude. Root cause: `assistant_send`'s `--allowed-tools` allowlist omitted `Skill` — three branches in `assistant/mod.rs:952` patched, doc comment corrected. Shipped as **v0.4.2-alpha** hot-fix.

Then voice arc. Two directions in one release. **v0.4.3-alpha** shipped end-of-session.

**TTS** — `msedge-tts` crate calls Edge's free read-aloud endpoint (Azure Neural voices, no API key). `tts::TtsService` runs one tokio task draining a sentence queue, emits MP3 b64 via `tts://audio`. Frontend `tts.svelte.ts` splits live stream text on `/[.!?]+["')\]]?\s+/`, plays back-to-back via HTMLAudioElement. AssistantHeader speaker toggle + per-message replay icon on MessageBubble + Settings → Voice picker / rate / pitch / volume / test.

**STT** — WebView2's `SpeechRecognition` (Edge/Chromium → Azure online). Live interim text streams directly into `assistant.composerDraft`. Composer gets a mic button on the left. `stt::stt_*_config` only persists settings; recognition is 100% browser-side. Pivoted away from `whisper-rs` mid-session — Windows `libclang.dll` requirement broke the build on this dev machine.

Archived `docs/design/v0.4.1-right-pane-refactor.md` + `v0.3-brainstorm.md` → `docs/archive/design/`.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. **v0.4.3-alpha** is the live release target. Tauri 2 + Svelte 5 + Rust + russh. Velopack updater, NSIS perUser installer.

**v0.4.1 shell** is the default daily-driver path; `useV03Shell` toggle = experimental v0.2 fallback (storage key kept verbatim, never rename).

**CDP autonomous-verify live** — `run-dev.bat` sets WebView2 port; `npm run cdp:serve` on 9223; drive via `scripts/cdp/c.sh state|eval|type|click|wait|shot|key`.

**Voice quick-start (S88):** Settings → Voice. Top half = TTS (master toggle, voice picker, sliders, test). Bottom half = STT (master toggle, language, live-partials/continuous/append-vs-replace toggles). Mic permission prompts once on first record.

**v0.2 queue (each needs `/grill` or `/plan` first):** auto-Mirror on rename; dry-run Mirror preview; EACCES auto-fix-perms; `lib.rs`→`commands/*.rs` split; LocalPane/RemotePane base-component extract; Diagnostics canonical-skeleton; integration tests phase 1. `lib.rs` is 1782L w/ 56 cmds — biggest impact-per-risk pick.

**Audit queue (post-S86):** 6 LOW lib/config, all upstream-blocked. Full list in [docs/AUDIT.md](docs/AUDIT.md).

**Multi-user:** Trey OFF Mirror until on-latest + fresh-Pulled baseline. Setup doc: [docs/TREY-SETUP.md](docs/TREY-SETUP.md). v0.4.2 fix lands on his next Velopack auto-update.

**Don't reintroduce:** dock primitive, maximize-to-center, `PanelState.slot`, `dockSplitPct`, Tasks-as-peer, AddPanelMenu, TabRail under v0.4.1, OpRail/TopBar, whisper-rs (libclang Windows dep).

**Ship pipeline:** `powershell -NoProfile -File ./scripts/release.ps1` — build → vpk pack → upload to `rift-releases`.

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
- **v0.2.56:** Assistant tab self-execs MCP via `RIFT_MCP_SERVER=1` env branch in `lib.rs::run()` BEFORE Tauri loop; CLI passes `--mcp-config` + `--allowed-tools mcp__rift__* + Skill` (S88 added Skill).
- **v0.4 chat tabs:** `openTabs` filters vs `assistant_list_conversations` on init. `send()` keys `isFirstTurn` off `convoCreatedAt` (NOT `currentConvoId`).
- **v0.4.1 right-pane:** keep `useV03Shell` storage key. Width 320-1200, default 560. Left-edge-resize only.
- **S87 context pill:** `recordTurnUsage(u, accumulate)` — only `result` envelope updates `sessionUsage`; both refresh `lastTurnUsage`. Effective ctx = `input + cache_read + cache_create`. `[1m]` suffix = 1M window.
- **S87 image paste:** `assistant_send` flips `--input-format text → stream-json` when attachments present. 20 MiB cap + `image/*` gate.
- **S88 Skill tool:** `--allowed-tools` allowlist MUST include `Skill` in all 3 branches of `assistant_send`. Comment at L192 corrected — skills need both `--disable-slash-commands` OFF and `Skill` in allowlist.
- **S88 TTS:** sentence-boundary chunking is per-message-id buffered; `tts.flush(messageId)` on `onDone` flushes trailing fragment. `stop()` cancels both assistant stream + tts queue (generation counter + local queue clear).
- **S88 STT:** WebView's `SpeechRecognition` writes directly to `assistant.composerDraft`. `baseDraft` snapshot on start preserves pre-existing text in append mode. `errno("not-allowed")` → friendly mic-perm message.

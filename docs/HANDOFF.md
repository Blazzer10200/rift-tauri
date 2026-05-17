# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 90 — 2026-05-17 — v0.4.4-alpha source ship + stress-test fix-ups

Autonomous CDP-driven stress test across every UI surface: ActivityBar (Ctrl+1..7 / Ctrl+0 / drag-reorder + persistence), chat tabs (Ctrl+T/W, Ctrl+Tab, Alt+1..9), right-pane × 7 (lazy-mount latch + width clamp 320..1200 + dblclick-snap), Settings × 7 sections (v0.3↔v0.2 shell round-trip, all STT/Assistant/Terminal toggles, language picker, diagnostic copy), Sync (drift scanner caught 1 pull in `[endure]`), Files (TwoPane nav + remote ctx menu), Terminal (PTY echo verified), Velopack ("Up to date" vs released 0.4.3). Status bar `isHandshaking` invariant held across reconnect.

Two latent UX bugs caught + fixed: (a) `right-pane.svelte.ts::init()` clamped state.width but didn't re-persist, so OOB localStorage values survived launches — now writes back. (b) Composer mic button rendered unconditionally; clicking it with STT disabled silently set `stt.lastError`. Gated on `stt.config.enabled && stt.supported` w/ `onMount(() => stt.init())` so the gate reflects backend config without a Settings visit. Tooling: `scripts/cdp/serve.cjs` `KEY_DEFS` gained Comma / Slash / Space / Period / Backquote / ArrowLeft / ArrowRight (drives `Ctrl+,`, `Ctrl+\`` directly).

Bumped 3-file version 0.4.3 → 0.4.4-alpha; Cargo.lock auto-synced. CHANGELOG v0.4.4 extended w/ S90 fix-ups. Source committed + pushed.

**Pending:** binary release via `powershell -NoProfile -File ./scripts/release.ps1` — vpk+nsis → `rift-releases`.

---

## Session 89 — 2026-05-17 — TTS rollback + workspace clean-out

TTS reversed (only STT wanted): removed `src-tauri/src/tts/`, `tts.svelte.ts`, speaker UI, `msedge-tts` chain. Settings `Voice` → `Speech` (id `"speech"`). 6 dead npm deps + stale `cdp/smoke-v04.sh` + orphan branches dropped. svelte-check **0/0**. Folded into v0.4.4-alpha (S90).

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. Source at **v0.4.4-alpha** (committed); binary release pending — run `powershell -NoProfile -File ./scripts/release.ps1` to publish vpk+nsis to `rift-releases`. Tauri 2 + Svelte 5 + Rust + russh. Velopack updater, NSIS perUser installer.

**v0.4.1 shell** is the default daily-driver path; `useV03Shell` toggle = experimental v0.2 fallback (storage key kept verbatim, never rename).

**CDP autonomous-verify live** — `run-dev.bat` sets WebView2 port; `npm run cdp:serve` on 9223; drive via `scripts/cdp/c.sh state|eval|type|click|wait|shot|key`.

**Voice:** Settings → Speech (STT only — TTS removed S89). Mic button in composer; WebView `SpeechRecognition` writes to `assistant.composerDraft`. Mic permission prompts once on first record.

**v0.2 queue** (each needs `/grill` or `/plan`): auto-Mirror on rename; dry-run Mirror preview; EACCES auto-fix-perms; `lib.rs`→`commands/*.rs` split (1790L / 51 cmds — biggest pick); LocalPane/RemotePane base-component extract; Diagnostics canonical-skeleton; integration tests phase 1.

**Audit queue:** 6 LOW lib/config, upstream-blocked. See [docs/AUDIT.md](docs/AUDIT.md).

**Multi-user:** Trey OFF Mirror until on-latest. Setup: [docs/TREY-SETUP.md](docs/TREY-SETUP.md). v0.4.4 auto-updates him.

**Don't reintroduce:** dock primitive, maximize-to-center, `PanelState.slot`, `dockSplitPct`, Tasks-as-peer, AddPanelMenu, TabRail under v0.4.1, OpRail/TopBar, whisper-rs (libclang Windows dep), `msedge-tts` / TTS module / speaker UI.

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
- **S88 STT:** WebView's `SpeechRecognition` writes directly to `assistant.composerDraft`. `baseDraft` snapshot on start preserves pre-existing text in append mode. `errno("not-allowed")` → friendly mic-perm message. Settings section id=`"speech"` (was `"voice"`). TTS is fully removed — do not reintroduce.

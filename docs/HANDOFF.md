# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 93 — 2026-05-17 — v0.4.7-alpha: Settings → Accessibility

Phase 1 of the dyslexia-friendly arc. New section between Assistant + Speech (lucide `Accessibility` icon) — one master toggle + three dials:

- **Master switch** — first-time-on seeds Lexend + line-height boost; forwards `dyslexiaMode: true` through `assistant_send` so [assistant/mod.rs](src-tauri/src/assistant/mod.rs) appends a per-turn addendum telling Claude to interpret phonetic/letter-swap typos + STT slur artifacts charitably. Zero per-turn cost.
- **UI font** — System (Inter) ↔ Lexend (`@fontsource-variable/lexend`, ~150KB offline).
- **Line + letter spacing** — 1.85 line-height + letter/word spacing bumps, scoped to `.bubble`/`.markdown-body`/textarea.
- **Warm reading tint** — sepia overlay on bubbles + code only; rest of UI keeps dark theme.

Store [accessibility.svelte.ts](src/lib/state/accessibility.svelte.ts) mirrors `ui-prefs.svelte.ts` (localStorage + `documentElement.dataset.a11y*`). CSS at bottom of `app.css`. Settings UI matches Speech section. Wired via `+layout.svelte`. 3-file bump 0.4.6 → 0.4.7-alpha. Auto-verifier clean. Phase 2 deferred.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. Source at **v0.4.7-alpha** (S93 pending commit; v0.4.6 binary live in `rift-releases`). Tauri 2 + Svelte 5 + Rust + russh. Velopack updater, NSIS perUser installer.

**v0.4.1 shell** = default; `useV03Shell` toggle = experimental v0.2 fallback (storage key verbatim, never rename).

**CDP autonomous-verify live** — `run-dev.bat` sets WebView2 port; `npm run cdp:serve` on 9223; `scripts/cdp/c.sh state|eval|type|click|wait|shot|key`.

**Voice:** Settings → Speech (STT only); v0.4.5 picks highest-conf of 3 alternates. **A11y:** Settings → Accessibility (dyslexia-friendly mode, font, spacing, warm tint — v0.4.7).

**v0.2 queue** (needs `/grill` or `/plan`): auto-Mirror on rename; dry-run Mirror preview; EACCES auto-fix-perms; `lib.rs`→`commands/*.rs` split (1790L); LocalPane/RemotePane base extract; integration tests phase 1. A11y stretch: SymSpell+Metaphone "did you mean" pill; STT vocab hints / Azure-direct.

**Audit queue:** 6 LOW lib/config, upstream-blocked. See [docs/AUDIT.md](docs/AUDIT.md).

**Multi-user:** Trey OFF Mirror until on-latest. [docs/TREY-SETUP.md](docs/TREY-SETUP.md). v0.4.7 auto-updates him.

**Don't reintroduce:** dock primitive, maximize-to-center, `PanelState.slot`, `dockSplitPct`, Tasks-as-peer, AddPanelMenu, TabRail under v0.4.1, OpRail/TopBar, whisper-rs (libclang dep), `msedge-tts` / TTS / speaker UI.

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
- **S91 allowlist + S92 mode:** `assistant_send` MUST keep `--permission-mode bypassPermissions` (NOT `dontAsk` — that auto-denies MCP calls) AND the full `BUILTINS` const in `--allowed-tools` (Agent/BashOutput/KillBash/SlashCommand etc) across all three branches. Both gates required; either change → per-tool denials in the Assistant.
- **S88 STT:** WebView's `SpeechRecognition` writes directly to `assistant.composerDraft`. `baseDraft` snapshot on start preserves pre-existing text in append mode. `errno("not-allowed")` → friendly mic-perm message. Settings section id=`"speech"` (was `"voice"`). TTS is fully removed — do not reintroduce.

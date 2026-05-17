# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `docs/archive/CHANGELOG-archive.md` (and `git log -- docs/CHANGELOG.md`).

## v0.4.4-alpha — 2026-05-17 — Revert TTS + workspace clean-out

User reversed course on text-to-speech same day v0.4.3 shipped: only STT was wanted. This release rips TTS out and folds in a wider hygiene pass.

**TTS removed end-to-end.** `src-tauri/src/tts/` deleted, `msedge-tts` crate dropped from `Cargo.toml` (Cargo.lock shed 30+ transitive crates incl. tungstenite/tokio-tungstenite, -447 lines). Frontend `src/lib/state/tts.svelte.ts` deleted; speaker toggle + per-message replay button + all 5 TTS cards in Settings + orphan voice-dropdown / slider / sub-head CSS purged. Settings section renamed `"Voice"` → `"Speech-to-text"` (id `"speech"`, icon `Mic`). `stt.svelte.ts:121` stale error string updated. `assistant.svelte.ts` lost `tts.init/feed/flush/cancel` integration points. STT is now the sole voice surface, unchanged.

**Workspace clean-out.** Dropped 6 dead npm deps (`bits-ui`, `clsx`, `tailwind-merge`, `tailwind-variants`, `tw-animate-css`, `@types/dompurify`) — leftover from the shadcn yank — pruning 18 packages from `package-lock.json` and cutting npm-audit findings 13 → 10. Deleted stale `scripts/cdp/smoke-v04.sh` (exercises retired dock primitive; `smoke-v04-1.sh` covers current shell). Cleared 24 CDP debug screenshots from `scripts/cdp/.tmp/` and the empty `.claude/worktrees/` directory. Removed pre-existing orphan `.reasoning-meta.subtle` CSS class from `MessageBubble.svelte` and trimmed 2 unused lucide-svelte icon imports (`ExternalLink` in `ConflictResolver.svelte`, `Upload` in `Bootstrap.svelte`). Pruned local branch `backup-s25` (4 unique commits from S22-S25 era) and remote orphan `claude/determined-driscoll-32834e`.

**Doc hygiene.** Project `CLAUDE.md` hot-files table refreshed to current line counts (10 backend + 5 frontend files drifted; `assistant/mod.rs` went 775 → 1167L, `Settings.svelte` 1060 → 1505L). Memory `project_rift_tauri.md` updated: current-state line rewritten as STT-only, two resolved caveats dropped (`state_referenced_locally`, `mode-watcher`). `docs/design/` claim corrected (no longer empty — carries `assistant-roadmap.md`).

Verify: `npm run check` clean (0 errors, 0 warnings, 0 files-with-problems for the first time in the v0.4.1 era). Net diff across 16 files: **-1858 / +149** = -1709 lines.

**S90 stress-test fix-ups.** Autonomous CDP-driven pass across every UI surface (ActivityBar / chat tabs / right-pane / Assistant / Settings × 7 / Sync / Files / Terminal / Velopack / status bar). Two latent UX bugs caught + fixed: (1) `right-pane.svelte.ts::init()` clamped the in-state width but didn't re-persist, so an out-of-range stored value survived across launches — now writes back the clamped width on first load. (2) `Composer.svelte` rendered the mic button unconditionally, so clicking it with STT disabled silently set `stt.lastError` and looked broken — now gated on `stt.config.enabled && stt.supported`, paired with an `onMount(() => stt.init())` so the gate reflects real backend config (without that, users with STT enabled would lose the mic until they touched Settings → Speech once). Tooling: extended `scripts/cdp/serve.cjs` `KEY_DEFS` with `Comma / Slash / Space / Period / Backquote / ArrowLeft / ArrowRight` so future CDP runs can drive `Ctrl+,`, `Ctrl+\``, etc. directly.

## v0.4.3-alpha — 2026-05-17 — Voice arc: text-to-speech + speech-to-text

Two-direction voice integration. Both surface through a new **Settings → Voice** section and toggle from the Assistant header / Composer respectively.

### Text-to-speech (Claude → audio)

`msedge-tts` Rust crate calls Microsoft Edge's read-aloud endpoint (Azure Neural voices, free, no API key). `src-tauri/src/tts/mod.rs` owns a single tokio task that drains a sentence queue serially and emits MP3 b64 over `tts://audio`. Frontend [src/lib/state/tts.svelte.ts](src/lib/state/tts.svelte.ts) buffers streaming text per message id, splits on `/[.!?]+["')\]]?\s+/`, dispatches each completed sentence, and plays back-to-back via HTMLAudioElement. Cancel = generation counter bumps (drops in-flight + queued) plus local queue clear.

AssistantHeader speaker toggle = single-click `enabled + auto_speak` on; click again mutes auto-speak (master stays on so per-message replay still works). MessageBubble gets a speaker icon next to copy for one-shot replay. Settings carries the voice picker (~500 Edge voices, English first), rate/pitch/volume sliders (-50..+50), and a Test button.

### Speech-to-text (audio → composer)

WebView2's built-in `SpeechRecognition` (Edge/Chromium → Azure when online) writes directly into `assistant.composerDraft`. Live interim text streams as the user speaks; final committed text replaces interim segments on each phrase commit. No Rust-side audio capture, no model download, no build deps. [src-tauri/src/stt/mod.rs](src-tauri/src/stt/mod.rs) only owns settings persistence at `~/.rift/stt-config.json`. Composer gets a mic button on the left — click to record (pulsing red), click again to stop (focus returns to textarea, cursor at end).

Settings exposes language (12 BCP-47 locales), live-partials toggle, continuous mode toggle, append-vs-replace insertion mode. Microphone permission prompts once via WebView; subsequent uses are silent.

### Why not whisper.cpp local

Pivoted away from `whisper-rs` mid-session. Build-time libclang requirement on Windows broke `cargo run` w/o LLVM installed; bindgen route would have forced every dev (Trey included) to install LLVM. Web Speech API delivers comparable quality (same Azure backbone as the TTS path) with zero install footprint, true real-time streaming, and no first-launch model download. Trade-off: requires internet (so does Anthropic, so does the TTS).


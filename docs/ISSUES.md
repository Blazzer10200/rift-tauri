# Rift — Issue Tracker

> Captured 2026-05-19 across one session. Source: combined user observation + Blazzer-supervised audit (security fork + settings fork + inline spot-checks). Every claim w/ a `file:line` ref was verified against source the same turn it was made unless explicitly marked `UNVERIFIED`.
>
> **Session log:** 2026-05-19 — earlier session shipped #3b, #12, #13, partial #11 (all uncommitted). Parallel session shipped S106 telemetry overhaul (assistant.svelte.ts + scripts/cdp/serve.cjs) — fixed a thinking-block double-count bug (different from #1's double-write), added effortFlag/streamEventCount/toolUses/thinkingBlocks/maxStreamGapMs telemetry, verified envelope === result on simple turns. Continuation session (afternoon) verified #19 as non-bug + tightened doc, then shipped #9.1 (IPC token strip + DTO), #9.2 (chmod 0600 + delete-on-exit), and #10 (require_pinned_fingerprint guard). All stacks land via one /git-ship.
>
> This is a living document. Next session that touches Rift should:
> 1. Read this top-to-bottom before starting any new work.
> 2. Cross off issues as they ship, w/ a one-line note pointing to the commit/CHANGELOG entry that fixed them.
> 3. Append new findings to the end, never re-number — issue IDs are durable references.
>
> Priority tiers at the bottom — read those first if you only have 30 seconds.

---

## 1. Assistant context counter is "jumpy" / inaccurate

> **SHIPPED v0.4.12-alpha** — per-turn semantics kept; `lastTurnUsage` now only updates on the `result` event, not on `envelope`. Pill sits on the previous turn's confirmed value through the in-flight turn and lands on the new value once result arrives -- single update per turn, no visible jump on complex turns where envelope+result diverge. S106 telemetry capture (`envelopeUsage` vs `resultUsage` on `currentTurnRecord`) is preserved so divergence metrics keep their signal. Verified clean via `npm run check` (0/0).

- **Where:** [src/lib/components/assistant/AssistantHeader.svelte:85-89](../src/lib/components/assistant/AssistantHeader.svelte#L85-L89), [src/lib/state/assistant.svelte.ts:760](../src/lib/state/assistant.svelte.ts#L760) (line shifted from 560 → 760 post-S106 telemetry overhaul)
- **S106 update (2026-05-19):** other session verified `envelopeUsage === resultUsage on simple turns`, so the double-write is visually invisible on no-thinking/no-tool turns. Pill-jumping is therefore most likely on complex turns where the two diverge (e.g. when the result event corrects a partial envelope after thinking blocks). Per-turn vs cumulative semantics decision still needed.
- **Symptom:** Top-right context pill jumps up and down between turns (`729K / 1.00M 73%` one turn, lower the next), confusing users about how much context budget they actually have.
- **Root cause:** Two compounding things. (a) The pill shows `input + cacheRead + cacheCreate` for the *latest turn's prompt*, NOT cumulative session usage — that's by design but doesn't match user mental model. (b) `lastTurnUsage` is replaced TWICE per turn (once from the `envelope` event, once from the `result` event, see the `accumulate` flag), so the pill visibly updates twice. The S105 probe comment at line 561-563 already flags cache investigation as an open thread.
- **Fix sketch:** Decide on the semantics first — cumulative session window utilization (matches user expectation) or per-turn prompt size (current). If staying per-turn, suppress the `envelope`-source update from rendering and only emit on `result`. If switching to cumulative, derive from `sessionUsage` instead and rebrand the pill ("Session: 1.2M / 4× turns").

## 2. Tool-result blocks lack visual rhythm — "done" ambiguous mid-turn

- **Where:** [src/lib/components/assistant/MessageBubble.svelte](../src/lib/components/assistant/MessageBubble.svelte) (728L), inline diff renderer + StepGroup
- **Symptom:** When an assistant turn contains short narration → Edit block → more narration → another Edit block → final summary, the visual cadence reads as if the message ended after the first big block. User assumes Claude is done and looks away; new block appears "out of nowhere." Particularly bad w/ multi-Edit batches (verified in user screenshots S104 era).
- **Fix sketch:** Stronger end-of-turn marker (footer w/ cost + model + duration is partially there via `costLabel` / `modelLabel` derivations, but isn't visually distinct from a mid-turn block). Consider dimming/collapsing intermediate tool blocks once a turn finishes, tighter visual grouping of "narration + its tool call(s)" as one unit, or a "still working…" pulse on the role row until `streaming=false`.

## 3. Speech-to-text — accuracy + send-cancel + duplicate-on-stop

> **#3b SHIPPED v0.4.12-alpha** — `stt.svelte.ts:131-149` `consume()` now calls `recognition.abort()` + resets `recording/transcribing/recognition`/timer. Likely also resolves #3c — verify next runtime. #3a (accuracy) remains.

- **Where:** [src/lib/state/stt.svelte.ts](../src/lib/state/stt.svelte.ts), [src/lib/components/assistant/Composer.svelte:212,219,235,245-260](../src/lib/components/assistant/Composer.svelte#L212)
- **3a Accuracy:** General transcription quality needs polish — slurred input, technical terms ("Tauri", "Rift", "SFTP"), short utterances. Currently uses WebView2's Edge/Azure-backed `SpeechRecognition` w/ 3 alternates + best-confidence pick. *Ideas:* custom vocab/grammar (Web Speech doesn't support directly — would need swap to a Whisper-based pipeline), post-process punctuation pass, longer silence threshold.
- **3b Send doesn't cancel recording — VERIFIED BUG:** `stt.consume()` at [stt.svelte.ts:131-136](../src/lib/state/stt.svelte.ts#L131-L136) only sets the `consumed` flag and clears `finalText`/`baseDraft`/`lastTranscript`. It does NOT call `recognition.stop()` or `abort()`. All three Composer send paths (slash-fire, slash-pick, `fire()`) call `stt.consume()` but never `stt.cancel()`. Mic keeps recording silently after send.
- **3c Duplicate-on-stop:** User reports the just-sent transcript reappears in the prompt when they click mic to stop after sending. Code review of `onResult` and `onEnd` (both correctly guarded by `if (this.consumed) return;`) suggests this should NOT happen — needs runtime repro to root-cause. May be a race condition or a third code path writing to `assistant.composerDraft`.
- **Fix sketch:** In `consume()`, also call `this.recognition?.abort()` to hard-stop. That likely resolves 3c as a side effect since no further results can fire. Then revisit 3a as a separate epic.

## 4. UI/UX consistency + navigability sweep (app-wide)

- **Scope:** Not a single bug — tracking the user's stated goal of an app-wide consistency pass. Settings page is densest control surface and the natural starting point. App-wide pass after.
- **Goal:** Every visible control is wired, every section is necessary, terminology + styling consistent. Navigation is intuitive — current state has "hard to navigate" hotspots per user feedback.
- **Approach when actioned:** Per-page audit checklist (control → wired? necessary? consistent?). Hotspot list grows as specific pain points are flagged (currently #6 + #11 are concrete instances). [src/lib/components/settings/Settings.svelte](../src/lib/components/settings/Settings.svelte) is 1505L — the audit alone is non-trivial.

## 5. Live status indicator placement (QoL)

- **Where:** [src/lib/components/assistant/MessageBubble.svelte:189-194](../src/lib/components/assistant/MessageBubble.svelte#L189-L194) `stageLabel` derivation; renders inline at top of assistant bubble.
- **Symptom:** Status word ("Cogitating…", "Reading X", "Running cargo check") shows at the top of the in-flight response bubble, easy to miss while scrolling or focused on the input.
- **User proposal:** Surface the live status more prominently — possibly a small "hub" above the prompt input alongside the send + mic buttons. Could consolidate current activity, elapsed time, maybe per-turn token delta.
- **Fix sketch:** Defer to whoever does the frontend pass. Likely files: `src/lib/components/assistant/` (Composer + a new StatusStrip component), `src/lib/state/assistant.svelte.ts` (status state already exists since `Cogitating` renders).

## 6. Scrollbar collides w/ `+` button (top-right of assistant)

> **SHIPPED v0.4.12-alpha (9d0bb55)** — Phase 3c: `+` button moved out of the scrollable `.strip` to the right end of `ChatTabsBar.svelte` w/ a 5px gap from the activity-bar boundary; `scrollbar-gutter: stable` on the chat-thread scroller in `AssistantPage.svelte` kills the horizontal jump when overflow appears.

- **Where:** Source elusive from static grep. NOT the [ChatTabsBar](../src/lib/components/shell/ChatTabsBar.svelte) (that's horizontal w/ a 4px scrollbar). Most likely candidate: an `overflow-y: auto` on a parent container in [WorkspaceShell.svelte](../src/lib/components/shell/WorkspaceShell.svelte) or [AssistantPage.svelte:197](../src/lib/components/assistant/AssistantPage.svelte#L197) that doesn't reserve gutter space.
- **Symptom:** Vertical scrollbar visually crowds the `+` (new conversation / new tab) button at the top-right of the Assistant pane. Aesthetic + likely click-target issue.
- **Fix sketch:** Inspect via CDP (`bash scripts/cdp/c.sh eval ...`) to pinpoint the offending element. Options: move scrollbar to inner message list only, add right padding/margin so `+` clears it, `scrollbar-gutter: stable`, or overlay-style scrollbar.

## 7. New-user onboarding flow (untested cold-start path)

- **Where:** No dedicated first-run UI exists. [docs/ONBOARDING.md](ONBOARDING.md) is only 42 lines (see #24).
- **Symptom:** Unknown what happens on a fresh install — no profile, no SSH keys, no server configured, no Claude auth. Empty states across Sync / Assistant / Activity pages will likely confuse a new user.
- **Fix sketch:** Deliberate first-run flow — welcome → SSH key generate-or-import → profile setup → server add → Claude auth handoff → first sync. Empty states across every page should guide, not confuse. Should be self-contained: no manual file edits, no env vars.

## 8. Extend `scrubUser` pattern to log forwarding + IPC paths

- **Where:** [Settings.svelte:91-97](../src/lib/components/settings/Settings.svelte#L91-L97) `scrubUser()` redacts `C:\Users\<name>\` → `<user>` for the copy-diagnostic button. Pattern is good but not applied anywhere else.
- **Symptom:** Log forwarding from Rust to frontend (DiagBus + LogForwarder) passes raw paths unredacted (verified — see #9 and `diagnostics/mod.rs:326`). Anywhere a path crosses the IPC boundary or surfaces to a log line is a potential username leak.
- **Fix sketch:** Lift `scrubUser` into a shared util (frontend + Rust-side equivalent). Apply at every log-emission point in Rust and at every path-surfacing IPC return value. See also #9 for the specific Rust gap.

## 9. Bridge token stored plaintext in two places + leaked via IPC (HIGH)

> **9.1 + 9.2 SHIPPED v0.4.12-alpha.**
> - **9.1** New `ServerProfilePublic` DTO (profile/mod.rs) omits `bridge_token`, replaces it w/ `hasBridgeToken: bool`. `list_servers` + `save_server` return it. `save_server` preserves the on-disk token when an edit submits an empty value (mirrors existing fingerprint preserve pattern). Frontend `ServerProfile` type updated; `AddServer.svelte` summary shows "existing token" on edit. Verified via CDP — `bridgeToken` field is gone from `list_servers` JSON.
> - **9.2** `write_mcp_config` now chmods 0600 on Unix (Windows relies on NTFS inheritance from `%USERPROFILE%\.rift\`). New `pub fn cleanup_mcp_config_on_exit` removes `~/.rift/assistant/mcp-config.json` on `RunEvent::Exit` (lib.rs `.build().run(...)` hook). Best-effort, swallows errors. Verified — new binary boots cleanly and autosync runs.
> - **9.3** OS keyring / Tauri 2 secure-storage still deferred (Phase 6).

- **Where:**
  - [src-tauri/src/profile/mod.rs:30-36](../src-tauri/src/profile/mod.rs#L30-L36) — `bridge_token: Option<String>` written plaintext into `~/.rift/rift.json`. Source comment explicitly acknowledges this: "Tauri 2 secure-storage integration is on the Phase 6 list — keep this gap visible until then. File perms (~/.rift owner-only) are the only protection until then."
  - [src-tauri/src/assistant/mod.rs:528-546](../src-tauri/src/assistant/mod.rs#L528-L546) — `RIFT_BRIDGE_TOKEN` written verbatim into `~/.rift/assistant/mcp-config.json` via `std::fs::write`. No ACL set. File persists across sessions. Not cleaned on exit.
  - [src-tauri/src/lib.rs:743](../src-tauri/src/lib.rs#L743) — `list_servers()` returns full `Vec<ServerProfile>` (incl. `bridge_token`) to the renderer.
  - [src-tauri/src/lib.rs:766](../src-tauri/src/lib.rs#L766) — `save_server()` accepts + echoes full `ServerProfile` including token.
- **Symptom:** Token is on-disk plaintext, readable by any local process running as the user. Also flows over IPC to renderer code that has no legitimate need for it.
- **Token strength is fine:** 24 bytes of CSPRNG, URL-safe base64, constant-time comparison (`remote_bridge.rs:144`). Storage + exposure is the issue, not strength.
- **Fix sketch (in priority order):**
  1. Strip `bridge_token` from `list_servers` / `save_server` IPC return values. Smallest blast radius, biggest win — renderer doesn't need the raw value.
  2. Write `mcp-config.json` w/ owner-only ACL, OR delete on app exit. Currently survives reboots.
  3. Long-term: migrate to Tauri 2 secure-storage / OS keyring (Phase 6 plan).

## 10. Silent TOFU on first sync connect (MEDIUM, MITM window)

> **SHIPPED v0.4.12-alpha** — new `require_pinned_fingerprint(server_key, Option<&str>)` helper in lib.rs returns an actionable error string. Guard wired into three entry points: `scan_drift`, `start_autosync`, and (most importantly) `open_sftp_for` — the latter funnels ~9 IPC commands (remote_list_dir, upload/download, edit_in_place, sync_*, detect_bootstrap, etc.) so the single guard there closes the silent-TOFU window for all of them. Dead `persist_fingerprint_if_new` function removed (only sanctioned trust path is now `probe_server_fingerprint` → user confirm → `set_server_fingerprint`). Verified via CDP — autosync correctly transitions to `watching` for the pinned-fingerprint server.

- **Where:** [src-tauri/src/lib.rs:393](../src-tauri/src/lib.rs#L393) (sync scan), [src-tauri/src/lib.rs:452](../src-tauri/src/lib.rs#L452) (`start_autosync`). Both contain: `if server.fingerprint.as_deref().unwrap_or("").is_empty() { persist_fingerprint_if_new(...) }` — empty fingerprint → silent accept-and-pin.
- **Symptom:** First connection to a new server accepts ANY host key without user confirmation. MITM-during-onboarding risk.
- **Confirmation flow already exists:** [src-tauri/src/lib.rs:1494](../src-tauri/src/lib.rs#L1494) `probe_server_fingerprint` IPC is used by the AddServer dialog — but sync entry paths bypass it.
- **Post-pin enforcement is strict:** Mismatched fingerprint correctly hard-rejects (`ssh_handler.rs:52-55`).
- **Fix sketch:** Route all first-connect fingerprint capture through `probe_server_fingerprint`. Don't allow sync entry paths to silently TOFU. AddServer dialog already does it right — reuse.

## 11. Settings page dead UI cluster

> **PARTIAL — SHIPPED v0.4.12-alpha:** removed unused `uiPrefs` import (L11), added `Palette` to lucide imports and assigned it to Appearance (kills the Sparkles dup), dropped dead `lg` modifier on `srv-dot` (L941), descriptive aria-labels on the three terminal toggles (`Blink cursor` / `Copy on select` / `Right-click paste`). **Remaining:** Appearance "More coming soon" placeholder (L246-251), empty SSH Keys section (L964-975), STT-namespaced font-picker class rename (L736-758) — all non-mechanical, deferred.

- **Where:** [src/lib/components/settings/Settings.svelte](../src/lib/components/settings/Settings.svelte) (verified inline):
  - **Line 11** — `uiPrefs` imported, never used in template or script (only hit in entire file).
  - **Lines 117, 119** — Both "Appearance" and "Assistant" nav items use `Sparkles` icon; every other section has a distinct icon.
  - **Line 941** — `<span class="srv-dot lg">` references a `.lg` modifier class that has no matching CSS rule (only bare `.srv-dot` at line 1191).
  - **Lines 246-251** — Appearance "More coming soon" placeholder card.
  - **Lines 964-975** — SSH Keys section: header + one button only. No key inventory, no fingerprints, no management UI.
  - **Lines 736-758** — Accessibility font picker reuses STT-namespaced classes (`stt-lang-grid` / `stt-lang-pick`); works but misleading.
  - **Lines 458, 513, 529** — Three toggles use generic `aria-label="Toggle"` while every other switch has descriptive labels (accessibility regression).
- **Fix sketch:** Mechanical sweep. Remove unused `uiPrefs` import, assign a distinct icon to Appearance, drop the `lg` modifier OR add the CSS rule, expand or remove the SSH Keys empty section, extract shared `.option-grid` utility for the font picker, write descriptive aria-labels.

## 12. Manual 3-file version bump is the #1 ship-failure mode

> **SHIPPED v0.4.12-alpha** — `scripts/bump.ps1` accepts a semver arg, regex-replaces the first version line in all three files, post-bump cross-checks all three match. Patterns dry-ran clean (1 match each, current version). Usage: `pwsh ./scripts/bump.ps1 0.4.12-alpha`. Em-dashes replaced w/ `--` to avoid PS5.1 BOM-loss mojibake on future Edits.

- **Where:** [package.json](../package.json), [src-tauri/Cargo.toml](../src-tauri/Cargo.toml), [src-tauri/tauri.conf.json](../src-tauri/tauri.conf.json) — all three must match. [scripts/release.ps1:28-37](../scripts/release.ps1#L28-L37) detects mismatch and bails.
- **Symptom:** Per CLAUDE.md gotcha #5 + HANDOFF history, this is the #1 ship-attempt failure mode. v0.2.49's first ship attempt died here. The script catches the mistake but doesn't fix it.
- **Fix sketch:** `scripts/bump.ps1 <new-version>` that writes all three files in one shot (~25 lines of PowerShell). Optionally appends a CHANGELOG `## <version> — <date>` stub w/ blank `### Added / Changed / Fixed` headers. Optionally `release.ps1`'s preflight can offer to auto-bump on mismatch w/ a `-AutoBump` flag.

## 13. Release notes never flow into the GitHub release

> **SHIPPED v0.4.12-alpha** — `release.ps1` now extracts the top `## v<version>` entry body from `docs/CHANGELOG.md` (only when the entry version matches the bumped version, else warns and skips), writes to `$env:TEMP/rift-release-notes-<version>.md`, passes `--releaseNotes` to `vpk pack` (verified flag — `vpk upload github` has no such flag), cleans up on success. Dry-run extracted v0.4.11-alpha body cleanly (2155 chars).

- **Where:** [scripts/release.ps1:119-130](../scripts/release.ps1#L119-L130) — `vpk upload` args do not include `--releaseNotes` / `--releaseNotesFile`.
- **Symptom:** Published GitHub release body is empty. CHANGELOG.md entry sits unused. Velopack client can't surface "what's new" in the in-app update dialog.
- **Fix sketch:** Parse `docs/CHANGELOG.md` for the top entry (regex match on `## <version> —`), write to a temp file, pass `--releaseNotesFile $tmpPath` to `vpk upload`. ~15 lines of PowerShell.

## 14. No CI — every release is local-only

- **Where:** No `.github/workflows/` directory exists in the repo.
- **Symptom:** 5-15 min wall time blocking your machine per release. Mid-run build failure leaves stale `Releases/staging-*` (cleanup is on success path only at line 138). No cross-machine reproducibility — only works on Blazzer's box w/ `gh auth` + `vpk` + Node toolchain installed.
- **Fix sketch:** `.github/workflows/release.yml` triggered on `v*` tag push. Job: checkout → setup-node → cargo cache → `dotnet tool install -g vpk` → run `release.ps1` w/ `GITHUB_TOKEN`. Removes local toolchain dependency. Add a cleanup hook for `Releases/staging-*` on failure.

## 15. Unsigned Windows builds (SmartScreen blocker)

- **Where:** Acknowledged in [scripts/release.ps1:4-5](../scripts/release.ps1#L4-L5): "Unsigned for now (audit H4 — signing deferred until cert + AAS budget is in place)."
- **Symptom:** Every fresh install triggers Windows SmartScreen "Unknown publisher" dialog. Real adoption blocker for non-technical users.
- **Fix sketch (options ranked by cost/value):**
  1. **Azure Code Signing** (~$10/mo) — EV-equivalent reputation, no hardware token, CI-friendly.
  2. **SignPath.io free OSS tier** — only viable if the repo goes public.
  3. **DigiCert/Sectigo EV cert** (~$300-400/yr) — instant SmartScreen reputation, hardware token, less CI-friendly.
- **Pipeline integration:** `vpk pack --signParams` flag exists; CI secret holds the cert handle.

## 16. Custom `GithubSource` is 130 lines of SDK-gap debt

- **Where:** [src-tauri/src/update_service.rs:138-269](../src-tauri/src/update_service.rs#L138-L269).
- **Symptom:** Written because velopack-rust 0.0.1298 ships no `GithubSource` type — `AutoSource` for an HTTP URL only works against flat static `releases.{channel}.json`, not real GitHub release pages. Reimplements REST asset enumeration, prerelease filtering, asset-URL caching, download streaming. Every bug in this code is yours; every bug fixed in upstream Velopack you don't get.
- **Fix sketch:** Check current velopack-rust crate version. If newer ships a `GithubSource`, swap impl + delete 130 lines. If still missing, file an upstream issue/PR — the impl is reusable.

## 17. Two-repo split exists only for the velopack auth gap

- **Where:** Coupling btw [scripts/release.ps1:62](../scripts/release.ps1#L62) (`Blazzer10200/rift-releases` hardcoded) + [src-tauri/src/update_service.rs:29-30](../src-tauri/src/update_service.rs#L29-L30) (`GITHUB_OWNER` / `GITHUB_REPO` consts).
- **Symptom:** Every release requires manual sync btw private source repo and public releases repo. Forks/contributors can't test the update path against the real source.
- **Fix sketch:** Revisit when #16 lands. If upstream `GithubSource` supports a token, the public-mirror repo becomes optional. Until then, document the split prominently in `CONTRIBUTING.md` so it's not folklore.

## 18. No round-trip verify post-publish

- **Where:** [scripts/release.ps1:134-135](../scripts/release.ps1#L134-L135) — only runs `gh release view` (metadata only).
- **Symptom:** Script reports success even if Setup.exe is corrupt, wrong asset name, or missing nupkg. You only find out when users' auto-update fails.
- **Fix sketch:** Cheap baseline — after `vpk upload`, `gh release download $tag --pattern "*Setup.exe" -D Releases/verify-$version/` then SHA256 against the local pre-upload artifact. Better: launch the downloaded Setup.exe in a sandbox/VM via CI smoke test.

## 19. `apply_updates` IPC may not stop autosync before binary swap (RESOLVED — non-bug)

> **RESOLVED v0.4.12-alpha** — verified [lib.rs:1554-1573](../src-tauri/src/lib.rs#L1554-L1573) `apply_updates` already stops autosync (engine.stop) + tunnel (t.stop) BEFORE `spawn_blocking(|| UpdateService::new().apply())`. The audit re-read mistook the boundary — frontend correctly does NOT stop anything; the Tauri command layer owns it. Tightened doc comment at `update_service.rs:82-88` to make explicit that direct callers of `UpdateService::apply` must do their own stop, but the `apply_updates` command handles it.

- **Where:** Comment at [src-tauri/src/update_service.rs:85-86](../src-tauri/src/update_service.rs#L85-L86) explicitly says "Caller MUST stop autosync + tunnel BEFORE invoking — in-flight uploads die when the process exits." Frontend [src/lib/state/updates.svelte.ts:40-50](../src/lib/state/updates.svelte.ts#L40-L50) just calls `invoke("apply_updates")` with no JS-side stop.
- **Symptom (if confirmed):** In-flight SFTP uploads + tunnel sessions die mid-operation when process exits for binary swap. Possible orphaned `.rift-tmp` files on remote, possible partial uploads to the running FXServer.
- **Verification needed:** Grep `lib.rs` for the `apply_updates` Tauri command body — does it call into autosync/tunnel stop before delegating to `update_service.apply()`? If yes, the comment is stale (still worth fixing for clarity). If no, this is a real bug.

## 20. Hot files exceeding the 2000-line agent-split threshold

- **Where:** Per CLAUDE.md agent-routing guidance, files >2000 lines are agent-bail risks. Current state:
  - [src/lib/state/assistant.svelte.ts](../src/lib/state/assistant.svelte.ts) — **2320L (over and growing; +233L from S106 telemetry overhaul 2026-05-19)**
  - [src-tauri/src/sync/auto_sync.rs](../src-tauri/src/sync/auto_sync.rs) — 1966L (right at the edge)
  - [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) — 1790L (next in line, already queue item (e) in CLAUDE.md)
- **Symptom:** Targeted edits become brittle, LSP slows down, agents bail mid-emit on audit-shaped prompts.
- **Fix sketch:** Split each by domain. For `lib.rs`: per-domain `commands/*.rs` (sync, sftp, profile, assistant, update). For `assistant.svelte.ts`: extract per-concern classes (tabs, streaming, usage, tasks). For `auto_sync.rs`: continued extraction along the `flush.rs` / `watch.rs` precedent.

## 21. Zero test coverage anywhere in the repo

- **Where:** I found no `test/` dir at repo root, no substantive `#[cfg(test)]` blocks in `src-tauri/src/`, no vitest / playwright config, no Rust integration tests dir.
- **Symptom:** For a release-grade app that moves real files over SFTP, atomically renames into a running FXServer, and resolves drift between three states — zero automated tests is genuinely scary. One regression in `flush_batch`, the drift reconciler, or the ignore-rule parser can corrupt user data silently.
- **Severity:** HIGH for long-term sustainability, LOW for current alpha velocity.
- **Fix sketch:** Start small — Rust unit tests for `sync/ignore.rs` (pure logic, no I/O, easy wins), `state/sync_snapshot.rs` (serialization), `sftp/transfer.rs` atomic rename semantics (needs mock SFTP). Frontend: vitest for `stt.svelte.ts` consume/onResult/onEnd state machine, `assistant.svelte.ts` usage accumulation.

## 22. `console.debug` / `console.warn` noise in production

- **Where:**
  - [src/lib/state/assistant.svelte.ts:764](../src/lib/state/assistant.svelte.ts#L764) — S105 cache-hit-ratio probe (line shifted from 564 post-S106).
  - [src/lib/state/assistant.svelte.ts:939](../src/lib/state/assistant.svelte.ts#L939) — new `console.debug` added by S106 telemetry layer.
  - [src/lib/state/assistant.svelte.ts:821](../src/lib/state/assistant.svelte.ts#L821) — `console.debug` for non-JSON idle stream lines.
  - [src/lib/state/stt.svelte.ts:104, 202, 266](../src/lib/state/stt.svelte.ts#L104) — warns on routine paths (config load fail, stop fail, recognition error).
- **Symptom:** Users w/ devtools open see scary-looking spam during normal operation.
- **Fix sketch:** Strip the S105 probe log (it's served its purpose if cache investigation is complete). Gate routine warns behind a dev flag or downgrade to `console.debug`. Reserve `console.warn` for actionable failure modes only.

## 23. `use_full_config=true` admits broad MCP tools beyond Rift's own

- **Where:** `src-tauri/src/assistant/mod.rs:1193` (per security fork — I did not re-verify line number, claim is the fork's).
- **Symptom:** Passes `mcp__*` to `--allowed-tools`, which lets the model use any tool from user-side MCP servers the Claude CLI merges in. Surface area users should know they're opting into when flipping the related toggle.
- **Fix sketch:** Document explicitly in Settings → Assistant near the toggle ("This admits MCP tools from your global Claude config — see ~/.claude/.mcp.json for what's enabled"). Optionally allowlist instead of wildcarding.

## 24. `docs/ONBOARDING.md` is only 42 lines — too thin for the first-run experience

- **Where:** [docs/ONBOARDING.md](ONBOARDING.md).
- **Symptom:** Way too thin for the cold-start path #7 is worried about. No screenshots, minimal walkthrough.
- **Fix sketch:** Expand before any public release. Cover: install from Setup.exe, SmartScreen warning (until #15 lands), SSH key setup (generate or import), first server profile, first sync, troubleshooting (no SFTP connect, no fingerprint match, no Claude CLI found).

## 25. velopack-rust pinned at pre-1.0 version

- **Where:** [src-tauri/Cargo.toml](../src-tauri/Cargo.toml) — `velopack = "0.0.1298"` per source comment in `update_service.rs`.
- **Symptom:** Pre-1.0 software (no SemVer guarantees) carrying your entire distribution path. Every minor bump can break the custom `GithubSource` from #16. Implicit risk every `cargo update`.
- **Fix sketch:** Pin the exact version in Cargo.toml (not a range). Audit upgrade cadence — check the velopack-rust changelog before each bump. Coordinate w/ #16 (when upstream `GithubSource` lands, that's the upgrade trigger worth taking).

## 26. `.gitignore` audit for `Releases/staging-*`

- **Where:** [scripts/release.ps1:90-92, 138](../scripts/release.ps1#L90) — creates `Releases/staging-$version/`, deletes on success.
- **Symptom:** Mid-run build failure leaves the staging dir on disk. Next `git add .` could accidentally commit build artifacts if `.gitignore` doesn't cover it.
- **Fix sketch:** Verify `.gitignore` has `Releases/staging-*` (or `Releases/` entirely if no other content lives there). One-line check.

---

## Backend hardening — migrated from AUDIT.md 2026-05-19

Open audit items folded in when AUDIT.md was archived to `docs/archive/AUDIT-fix-log.md`. All low-severity backend hardening; full fix-pass history (S81-S86 + Codex passes) lives in the archive.

## 27. `atomic_write_json` blocks a Tokio worker (LOW)

- **Where:** [src-tauri/src/state/paths.rs:68](../src-tauri/src/state/paths.rs#L68).
- **Symptom:** `std::thread::sleep` retry loop runs on the async cmd thread, blocking a Tokio worker for up to the retry window.
- **Fix sketch:** Wrap the retry+write in `tokio::task::spawn_blocking`, or convert to `tokio::time::sleep` + async fs write.

## 28. Dual HTTP stacks — reqwest + ureq (LOW, deferred)

- **Where:** [src-tauri/Cargo.toml:41-44](../src-tauri/Cargo.toml#L41-L44).
- **Symptom:** Two HTTP transports compiled in. reqwest is async (used app-wide); ureq is sync (forced by `velopack` 0.0.1298's sync `UpdateSource`).
- **Fix sketch:** Revisit when velopack ships an async source. No action until then — pin acknowledged.

## 29. CSP allows `style-src 'unsafe-inline'` (LOW)

- **Where:** [src-tauri/tauri.conf.json:24](../src-tauri/tauri.conf.json#L24).
- **Symptom:** Inline styles permitted — required by current Tailwind output, weakens CSP.
- **Fix sketch:** Switch to nonce/strict-dynamic once Tailwind supports hashed inline styles end-to-end.

## 30. `capabilities/default.json` uses broad `core:default` (LOW)

- **Where:** [src-tauri/capabilities/default.json:7](../src-tauri/capabilities/default.json#L7).
- **Symptom:** `core:default` pulls in a superset of permissions; specific `core:*` perms actually in use are a subset.
- **Fix sketch:** Audit actual `invoke()` surface, pin specific `core:*` perms, drop the wildcard.

## 31. `capabilities/default.json` `opener:default` unscoped (LOW)

- **Where:** [src-tauri/capabilities/default.json:12](../src-tauri/capabilities/default.json#L12).
- **Symptom:** Opener plugin can launch any URL/path the renderer hands it.
- **Fix sketch:** Scope to known prefixes — update URL, docs URL, `https://github.com/Blazzer10200/*`.

## 32. `transport/env.rs::hostname` shells out on non-Windows (INFO)

- **Where:** [src-tauri/src/transport/env.rs:16-24](../src-tauri/src/transport/env.rs#L16-L24).
- **Symptom:** Spawns external `hostname` binary on macOS/Linux — ambient PATH risk if an attacker controls `$PATH` for the user's shell.
- **Fix sketch:** Read `/proc/sys/kernel/hostname` or use `gethostname` crate; absolute-path the binary call as a fallback.

## 33. `lib.rs::local_list_dir` missing profile containment (open from 2026-05-11)

- **Where:** `src-tauri/src/lib.rs` `local_list_dir` Tauri cmd.
- **Symptom:** Lacks `path_guard::validate_local_child` against active profile's `local_root`. Skipped at fix-pass time b/c the command has no `server_key` input — fixing requires a frontend contract change.
- **Fix sketch:** Add a `server_key` param, validate against that profile's roots. Frontend `LocalPane` calls pass the active server. Coordinated frontend + backend change.

Also accepted as INFO (no action expected): `path_guard.rs:21` Linux-only remote containment (matches Rift's deploy target); `bridge/mod.rs:57` token over loopback HTTP (documented); `edit/edit_trail.rs:75-80` subdir PID-race (collision astronomical after `short_id` widened to 8 bytes).

---

## Priority tiers

**S120 (uncommitted) — Wave-2 backend MED + LOW sweep, ~40 issues SHIPPED.** Full lane breakdown in `docs/HANDOFF.md`. New SHIPPED: #54 #55 #56 #68 #70 #72 #73 #75 #77 #79 #80 #83 #84 #86 #91 #93 #94 #95 #97 #98 #101 #105 #110 #111 #114 #116 #117 #118 #121 #122 #123 #124 #126 #128 #130 #132 #133 #136 #137 #138. Pending `/git-ship` → v0.4.17-alpha.


**Tier 0 — verify before anything else**
- ~~#19 `apply_updates` autosync-stop~~ — verified 2026-05-19, non-bug (Tauri cmd already handles it; doc tightened)

**Tier 1 — ship blockers / data safety**
- #21 Zero test coverage — see #265 for plan; reality 35 tests in 10 files; uncovered HIGH-risk modules listed
- #9 Bridge token plaintext + IPC leak — 9.1+9.2 SHIPPED 2026-05-19; 9.3 (OS keyring) deferred to Phase 6
- #15 Unsigned Windows builds (adoption blocker)
- ~~#10 Silent TOFU on first sync~~ — SHIPPED 2026-05-19 (`require_pinned_fingerprint` guard)
- ~~#36~~ `save_server` overwrites server list — SHIPPED v0.4.14-alpha S113 (`map_err` propagation)
- **#37** API key plaintext in `assistant/config.json` (mirrors #9.3 deferred — Phase 6)
- **#38** `mcp-config.json` Windows DACL gap (continues #9.2 — Phase 6)
- ~~#42~~ — VERIFIED NOT A BUG by Wave 3 T (see verdict above). Closed.
- ~~#41~~ Bridge lock leak — SHIPPED v0.4.14-alpha S113 (`BridgeLockGuard` RAII w/ Drop)
- ~~#74~~ `walk_local` panic → mass ToPull — SHIPPED v0.4.14-alpha S113 (JoinError → SuspiciousEmptyAborted)
- ~~#219~~ No panic hook — SHIPPED v0.4.14-alpha S113 (`std::panic::set_hook` after LogForwarder)
- ~~#220~~ session_id traversal — SHIPPED v0.4.14-alpha S113 (`is_valid_session_id` UUID guard)

**Tier 2 — recurring friction**
- ~~#12 Manual 3-file version bump~~ — shipped 2026-05-19 (`scripts/bump.ps1`)
- ~~#3b STT send doesn't stop recognizer~~ — shipped 2026-05-19 (`stt.svelte.ts` `consume()`)
- ~~#1 Context counter semantics + double-write~~ — shipped 2026-05-19 (per-turn, result-only render)
- #14 No CI (deferred — pairs w/ #15 signing)
- ~~#34~~ Cancel-token ownership — SHIPPED v0.4.14-alpha S113 (u64 nonce identity compare; agent + 4-site fixup)
- ~~#35~~ Perm-heal spawn untracked — SHIPPED v0.4.14-alpha S113 (`track_background(h)`)
- ~~#39~~ Stop-flag race — SHIPPED v0.4.14-alpha S113 (post-PID re-check + mark_session_stopped)
- ~~#40~~ Single shared mcp-config.json — SHIPPED v0.4.14-alpha S113 (per-session `mcp-config-<id>.json` + `McpConfigGuard` Drop)
- ~~#43~~ is_pushing race — SHIPPED v0.4.14-alpha S114 (Err→true safer direction)
- ~~#44~~ stop_watch order — SHIPPED v0.4.14-alpha S114 (unwatch before remove)
- ~~#45~~ FS-drop counter — PARTIAL S114 (AtomicU64 + 100-drop Error escalation; AutoSyncStatus exposure deferred)
- ~~#46~~ pending_dir_reconcile order — SHIPPED v0.4.14-alpha S114 (kick before flag-clear)
- ~~#48~~ force_pull_now poison silent — SHIPPED v0.4.14-alpha S114 (diag Error + closing DriftScanResult)
- ~~#57~~ download_paths ghost CT — SHIPPED v0.4.14-alpha S114 (open_sftp before CT register)
- ~~#61~~ probe_server_fingerprint write probe — SHIPPED v0.4.14-alpha S114 (write_probe_root: None)
- ~~#63~~ SESSION_PIDS poison silent — PARTIAL S114 (into_inner recovery; orphan-kill on drop deferred)
- ~~#64~~ CLAUDE_EXE stale — SHIPPED v0.4.14-alpha S114 (Mutex<Option<Option<PathBuf>>> w/ is_file revalidate)
- ~~#65~~ save_config torn-write — SHIPPED v0.4.14-alpha S114 (tmp+rename pattern)
- **Remaining sync MEDs:** #47 (CT plumbing), #58 (silent list_recursive), #59 #60 (post-expansion guard refactor), #62 (token split — needs FE coord)
- ~~#139~~ `drainTick` rAF on dropped tab — SHIPPED v0.4.14-alpha S113 (`dropTab` calls `flushPendingText`)
- ~~#140~~ confirmMirrorApply bucket — SHIPPED v0.4.14-alpha S113 (re-filter at dispatch verified already correct; invariant doc'd)
- ~~#141-#142~~ ActivityFeed reactivity — SHIPPED v0.4.14-alpha S113 (`$state` + `untrack` — Agent C, clean)
- ~~#143~~ store-level per-tab fields — SHIPPED v0.4.14-alpha S114 (moved to TabState w/ delegating getters)
- ~~#144~~ closeTabsToRight/closeOtherTabs leak — SHIPPED v0.4.14-alpha S114 (dropTab + pruneTabUi per removed id)
- ~~#145~~ scheduleSave cross-tab save — SHIPPED v0.4.14-alpha S114 (per-tab saveTimer + tab-snapshot capture + flushNow iterates all tabs)
- ~~#150~~ Settings $effect draft clobber — SHIPPED v0.4.14-alpha S114 (untrack store reads; effect tracks only section)

**Tier 3 — UX + cleanup**
- #11 Settings dead UI cluster — PARTIAL shipped v0.4.12-alpha (mechanical bits done; placeholder card + SSH Keys empty + font-picker class rename remain)
- #2 Tool-block rendering rhythm
- ~~#22 Console noise~~ — shipped v0.4.12-alpha (assistant.svelte.ts probes removed, stt warns downgraded)
- ~~#6 Scrollbar collision~~ — shipped v0.4.12-alpha (Phase 3c — `+` button + `scrollbar-gutter: stable`)
- #5 Status indicator placement
- ~~#13 Release notes auto-flow~~ — shipped v0.4.12-alpha (`release.ps1` `--releaseNotes`)

**Tier 4 — strategic / longer-term**
- #4 App-wide UX consistency sweep
- #7 New-user onboarding flow + #24 ONBOARDING.md expansion
- #20 Hot-file split threshold (start w/ `lib.rs` per CLAUDE.md queue item (e))
- #16 Custom `GithubSource` → upstream Velopack
- #17 Two-repo split debt
- #8 Extend `scrubUser` to log forwarding
- #18 Round-trip publish verify
- #25 velopack-rust pin policy
- #23 MCP tool wildcarding documentation
- #26 `.gitignore` audit for `Releases/staging-*`
- #3a STT accuracy polish (separate epic — likely needs Whisper pipeline)
- #3c STT duplicate-on-stop (likely resolved by #3b fix, verify after)

**Tier 5 — backend hardening (LOW, opportunistic)**
- #27 `atomic_write_json` blocks Tokio worker
- #29 CSP `style-src 'unsafe-inline'`
- #30 `core:default` capability superset
- #31 `opener:default` unscoped
- #33 `local_list_dir` profile containment (needs FE contract change)
- #32 `transport/env.rs::hostname` shell-out (INFO)
- #28 Dual HTTP stacks (blocked on velopack async)
- **Wave-1 LOWs** #91-#134 — see entries above; mostly clippy-adjacent, doc gaps, perf nits
- **Wave-1 INFOs** #135-#138 — comment/ordering tweaks only

---

## Audit 2026-05-20 — Wave 1 (backend deep audit)

> 11 parallel `operator` agents over `src-tauri/src/`. Reports persisted at `state/audit-2026-05-20/{A..K}-*.md`; synthesis at `SYNTHESIS-wave1.md`. Format below is compressed (Where / Symptom / Fix); see reports for root-cause detail. Dupes collapsed where multiple agents found same site.

### HIGH (9)

## 34. Sync cancel-token slot has multiple ownership bugs

> **SHIPPED v0.4.14-alpha S113 (uncommitted)** — added `cancel_nonce: AtomicU64` field; slot type now `Option<(u64, CancellationToken)>`. Install sites generate nonce via `fetch_add`, clear-time compares stored nonce to local nonce (identity, not state). `cancel_drift_reconcile` updated to `entry.1.cancel()`. Wave-2 Agent A correctly wired struct + install (L243-245, L752, L1096) but bailed before the ripple-edits — I patched 4 read sites (L1212 force_push clear, L1531-1534 force_pull install, L1774 force_pull clear, L1789 cancel). `cargo check` green.

- **Where:** [auto_sync.rs:747](../src-tauri/src/sync/auto_sync.rs#L747) (push), [:1090](../src-tauri/src/sync/auto_sync.rs#L1090) (reconcile), [:1205](../src-tauri/src/sync/auto_sync.rs#L1205) + [:1767](../src-tauri/src/sync/auto_sync.rs#L1767) (clear logic)
- **Symptom:** One shared `current_scan_cancel` slot — `force_push_now` and `kick_drift_reconcile` overwrite each other; cleanup guard `stored.is_cancelled() == ct.is_cancelled()` boolean-compares state instead of identity, so any op finishing cleanly can clear the next op's token.
- **Fix:** Separate slots per op type, OR store a `u64` nonce alongside each token and compare nonces at clear-time. (Cross-agent dupe: A1+B1+C1.)

## 35. Perm-heal `tokio::spawn` not tracked — outlives engine `stop()`

> **SHIPPED v0.4.14-alpha S113 (uncommitted)** — `watch.rs:108-115` captures the JoinHandle and calls `self.track_background(h)`, matching the L92 lock-sweep pattern. Inline fix (not delegated).

- **Where:** [auto_sync/watch.rs:108](../src-tauri/src/sync/auto_sync/watch.rs#L108)
- **Symptom:** Untracked spawn holds `Arc<SftpClient>`; after `stop()` aborts `background_tasks`, perm-heal keeps issuing `chmod` against a disconnected/reconnected session. The lock-sweep spawn at L92 correctly calls `track_background`.
- **Fix:** Capture the JoinHandle and call `self.track_background(h)`.

## 36. `save_server` silently discards entire server list on config load error

> **SHIPPED v0.4.14-alpha S113 (uncommitted)** — `lib.rs:805` `.or_else(|_| Ok::<_, String>(RiftConfig::default()))` replaced w/ `.map_err(|e| format!("failed to load rift config: {e}"))?`. Load failures now propagate; never fall back to empty default before a save. Closes the data-loss path.

- **Where:** [lib.rs:805](../src-tauri/src/lib.rs#L805)
- **Symptom:** Any I/O error reading `~/.rift/*.json` → `.or_else(|_| Ok::<_, String>(RiftConfig::default()))` falls back to empty default; `cfg.save()` then overwrites the file with only the new server. **Real data-loss path.**
- **Fix:** Propagate the load error with `?`; never fall back to default on save_server.

## 37. API key plaintext in `~/.rift/assistant/config.json`
- **Where:** [assistant/mod.rs:209](../src-tauri/src/assistant/mod.rs#L209), [:490](../src-tauri/src/assistant/mod.rs#L490)
- **Symptom:** `AssistantConfig.api_key: Option<String>` serialized cleartext. No keychain, no encryption. Mirrors #9 pattern but separate file. Source comment acknowledges "keychain migration planned".
- **Fix:** Stronghold / OS keychain. Pair w/ #9.3 (Phase 6).

## 38. `mcp-config.json` Windows DACL not tightened
- **Where:** [assistant/mod.rs:547-557](../src-tauri/src/assistant/mod.rs#L547-L557)
- **Symptom:** Unix sets 0600; Windows defers to NTFS inheritance. Assumption that `%USERPROFILE%\.rift\` is user-only is wrong on domain-joined / shared-profile setups where inheritance can grant SYSTEM/Administrators/other users read. Bridge token leaks to those readers.
- **Fix:** After write on Windows, call `icacls` or `SetNamedSecurityInfo` (`windows-permissions` crate) to set explicit user-only DACL. Continues #9.2 work.

## 39. Stop flag consumed before PID registered — Stop button no-op

> **SHIPPED v0.4.14-alpha S113 (uncommitted)** — kept pre-spawn `take_session_stopped` (stale-marker hygiene for retry-after-stop turns) AND added a post-PID re-check at `assistant/mod.rs:1362-1374`. If a stop arrives during spawn, the re-check sees it, calls `child.start_kill()`, and re-marks via `mark_session_stopped` so the post-wait branch at L1456 emits the normal stop-path done event.

- **Where:** [assistant/mod.rs:1313-1316](../src-tauri/src/assistant/mod.rs#L1313-L1316)
- **Symptom:** `take_session_stopped` (L1313) clears stop marker before `set_session_pid` (L1316). Concurrent `assistant_stop` between those lines finds no PID, returns Ok, discards stop intent; CLI child runs unkilled.
- **Fix:** Move `take_session_stopped` AFTER `set_session_pid`; re-check the stopped flag post-PID-registration before entering wait loop.

## 40. Single shared `mcp-config.json` across concurrent tabs

> **SHIPPED v0.4.14-alpha S113 (uncommitted)** — `write_mcp_config` now takes `session_id` and writes to `~/.rift/assistant/mcp-config-<session_id>.json`. New `McpConfigGuard` struct (Drop impl removes file) bound to `_mcp_guard` at the spawn site survives panic + cancellation. `cleanup_mcp_config_on_exit` extended to glob `mcp-config-*.json` plus legacy fixed name. 0600 chmod on Unix preserved; Windows DACL gap still tracked as #38. Wave-2 Agent B did the edits cleanly; bailed on the report only.

- **Where:** [assistant/mod.rs:514,546](../src-tauri/src/assistant/mod.rs#L514)
- **Symptom:** `write_mcp_config` always writes one fixed path. Two concurrent `assistant_send` (multi-tab) race — second writer's config (different roots/bridge creds) is what the first-spawned CLI reads.
- **Fix:** Per-call temp file `mcp-config-<session_id>.json`, pass via `--mcp-config`, delete after `child.wait()`.

## 41. Bridge lock acquired but never released on exec error

> **SHIPPED v0.4.14-alpha S113 (uncommitted)** — new `BridgeLockGuard` struct in `assistant/remote_bridge.rs` holds `Arc<LockPresence>` + key; `Drop` spawns a tokio task that calls `locks.release(&key).await`. Lock release now survives panics + future cancellation, not just normal returns. Closes the permanent-block-of-remote-root window.

- **Where:** [assistant/remote_bridge.rs:250-258](../src-tauri/src/assistant/remote_bridge.rs#L250-L258)
- **Symptom:** `locks.acquire(&lock_key)` at L250 unconditional; `locks.release` at L256 only reached on normal return. Panic, early return, or secondary `eng.locks()` returning `None` leaks the advisory shell lock — permanently blocks all other users on that remote root.
- **Fix:** RAII guard struct (Drop calls release), or `scopeguard::defer!`.

## 42. Conflict copy can re-enter dirty queue → re-upload to remote
- **Where:** [drift_watcher.rs:64](../src-tauri/src/sync/drift_watcher.rs#L64) + [ignore.rs:157](../src-tauri/src/sync/ignore.rs#L157)
- **Symptom:** `derive_conflict_path` writes `.rift-conflict.` filename; `mark_recently_written` is set on target only, not original. If `watch.rs` event filter applies `should_ignore` to the absolute path (not rel-path), the conflict-marker substring guard may miss → conflict copy queued + pushed to remote.
- **Fix:** Verify watch.rs event handler uses relative-path `should_ignore`; if not, strip prefix before the check.
> CLOSED v0.4.16-alpha S119 (uncommitted) — non-bug confirmed. `ignore::classify` (ignore.rs:87) normalizes `\→/` then extracts filename via `rsplit('/').next()` — substring `.rift-conflict.` matches against the filename regardless of whether an absolute or rel path is passed. `watch.rs:243` `is_recently_written` fires BEFORE classify and targets `target_local` (which IS the conflict path the watcher sees). Both guards correctly trigger. No code change required.

### MED (45)

## 43. `is_pushing()` false-negative race — pull races tail of push flush
- **Where:** [auto_sync.rs:625-634](../src-tauri/src/sync/auto_sync.rs#L625-L634)
- **Symptom:** If `flush_batch` removes last dirty entry between `dirty.is_empty()` and `state.try_lock()`, both checks return false → pull races with in-flight upload, possibly overwriting mid-upload. `try_lock` Err arm silently returns `false` (unsafe direction).
- **Fix:** `pushing_in_flight: AtomicBool` set at `flush_batch` entry/exit; OR `is_pushing` returns `true` on `try_lock` Err.
> SHIPPED v0.4.14-alpha S114 (uncommitted) — `Err(_) => true` (safer direction). The AtomicBool flag was overkill for the same correctness guarantee; a brief false-positive `is_pushing` causes a one-tick pull delay, vs. the prior false-negative which could overwrite mid-upload.

## 44. `stop_watch` removes folder before unregistering notify
- **Where:** [auto_sync/watch.rs:122-127](../src-tauri/src/sync/auto_sync/watch.rs#L122-L127)
- **Symptom:** Between `self.folders.remove(remote_root)` and `w.unwatch(&fw.local_root)`, FS events arrive for the removed root → `queue_path` finds no owning watch + silently drops them.
- **Fix:** Reverse order — `unwatch` first, then remove from map.
> SHIPPED v0.4.14-alpha S114 (uncommitted) — `unwatch` runs first using a cloned `local_root` from the DashMap `get`, then the remove + log proceed. Closes the FS-event silent-drop window during folder teardown.

## 45. FS event drop has no counter, no escalation, not surfaced to UI
- **Where:** [auto_sync.rs:354-361](../src-tauri/src/sync/auto_sync.rs#L354-L361)
- **Symptom:** When 2048-event channel fills, each drop logs a uniform-severity `warn` + `DiagLevel::Warn` w/ no aggregation, no rate-limit, no `AutoSyncStatus` field. Sustained bursts (webpack rebuild + stalled flush) are invisible.
- **Fix:** Add `dropped_events: AtomicU64`; expose in `AutoSyncStatus`; debounced `DiagLevel::Error` after threshold.
> PARTIAL v0.4.14-alpha S114 (uncommitted) — `dropped_events: AtomicU64` added on `AutoSyncEngine`; every 100th drop now emits `DiagLevel::Error` + log::error w/ cumulative count. AutoSyncStatus exposure deferred (not consumed by FE yet; diag bus carries the signal).

## 46. `pending_dir_reconcile` coalesce flag cleared before dispatch
- **Where:** [auto_sync/watch.rs:189-193](../src-tauri/src/sync/auto_sync/watch.rs#L189-L193)
- **Symptom:** Flag stored `false` at L189 then `kick_drift_reconcile()` called at L193. A new `Create(Dir)` arriving in the gap passes the `compare_exchange` at L183 → second 500ms reconcile, double SFTP scan.
- **Fix:** Swap order — call `kick_drift_reconcile()` first, then `store(false, Release)`.
> SHIPPED v0.4.14-alpha S114 (uncommitted) — kick now precedes flag clear; the disposed-check is also clear-aware (resets flag before bail).

## 47. `apply_selected` push path has no cancel token registered
- **Where:** [auto_sync.rs:1493-1506](../src-tauri/src/sync/auto_sync.rs#L1493-L1506)
- **Symptom:** Selected-entry push calls `flush_all_now(None)`. User-clicked Cancel fires on the slot token but `flush_all_now` continues. Semaphore-based pull/delete tasks (L1428-1492) same gap.
- **Fix:** Create + register CT via `current_scan_cancel`; pass to `flush_all_now`.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — CT installed via `current_scan_cancel` mirror of `force_push_now`; `flush_all_now(Some(ct))` honors the slot. Spawned pulls/deletes/remote-deletes finish naturally (russh streams can't be aborted mid-transfer without partial files); the push side is the one that closes the issue's explicit gap.

## 48. `force_pull_now` mutex poison → silent early return, UI hangs
- **Where:** [auto_sync.rs:1543-1546](../src-tauri/src/sync/auto_sync.rs#L1543-L1546)
- **Symptom:** `last_scan_entries` mutex poisoned (prior panic) → `Err(_) => return` arm emits no log, no diagnostic, no status update. Pull-Now modal frozen waiting for a `DriftScanResult` event that never arrives.
- **Fix:** On `Err(e)`, emit `DiagLevel::Error` diagnostic + final `DriftScanResult` before returning.
> SHIPPED v0.4.14-alpha S114 (uncommitted) — poison branch now emits `DiagStage::System` Error + closing `DiagStage::DriftScanResult` Error before returning. SyncModal closes instead of hanging.

## 49. `flush_batch` count delta uses pre-circuit-breaker input counts
- **Where:** [auto_sync/flush.rs:42-43](../src-tauri/src/sync/auto_sync/flush.rs#L42-L43), [:253-254](../src-tauri/src/sync/auto_sync/flush.rs#L253-L254)
- **Symptom:** `created_count`/`delete_count` computed once at L42 from input list. After breaker drops batch or per-entry cancels/fails occur, `apply_count_delta` uses the original counts → file-count cache reflects intents not outcomes.
- **Fix:** Accumulate `delta_created`/`delta_deleted` inside dispatch loop, only on `EntryResult::Ok`.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — `process_entry` now captures `entry_kind` before move and calls `apply_count_delta(±1, 0)` / `(0, ±1)` on `EntryResult::Ok` per real outcome. Bulk apply at flush_batch end removed; unused `created_count` local dropped. `delete_count` still feeds the mass-delete circuit breaker (correct — that's an intent guard).

## 50. `process_entry` outer `biased; select!` cancels completed work
- **Where:** [auto_sync/flush.rs:299-312](../src-tauri/src/sync/auto_sync/flush.rs#L299-L312)
- **Symptom:** If `ct.cancelled()` and `work` both ready in the same poll, `biased;` always picks cancel → completed upload silently dropped + entry re-inserted into dirty + redundant re-upload next cycle.
- **Fix:** Move cancel check before `select!` (already done inside `process_entry_body`), or remove biased select.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — biased arms reordered: `r = work => r` first, `_ = ct.cancelled() => Requeued` second. When both ready in the same epoch, completed Ok wins; cancel still fires immediately on every poll cycle once work is pending.

## 51. `walk_local_rebaseline` ignore-check diverges from drift_scanner
- **Where:** [auto_sync.rs:1961-1971](../src-tauri/src/sync/auto_sync.rs#L1961-L1971)
- **Symptom:** `should_ignore` called with bare filename for dirs (not full rel-path). Path-segment rules (`.git/`, `[disabled]/`) don't fire on nested entries. Rebaseline includes files drift_scanner excludes → phantom baseline mismatch. (Cross-agent: A9+C8.)
- **Fix:** Pass forward-slash rel-path to `should_ignore`, matching `drift_scanner::walk_local`.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — bare-name check at L2030 replaced with rel-path probe (trailing slash for dirs so `/{seg}/` segment rules fire). Skips recursion into ignored dirs entirely; per-file check at L2040 unchanged. Note: drift_scanner::walk_local has the same divergence — out-of-scope for this issue, left as-is.

## 52. `apply_selected` ToDeleteRemote failure leaves snapshot stale
- **Where:** [auto_sync.rs:1457-1491](../src-tauri/src/sync/auto_sync.rs#L1457-L1491)
- **Symptom:** Success path calls `e.snapshot.forget(&entry.remote_path)`; failure path logs but doesn't forget. Next scan sees remote-absent + snapshot-present → bucket becomes `ToDelete` (local delete) instead of `Synced` → spurious "delete local" entry next cycle.
- **Fix:** Also call `snapshot.forget` in failure branch, or mark for re-scan.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — failure branch now also calls `e.snapshot.forget(&entry.remote_path)`. Idempotent-failure case (remote already gone) no longer produces a spurious `ToDelete-local` row next scan; genuine failure case (file still present) repopulates the snapshot via the next fresh remote stat.

## 53. SFTP connection leak in `scan_drift` when SyncSnapshot::new fails
- **Where:** [lib.rs:431](../src-tauri/src/lib.rs#L431)
- **Symptom:** `SyncSnapshot::new` errors propagated via `?`; `client.close()` only called at L456 happy path. Russh session leaked until OS reclaim.
- **Fix:** RAII / defer-style close, or store client and close before each early return.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — `SyncSnapshot::new` result destructured via match; error arm calls `client.close().await` before returning the formatted error. Russh session closes cleanly on snapshot-init failure (disk full, perm denied on `~/.rift/state/`, etc.).

## 54. `scan_drift` opens new SFTP session despite active engine session
- **Where:** [lib.rs:421](../src-tauri/src/lib.rs#L421)
- **Symptom:** No consult of `AutoSyncState` — always `SftpClient::connect` fresh. Two concurrent SSH sessions for same server during normal use → server `MaxSessions` pressure.
- **Fix:** Pass `AutoSyncState`; reuse engine's SFTP client when `server_key` matches.
> SHIPPED v0.4.17-alpha S120 — `scan_drift` now takes `AutoSyncState`; checks `engine.profile_key() == server_key` and reuses `engine.sftp()` + `engine.snapshot()` when matched. Cold path (no engine for that server) still connects fresh and closes after. Snapshot reuse also avoids `set` race between flush loop and ad-hoc scanner.

## 55. `resolve_conflicts_bulk` skips canonicalization vs `validate_watched_local_path`
- **Where:** [lib.rs:727](../src-tauri/src/lib.rs#L727)
- **Symptom:** `owns_local_path` called with raw `PathBuf::from(p)`. Symlink / relative-prefix paths pass `reject_path_traversal` (`..`-only check) then fail `owns_local_path` silently → pushed as `false` with no error surfaced.
- **Fix:** Canonicalize before ownership check; extract `validate_and_own` helper shared with [lib.rs:621-653](../src-tauri/src/lib.rs#L621-L653).
> SHIPPED v0.4.17-alpha S120 — new `canonicalize_owned_path(engine, raw, label)` helper factored out of `validate_watched_local_path` so bulk callers can avoid the state-lock overhead. `resolve_conflicts_bulk` uses it inline; per-row failure emits a Block activity row w/ the actual reason instead of silent `false`.

## 56. `delete_server` doesn't stop the active engine before deleting profile
- **Where:** [lib.rs:858](../src-tauri/src/lib.rs#L858)
- **Symptom:** Removes profile from disk; engine continues w/ live SFTP/watchers/locks for a server that no longer exists in config — and can never be cleanly stopped via UI.
- **Fix:** Make async; accept `AutoSyncState` + `TunnelState`; stop active engine when key matches, or return error requiring frontend disconnect first.
> SHIPPED v0.4.17-alpha S120 — `delete_server` now `async` w/ `AutoSyncState` + `TunnelState`. Engine stops before profile delete when `profile_key() == key`; tunnel teardown follows. Mirrors `stop_autosync` ordering.

## 57. `download_paths` cancellation token not cleared on SFTP connect failure
- **Where:** [lib.rs:1117-1122](../src-tauri/src/lib.rs#L1117-L1122)
- **Symptom:** CT stored at L1119-1120 before `open_sftp_for` at L1122. `?` exit on connect-fail never clears `dl_state`; `cancel_download` later fires on ghost token; next `download_paths` overwrites stale entry.
- **Fix:** Only store CT after connect succeeds; OR reset state in connect-error arm.
> SHIPPED v0.4.14-alpha S114 (uncommitted) — `open_sftp_for` moved before CT registration. Connect failure now exits cleanly without leaving a ghost token in `dl_state`.

## 58. `expand_download_jobs` silently swallows `list_recursive` errors
- **Where:** [lib.rs:1067-1070](../src-tauri/src/lib.rs#L1067-L1070)
- **Symptom:** `.unwrap_or_default()` makes failed remote listings return empty vec → frontend sees "download empty" instead of error.
- **Fix:** Return `Result<Vec<_>, String>`; or emit error `ActivityRow` per failed dir before skipping.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — signature now `Result<Vec<(String, PathBuf)>, String>`; `list_recursive` error propagates; `download_paths` caller surfaces an error `ActivityRow` + cleans up `dl_state` CT before bailing.

## 59. `download_paths` guard runs pre-expansion only
- **Where:** [lib.rs:1111-1115](../src-tauri/src/lib.rs#L1111-L1115) (guards), [:1131](../src-tauri/src/lib.rs#L1131) (expansion)
- **Symptom:** `validate_remote_child`/`validate_local_child` run on caller-supplied jobs; `expand_download_jobs` then produces additional `(remote, local)` pairs that bypass all guards. SFTP symlink targets / `..` components in `full_path` can escape `remote_root`.
- **Fix:** Validate each expanded pair inside expansion, or re-validate post-expansion before `download_files_batch`.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — post-expansion re-validation loop runs `validate_remote_child` + `validate_local_child` on every expanded pair before `download_files_batch`; cleans up the `dl_state` CT slot on early-return.

## 60. `upload_paths` guard runs pre-expansion only (mirrors #59)
- **Where:** [lib.rs:1017-1022](../src-tauri/src/lib.rs#L1017-L1022) (guards), [:1024](../src-tauri/src/lib.rs#L1024) (expansion)
- **Symptom:** Same gap as #59 on the upload side. Local symlink inside watched dir yields paths outside watched root.
- **Fix:** Validate each `(local, remote_target)` pair inside walkdir loop or post-expansion.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — post-expansion re-validation loop on every `(local, remote_target)` pair from walkdir; closes SFTP/local symlink escape on the upload path.

## 61. `probe_server_fingerprint` issues write probe to unverified host
- **Where:** [lib.rs:1535-1542](../src-tauri/src/lib.rs#L1535-L1542)
- **Symptom:** TOFU probe correctly passes `trusted_fingerprint: None` but also `write_probe_root: Some(&server.remote_root)` — performs a filesystem write to probe write-access BEFORE the fingerprint is confirmed. Contradicts the TOFU rationale at L1521-1526.
- **Fix:** Pass `write_probe_root: None`; write-permission check only after fingerprint confirmed + server trusted.
> SHIPPED v0.4.14-alpha S114 (uncommitted) — `write_probe_root: None` in the TOFU probe call. Write-access verification (if needed) is now strictly post-trust.

## 62. Bridge token leaked into MCP child env regardless of remote-shell toggle
- **Where:** [assistant/mod.rs:527-533](../src-tauri/src/assistant/mod.rs#L527-L533)
- **Symptom:** `RIFT_BRIDGE_PORT`+`RIFT_BRIDGE_TOKEN` always injected (for `sync_status`). A compromised MCP tool can use the token to call any bridge endpoint, not just sync_status. Authorization scope conflated.
- **Fix:** Issue two scoped tokens — read-only `RIFT_BRIDGE_READONLY_TOKEN` always; write-capable `RIFT_BRIDGE_TOKEN` only when `remote_shell_enabled`.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — `BridgeInfo` now carries `token` (write) + `readonly_token`; `handle_conn` infers `Scope::Write`/`Scope::ReadOnly` from which presented; `dispatch` rejects `remote_bash` on read-only scope. `write_mcp_config` injects `RIFT_BRIDGE_READONLY_TOKEN` always, `RIFT_BRIDGE_TOKEN` only when `remote_shell_enabled`. `mcp_server::tool_sync_status` prefers readonly w/ fall-back to write; `bridge_enabled()` accepts either.

## 63. `SESSION_PIDS`/`SESSION_STOPPED` mutex poison silently swallowed → orphaned children
- **Where:** [assistant/mod.rs:43-55](../src-tauri/src/assistant/mod.rs#L43-L55)
- **Symptom:** `with_session_pids`/`with_session_stopped` use `.lock().ok()` → `None` on poison. All callers (`set_session_pid`, `clear_session_pid`, `get_session_pid`) silently no-op → `assistant_stop` fails to kill, child orphans.
- **Fix:** Recover via `e.into_inner()` or surface explicit error; add child-orphan kill in spawn-task drop.
> PARTIAL v0.4.14-alpha S114 (uncommitted) — both helpers now `into_inner()` on poison + log::error. PID/stop tracking continues working after a poisoning panic instead of silently no-op'ing. Child-orphan kill on spawn-task drop deferred (needs RAII guard on Child handles).

## 64. `CLAUDE_EXE` cached forever via OnceLock — stale after CLI install/update
- **Where:** [assistant/mod.rs:84-165](../src-tauri/src/assistant/mod.rs#L84-L165)
- **Symptom:** `OnceLock<Option<PathBuf>>` init once per process. New CLI install or path change requires full Rift restart. (Cross-agent: F5+G5.)
- **Fix:** Replace w/ `RwLock<Option<PathBuf>>` w/ TTL, or expose `assistant_reload_cli_path` Tauri command.
> SHIPPED v0.4.14-alpha S114 (uncommitted) — `OnceLock<Option<PathBuf>>` → `Mutex<Option<Option<PathBuf>>>`. Fast path stats cached `is_file()`; missing file triggers fresh resolution. CLI upgrades take effect on next spawn, no restart required. Body extracted to `resolve_claude_exe_uncached` helper.

## 65. `save_config` non-atomic — lost updates on concurrent commands
- **Where:** [assistant/mod.rs:490-494](../src-tauri/src/assistant/mod.rs#L490-L494)
- **Symptom:** Direct `std::fs::write` (no temp-then-rename). Two Tauri-command setters racing on read-modify-write produce a torn/empty config.json. `assistant_save_conversation` at L459-461 already has the correct tmp+rename pattern. (Cross-agent: F6+G4.)
- **Fix:** Apply same `.tmp` + `std::fs::rename` pattern, OR serialize through `Mutex<AssistantConfig>` in managed state.
> SHIPPED v0.4.14-alpha S114 (uncommitted) — tmp+rename pattern matches `assistant_save_conversation`. Concurrent setters now produce either old or new content, never torn.

## 66. Unbounded stderr buffer — OOM on wedged CLI
- **Where:** [assistant/mod.rs:1392-1400](../src-tauri/src/assistant/mod.rs#L1392-L1400)
- **Symptom:** `stderr_task` appends every line to a `String buf` with no cap. Long-running erroring session grows heap until OOM.
- **Fix:** Cap at 64 KiB; drop or truncate older lines, preserve tail for error surfacing.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — buf capped at 64 KiB; on overflow drains the first 32 KiB on a line boundary so the tail (where fatal/panic lines land) is preserved. Truncation prepends a `[... earlier stderr dropped (>64 KiB) ...]\n` marker so the surfaced error is honest.

## 67. `child.id()` None branch silently skips PID registration
- **Where:** [assistant/mod.rs:1315-1317](../src-tauri/src/assistant/mod.rs#L1315-L1317)
- **Symptom:** If process already exited by time `id()` is called (immediate-exit on bad args), `set_session_pid` never called → `assistant_stop` returns Ok but child kept running until natural exit.
- **Fix:** Log warning when `child.id()` is None; consider treating missing PID in `assistant_stop` as no-op vs false-Ok.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — `log::warn!` on the None branch with session_id context. The orphan-or-instant-exit case is now diagnosable from the log stream instead of being silently invisible.

## 68. MCP parse_error has no JSON-RPC error response back
- **Where:** [assistant/mcp_server.rs:650-653](../src-tauri/src/assistant/mcp_server.rs#L650-L653)
- **Symptom:** Malformed JSON silently `continue`s. MCP 2025-03-26 spec requires `-32700 Parse error` response when id is derivable; current code doesn't try.
- **Fix:** Attempt to parse minimal `{"id": ...}` before discarding; if id found, send `-32700`.
> SHIPPED v0.4.17-alpha S120 — `IdOnly` probe parses minimal `{id}` on parse-fail; emits `-32700` JSON-RPC response when id derivable. Malformed payloads without id still drop silently (no addressable id to reply to).

## 69. MCP `handle_conn` unauthorized write may not flush before drop
- **Where:** [assistant/remote_bridge.rs:144-147](../src-tauri/src/assistant/remote_bridge.rs#L144-L147)
- **Symptom:** `let _ = write_line(...)` then `Err("unauthorized")` returns; `TcpStream` drop on Windows may not flush before close → MCP child sees connection-reset, never reads "unauthorized".
- **Fix:** Replace `let _ =` w/ `await?` then `write_half.shutdown().await.ok()` before returning error.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — `write_line(...)?` propagates write errors; `write_half.shutdown().await` orderly-closes before the function returns. Folded into #62's scope-based gate.

## 70. MCP `run_stdio` silently discards response serialization errors
- **Where:** [assistant/mcp_server.rs:656-659](../src-tauri/src/assistant/mcp_server.rs#L656-L659)
- **Symptom:** `serde_json::to_string(&r)` fail → `continue`. MCP client hangs waiting for the response that never arrives.
- **Fix:** On serialize fail, `return` from `run_stdio` or write to stderr + `break`. Don't continue.
> SHIPPED v0.4.17-alpha S120 — serialize-fail arm now `return`s instead of `continue`. MCP child exits cleanly, parent observes disconnect (vs hanging waiting for the lost response).

## 71. MCP `tool_grep` reads whole file before 8KB binary check
- **Where:** [assistant/mcp_server.rs:253-258](../src-tauri/src/assistant/mcp_server.rs#L253-L258)
- **Symptom:** Comment says "read first 8 KB" but implementation reads the whole file then `take(8192)`. Up to 5000 full-file reads per scan → memory pressure on stdio process.
- **Fix:** `File::open` + `Read::take(8192)` for the NUL probe; re-open for the full UTF-8 regex pass.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — separate streaming probe (`File::open` + `take(8192).read_to_end`) runs before the full read. Binary files bail at 8 KiB without loading the whole file. Full `std::fs::read` only runs on files that pass the NUL probe.

## 72. MCP `sync_status` listed unconditionally (env-strip case)
- **Where:** [assistant/mcp_server.rs:551-561](../src-tauri/src/assistant/mcp_server.rs#L551-L561), [:600](../src-tauri/src/assistant/mcp_server.rs#L600)
- **Symptom:** Tools list shows `sync_status` while bridge env vars set, but call path at L600 invokes `tool_sync_status` unconditionally; internal check exists but is redundant. Env-stripped MCP launchers cause silent disappearance.
- **Fix:** Guard call-path w/ `bridge_enabled()` same as list-path; add comment making env-static assumption explicit.
> SHIPPED v0.4.17-alpha S120 — `tools/call` match arms now use `"sync_status" if bridge_enabled()` + `"remote_bash" if remote_shell_enabled()`. Env-stripped MCP launchers see "unknown tool" (callable + listable parity).

## 73. Drift scanner cancel race — folder loop continues past cancel signal
- **Where:** [drift_scanner.rs:138](../src-tauri/src/sync/drift_scanner.rs#L138)
- **Symptom:** Cancel within the batch-listing `select!` window doesn't mark `cancelled` for the per-folder phase. `scan_folder` already started will complete fully even if user cancelled.
- **Fix:** Check cancel before each `scan_folder`; propagate CT into `scan_folder` for SFTP-hash interruption.
> SHIPPED v0.4.17-alpha S120 — `scan_folder` signature now takes `cancel: Option<&CancellationToken>`. Pre-iteration check inside the all_keys loop short-circuits the per-entry hash budget so an in-flight folder honors Cancel between SFTP hash calls. Outer pre-folder check already in place.

## 74. `walk_local` panic → empty map → mass `ToPull` (data loss path)

> **SHIPPED v0.4.14-alpha S113 (uncommitted)** — `drift_scanner.rs:228-260` matches `JoinError` on the `spawn_blocking` walk. On panic, emits a `DriftScanProgress` diag at Error level and returns `FolderScan::SuspiciousEmptyAborted { baseline_count, listing_count: 0 }` — the existing safe-abort path. The data-safety guard at L241 (`!local_map.is_empty()`) is no longer bypassable via empty-default fallback.
- **Where:** [drift_scanner.rs:228](../src-tauri/src/sync/drift_scanner.rs#L228)
- **Symptom:** `spawn_blocking(walk).await.unwrap_or_default()` swallows JoinError-panic. Scanner sees "remote-only" for every file, queues all as `ToPull` → possibly overwrites locally-newer files.
- **Fix:** Match Result; log warning on Err; return `FolderScan::SuspiciousEmptyAborted` rather than empty.

## 75. First-scan mtime tie-break can mis-classify identical files as `ToPush`
- **Where:** [drift_scanner.rs:464](../src-tauri/src/sync/drift_scanner.rs#L464)
- **Symptom:** No baseline + sizes match + hash budget exhausted → mtime equality falls through to `ToPush` (local wins). A re-extracted/rsync'd copy has identical mtime → arbitrary classification.
- **Fix:** When sizes match + budget zero, bucket as `Conflict` w/ explicit reason; or don't deduct budget for the opportunistic-equality path.
> SHIPPED v0.4.17-alpha S120 — size-equal + mtime-mismatch + content-unverified now buckets as `Conflict` w/ reason "sizes match but mtimes diverged (content unverified)" instead of arbitrary mtime-newer-wins. Re-extracted / rsync'd identical copies no longer silently push or pull.

## 76. `delete_local_one` empty-dir cleanup can walk above resource root
- **Where:** [drift_watcher.rs:293](../src-tauri/src/sync/drift_watcher.rs#L293)
- **Symptom:** `while remove_dir(&cur).is_ok()` walks parents upward with no floor check. On a single-file resource, will attempt remove on profile local_root and beyond.
- **Fix:** Pass resource's `local_root` as floor; break when `cur == floor`.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — looks up the matching `FolderWatch` via `engine.folders_clone()` by `resource_name`, captures `local_root` as floor. Loop breaks if `cur == floor` or escapes via `!starts_with(floor)` before calling `remove_dir`. Single-file resources can no longer trigger an upward sweep past the resource root.

## 77. `acquire` TOCTOU — `my_locks` inserted before SFTP upload confirmed
- **Where:** [lock_presence.rs:167](../src-tauri/src/sync/lock_presence.rs#L167)
- **Symptom:** `my_locks.insert` before `sftp.upload_bytes`. `poll_once` skips entries in `my_locks` (L313); if two concurrent `acquire()` race on same path, second returns "already own" but first then fails → lock removed from `my_locks` but never on remote.
- **Fix:** Insert into `my_locks` only after `upload_bytes` Ok; use separate `pending_lock` set for in-flight reservation.
> SHIPPED v0.4.17-alpha S120 — added `pending_locks: DashSet<String>` for in-flight reservation. `acquire()` first checks `my_locks.contains` (confirmed-owned short-circuit), then `pending_locks.insert` (race-loser bails). `my_locks.insert` happens only on upload success. Pending set is cleared on both paths.

## 78. FiveM bypass requires trailing slash — bare `web/build` dir mis-ignored
- **Where:** [sync/ignore.rs:197](../src-tauri/src/sync/ignore.rs#L197)
- **Symptom:** `is_fivem_ui_output` substring match requires `/web/build/`. A listing entry for the bare dir `res/qbx_core/web/build` (no trailing slash) fails the predicate → directory itself reported as ignored, even though its children are correctly bypassed.
- **Fix:** Add `|| lower.ends_with("/web/build")`; or normalize trailing slash on dirs before check.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — `is_fivem_ui_output` now also matches `ends_with("/web/build")` + `ends_with("/web/dist")`. Bare-dir listings no longer get misreported as ignored.

## 79. Nested-negation in `classify` segment match — fragile, Clippy-flagged
- **Where:** [sync/ignore.rs:205](../src-tauri/src/sync/ignore.rs#L205)
- **Symptom:** `if !A { if !B { continue } }` is logically correct (skip when both fail) but reads as "skip when match", one mutation away from inversion bug. Clippy flags `redundant_else`.
- **Fix:** Rewrite as positive-flow guard `if lower.contains(needle) || lower.starts_with(leading) { ... }`.
> SHIPPED v0.4.17-alpha S120 — collapsed nested `!A { !B { continue } }` into a single positive-flow `if !lower.contains(&needle) && !lower.starts_with(&leading) { continue; }`. Equivalent logic, harder to invert by accident.

## 80. `atomic_write_json` temp file collides on concurrent saves of same snapshot
- **Where:** [state/paths.rs:53](../src-tauri/src/state/paths.rs#L53)
- **Symptom:** `path.with_extension("json.tmp")` is deterministic. `SyncSnapshot::set` (flush loop) and `replace_under` (rebaseline) both call `save_locked` → same `snapshot-<key>.json.tmp` truncates the other's in-flight write.
- **Fix:** Append `{pid}` or random suffix; or use `tempfile::NamedTempFile` in same dir.
> SHIPPED v0.4.17-alpha S120 — tmp name now `<basename>.<pid>-<counter>.json.tmp` (`AtomicU64` counter, per-call). `set` flush loop and `replace_under` rebaseline can no longer truncate each other's tmp. Rename target still canonical.

## 81. `SyncSnapshot::set`/`forget` silently discard save errors
- **Where:** [state/sync_snapshot.rs:74](../src-tauri/src/state/sync_snapshot.rs#L74), [:80](../src-tauri/src/state/sync_snapshot.rs#L80)
- **Symptom:** Both methods `let _ = self.save_locked(&g)` — disk-write fail silently leaves in-memory state diverged from on-disk. Next restart loads stale data → phantom drift / false ToDelete/ToPull. `replace_under` at L125 correctly propagates.
- **Fix:** Change signatures to `-> std::io::Result<()>`; propagate via `?`; callers surface via DiagBus.
> PARTIAL v0.4.16-alpha S119 (uncommitted) — `set` + `forget` now match the save_locked Result; failures emit `log::error!` with remote_path context. Signatures kept `-> ()` to avoid touching every caller (hot path: flush). Full `Result<(), io::Error>` propagation + DiagBus surface deferred — log is enough to diagnose; the silent-divergence case is closed.

## 82. `heal_owned_dirs` breaks on ExitStatus — v0.2.44 truncation bug not fixed here
- **Where:** [sftp/remote_exec.rs:107](../src-tauri/src/sftp/remote_exec.rs#L107)
- **Symptom:** Channel-drain loop breaks on `ChannelMsg::ExitStatus`, missing trailing Data frames. Same bug pattern fixed elsewhere (`list_via_exec`, `get_remote_sha1`, `exec_bash`) but missed in `heal_owned_dirs`.
- **Fix:** Replace `ExitStatus { .. } => break` with `_ => {}`; loop exits only when `wait()` returns `None`.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — drain loop replaced with `while chan.wait().await.is_some() {}` (we don't consume stdout here). Aligns with the v0.2.44 channel-close convention.

## 83. Worker SSH handles not explicitly closed on `SftpClient::close()`
- **Where:** [sftp/mod.rs:281-288](../src-tauri/src/sftp/mod.rs#L281-L288)
- **Symptom:** `close()` iterates `w.sftp.close()` but never drops/clears `Worker.handle`. Held `Arc<Worker>` clones (from batch ops) keep SSH session alive past intended close. Up to 4 leaked sessions per `SftpClient`.
- **Fix:** After SFTP-close loop, clear `self.workers` to release Arc references; or explicit `Arc::try_unwrap` per worker.
> SHIPPED v0.4.17-alpha S120 — `close()` drains the `Mutex<Vec<Arc<Worker>>>` via `std::mem::take` instead of cloning. Canonical store releases strong refs immediately; only in-flight batch ops keep workers alive past close (cooperative shutdown w/ those is a separate concern).

## 84. `rename_via` TOCTOU — exists-check + rename non-atomic
- **Where:** [sftp/ops.rs:158-165](../src-tauri/src/sftp/ops.rs#L158-L165)
- **Symptom:** SFTP has no conditional-rename primitive; check + rename has inherent race. Lock-presence write-back uses this and may silently overwrite a concurrent writer.
- **Fix:** Document explicitly; for authoritative-overwrite paths prefer `rename_overwriting_via` w/ explicit intent.
> SHIPPED v0.4.17-alpha S120 — doc-comment added above `rename_via` calling out the known TOCTOU and pointing at `rename_overwriting_via` for intent-clarity. No behavior change.

## 85. `list_recursive_batch` belt-and-braces retry has no timeout
- **Where:** [sftp/list.rs:179-193](../src-tauri/src/sftp/list.rs#L179-L193)
- **Symptom:** Worker paths wrap calls in `tokio::time::timeout(LIST_T, ...)`; the main-session retry at L181 calls `list_recursive_via` unwrapped. Wedged main session hangs indefinitely.
- **Fix:** Wrap the retry call in `tokio::time::timeout(LIST_T, ...)` matching worker paths.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — retry future now wrapped in `tokio::time::timeout(LIST_T, ...)`; timeout + inner Err both fall through to the empty-entry insert. Wedged main-session retries surface as empty-listing instead of hanging the batch.

## 86. `delete_recursive_via` has no per-op timeouts
- **Where:** [sftp/ops.rs:98-154](../src-tauri/src/sftp/ops.rs#L98-L154)
- **Symptom:** `symlink_metadata`/`read_dir`/`remove_file`/`remove_dir` all uncapped. A wedged session during multi-file delete blocks the entire delete-drift path.
- **Fix:** Wrap each op in `with_t(T_QUICK, ...)` / `with_t(T_NORMAL, ...)` matching transfer.rs discipline.
> SHIPPED v0.4.17-alpha S120 — added file-local `ops_with_t` + `OPS_T_QUICK` / `OPS_T_NORMAL` constants (to avoid promoting transfer.rs's helpers cross-module). Each SFTP op in `delete_recursive_via` now timeout-bounded.

## 87. `upload_bytes` write timeout leaks SFTP file handle
- **Where:** [sftp/transfer.rs:215-216](../src-tauri/src/sftp/transfer.rs#L215-L216)
- **Symptom:** `with_t(T_BODY, "write")` timeout returns `Err`; `f` dropped without explicit `shutdown()`. Server-side handle may not flush, write uncommitted. `upload_atomic_via` uses explicit scoped block (correct).
- **Fix:** Wrap create+write+shutdown in explicit scope, OR `let _ = f.shutdown().await` in error branch.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — write-Err branch now calls `with_t(T_QUICK, "close-on-err", ...)` on `f.shutdown()` before returning the error. Server-side handle closes promptly even on the timeout/IO-error path.

## 88. `exec_bash` channel not closed on timeout — server process leak
- **Where:** [sftp/remote_exec.rs:73-80](../src-tauri/src/sftp/remote_exec.rs#L73-L80)
- **Symptom:** Timeout drops the drain future, dropping `chan` without orderly close. Server-side process keeps running until session teardown — long `find` / scripts continue consuming server CPU.
- **Fix:** On timeout, `chan.eof().await` or send `ChannelMsg::Eof` before returning error.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — timeout branch now sends `chan.eof().await` (best-effort) before returning the timeout error. Remote process gets an orderly close signal instead of running until session reclaim.

## 89. `download_file` buffers entire remote file into memory
- **Where:** [sftp/transfer.rs:231](../src-tauri/src/sftp/transfer.rs#L231), [:337](../src-tauri/src/sftp/transfer.rs#L337)
- **Symptom:** `sftp.read(remote_path)` loads full bytes into `Vec<u8>` before writing local. Hundreds-of-MB asset files (FiveM map packs, .ytd) OOM on low-RAM servers / mobile WiFi.
- **Fix:** For files >16 MB, `sftp.open` + stream chunks to local tmp via `AsyncRead`. Deferred-complexity.

## 90. `shell_quote` allows tab — poisons `find -printf` parser
- **Where:** [sftp/remote_exec.rs:151-153](../src-tauri/src/sftp/remote_exec.rs#L151-L153)
- **Symptom:** Tab in remote filename passes validation, embeds in `find -printf '%p\t%s\t%T@\n'` output, breaks `splitn(3, '\t')` parser → `skipped_bad_size` + silently dropped file.
- **Fix:** Add `'\t'` to rejected charset: `if s.contains(['\0', '\n', '\r', '\t'])`.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — `\t` added to the reject set in `shell_quote`. Paths with embedded tabs now fail validation up-front instead of silently breaking the printf parser downstream.

### LOW (28)

## 91. `enqueue_for_flush_batch` declared `async` with no `.await`
- **Where:** [auto_sync.rs:916](../src-tauri/src/sync/auto_sync.rs#L916) — body L916-967 all sync DashMap ops.
- **Fix:** Drop `async`; update [lib.rs:675](../src-tauri/src/lib.rs#L675) caller.
> SHIPPED v0.4.17-alpha S120 — `async` dropped; Tauri-cmd caller in lib.rs + internal caller in `resolve_conflict::ForceLocal` arm both updated to drop `.await`.

## 92. `ActivityRow::default()` uses deprecated chrono associated-fn form
- **Where:** [auto_sync.rs:124](../src-tauri/src/sync/auto_sync.rs#L124) — `DateTime::<Utc>::from_timestamp(0,0).unwrap_or_else(Utc::now)`.
- **Fix:** Use `DateTime::UNIX_EPOCH` or free-fn `chrono::DateTime::from_timestamp(0,0).unwrap()`.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — `at: DateTime::UNIX_EPOCH` constant.

## 93. `suppress_local_delete_uploads` window is 2s — too short for slow SFTP
- **Where:** [auto_sync.rs:888](../src-tauri/src/sync/auto_sync.rs#L888) — hardcoded `chrono::Duration::seconds(2)`; debounce ceiling is `CEILING_MS=3000`. (Cross-agent: A10+C9.)
- **Fix:** Raise to ≥5s (match `recently_written` window at [watch.rs:211](../src-tauri/src/sync/auto_sync/watch.rs#L211)), or extract `SUPPRESS_WINDOW_SECS=5` const.

## 94. `resolve_conflict AcceptRemote` drops conflict row on download failure
- **Where:** [auto_sync.rs:1006-1017](../src-tauri/src/sync/auto_sync.rs#L1006-L1017)
- **Fix:** Re-insert conflict via `self.conflicts.insert(...)` on download fail before returning, matching `SaveLocalCopy` bail.

## 95. `resolve_conflict ForceLocal` enqueues but never triggers flush
- **Where:** [auto_sync.rs:1055-1060](../src-tauri/src/sync/auto_sync.rs#L1055-L1060)
- **Fix:** After `enqueue_for_flush_batch`, call `kick_drift_reconcile` or trigger flush.

## 96. `apply_selected` guard-override emits ActivityRow direct (bypasses buffered feed)
- **Where:** [auto_sync.rs:1413-1424](../src-tauri/src/sync/auto_sync.rs#L1413-L1424), [:1656-1666](../src-tauri/src/sync/auto_sync.rs#L1656-L1666)
- **Fix:** Use `engine.log_activity(...)` (buffered) not direct `engine.app().emit(...)`.

## 97. `rebaseline_folder` blocking walk ignores cancel + disposal
- **Where:** [auto_sync.rs:1276-1285](../src-tauri/src/sync/auto_sync.rs#L1276-L1285)
- **Fix:** Check `engine.disposed.load(SeqCst)` before `spawn_blocking`; pass AtomicBool cancel into walk.

## 98. `process_entry_body` fabricates `(0, Utc::now())` ConflictRecord on vanished file
- **Where:** [auto_sync/flush.rs:514-515](../src-tauri/src/sync/auto_sync/flush.rs#L514-L515)
- **Fix:** Check `stat_local` return; log "file vanished mid-preflight" + return `EntryResult::Fail`, don't insert synthetic conflict.

## 99. `flush_batch dispatched` count includes Requeued — `force_push_now` clears scan cache wrongly
- **Where:** [auto_sync/flush.rs:140-154](../src-tauri/src/sync/auto_sync/flush.rs#L140-L154), [auto_sync.rs:824](../src-tauri/src/sync/auto_sync.rs#L824)
- **Fix:** Return `(dispatched, ok, fail)` tuple; gate cache-clear on `ok > 0`.

## 100. Double lock release on successful upload (idempotent but wasteful)
- **Where:** [auto_sync/flush.rs:588-591](../src-tauri/src/sync/auto_sync/flush.rs#L588-L591) + [:317-325](../src-tauri/src/sync/auto_sync/flush.rs#L317-L325)
- **Fix:** Remove early-release inside `process_entry_body`; rely on `process_entry` cleanup.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — early-release `tokio::spawn` block in `process_entry_body` success branch dropped. Sole release path is now `process_entry`'s post-result inline await — also covers Fail branch.

## 101. `recently_written` map grows unbounded on never-re-queried entries
- **Where:** [auto_sync/watch.rs:210-220](../src-tauri/src/sync/auto_sync/watch.rs#L210-L220)
- **Fix:** Add periodic sweep on existing 5s root-vanish poll, or use bounded LRU (~1024).

## 102. `force_pull_now` clears scan cache before UI reads `ToDeleteRemote` entries
- **Where:** [auto_sync.rs:1617](../src-tauri/src/sync/auto_sync.rs#L1617), [:1734](../src-tauri/src/sync/auto_sync.rs#L1734)
- **Fix:** Don't clear cache for ToDeleteRemote; only clear dispatched (ToPull + ToDelete).

## 103. Mass-delete breaker block-path keeps blocked count in `to_delete` total
- **Where:** [auto_sync.rs:1631-1671](../src-tauri/src/sync/auto_sync.rs#L1631-L1671), [:1754](../src-tauri/src/sync/auto_sync.rs#L1754)
- **Fix:** Track `to_delete_blocked` separately; subtract before diag emit, or add `blocked_local_deletes` field.

## 104. `eprintln!` debug noise in sync command handlers (production)
- **Where:** [lib.rs:132,134,136,165,167,169,183,185,187,190,192](../src-tauri/src/lib.rs#L132)
- **Fix:** Convert to `log::debug!` or remove.
> SHIPPED v0.4.16-alpha S119 (uncommitted) — 11 sites in `sync_reconcile` / `sync_pull_pending` / `sync_push_pending` converted to `log::debug!`. Stderr no longer carries `[rift] sync_*` chatter in production.

## 105. `sync_set_mirror_mode` set/read-back TOCTOU
- **Where:** [lib.rs:299-300](../src-tauri/src/lib.rs#L299)
- **Fix:** `set_mirror_mode` returns new value, eliminate the read-back call.

## 106. `diag_state_pump` infinite loop has no cancellation path
- **Where:** [lib.rs:343](../src-tauri/src/lib.rs#L343)
- **Fix:** Pass CancellationToken; `tokio::select!` between tick and cancel.

## 107. `start_autosync` status sampled before prev engine fully stopped
- **Where:** [lib.rs:572-576](../src-tauri/src/lib.rs#L572-L576)
- **Fix:** Drop state lock first, then sample new engine status.

## 108. `diag_state_pump` duplicates `diag_get_state` DTO assembly
- **Where:** [lib.rs:355-385](../src-tauri/src/lib.rs#L355-L385) ≈ [lib.rs:83-113](../src-tauri/src/lib.rs#L83-L113)
- **Fix:** Extract shared `collect_diag_dto(engine: Option<Arc<AutoSyncEngine>>) -> DiagStateDto`.

## 109. `bootstrap_list_files` accepts dead `_local_root` IPC param
- **Where:** [lib.rs:1443](../src-tauri/src/lib.rs#L1443)
- **Fix:** Remove param from signature + update FE caller, OR restore use w/ validation guard.

## 110. `bootstrap::classify` skips `BadRemoteRoot` check for `remote_count < 3`
- **Where:** [bootstrap/mod.rs:46-60](../src-tauri/src/bootstrap/mod.rs#L46-L60)
- **Fix:** Lower `BAD_REMOTE_ROOT_MIN_DIRS` to 1, OR add fallback: `if remote_count < 3 && bracketed_count == 0 → BadRemoteRoot`.

## 111. `BadRemoteRoot` branch reports `missing_count: 0` (misleads FE)
- **Where:** [bootstrap/mod.rs:53-59](../src-tauri/src/bootstrap/mod.rs#L53-L59)
- **Fix:** Set `missing_count: remote_count as u32` OR document state-conditional semantics.

## 112. `remote_list_dir` double-loads `RiftConfig`
- **Where:** [lib.rs:957](../src-tauri/src/lib.rs#L957) + [:929](../src-tauri/src/lib.rs#L929) (inside `open_sftp_for`)
- **Fix:** Optional `&ServerConfig` param to `open_sftp_for` so callers can pass through.

## 113. `editor_for` race-loss drops SFTP client without explicit close
- **Where:** [lib.rs:1673-1680](../src-tauri/src/lib.rs#L1673-L1680)
- **Fix:** In race-loss branch, spawn `c.close().await` on the losing handle before returning.

## 114. `assistant_delete_conversation` orphans `cli_session_id` cwd sidecar
- **Where:** [assistant/mod.rs:466-473](../src-tauri/src/assistant/mod.rs#L466-L473)
- **Symptom:** Calls `delete_session_cwd(&id)` w/ Rift convo id only; since S103, `Conversation.cli_session_id` is a separate UUID — sidecar under that UUID never cleaned. (Cross-agent: F7+G7.)
- **Fix:** Load convo first; call `delete_session_cwd` for both `convo.id` AND `convo.cli_session_id`. Also move call to after `remove_file` success.

## 115. `session-lost` event re-broadcasts full prompt over Tauri bus
- **Where:** [assistant/mod.rs:1439-1446](../src-tauri/src/assistant/mod.rs#L1439-L1446)
- **Fix:** Emit only `{ session_id }` recovery signal; FE re-sends from its own buffered last message.

## 116. Attachment 20MiB cap uses approximate base64 estimate `len*3/4`
- **Where:** [assistant/mod.rs:1151](../src-tauri/src/assistant/mod.rs#L1151)
- **Fix:** Use `base64::decoded_len_estimate` or strip whitespace before `(len/4)*3`.

## 117. `stdin.take() == None` branch hangs child forever
- **Where:** [assistant/mod.rs:1323](../src-tauri/src/assistant/mod.rs#L1323), wait at [:1402](../src-tauri/src/assistant/mod.rs#L1402)
- **Fix:** Return `Err("claude stdin unavailable")` + kill child; OR `tokio::time::timeout` around `child.wait()`.

## 118. MCP `OnceLock::unwrap()` after race-loss in Result-returning fn
- **Where:** [assistant/remote_bridge.rs:100](../src-tauri/src/assistant/remote_bridge.rs#L100)
- **Fix:** `BRIDGE.get().cloned().ok_or_else(|| "...".into())?`.

### LOW continued + INFO (folded)

## 119. `tool_remote_bash` read timeout has no total deadline; single-thread MCP blocks
- **Where:** [assistant/mcp_server.rs:410-411](../src-tauri/src/assistant/mcp_server.rs#L410-L411)
- **Fix:** Background-thread + channel w/ hard deadline, or `tokio::time::timeout`; OR document single-thread limit.

## 120. `glob_to_regex` `*.rs` matches per path segment only — confuses users
- **Where:** [assistant/mcp_server.rs:485-507](../src-tauri/src/assistant/mcp_server.rs#L485-L507)
- **Fix:** When glob has no `/`, strip relpath to filename component before matching; OR document `**/*.rs` requirement.

## 121. `poll_once` stale-lock delete ignores `stale_delete_fails` cap
- **Where:** [lock_presence.rs:323](../src-tauri/src/sync/lock_presence.rs#L323) (vs [:217](../src-tauri/src/sync/lock_presence.rs#L217) which has cap)
- **Fix:** Apply same `STALE_DELETE_MAX_FAILS` counter in `poll_once`, or extract shared helper.

## 122. `try_read_lock` leaks temp scratch dir on parse failure
- **Where:** [lock_presence.rs:352](../src-tauri/src/sync/lock_presence.rs#L352)
- **Fix:** `remove_dir_all(&scratch)` at end, OR `tempfile::TempDir` RAII guard.

## 123. `lock_presence::stop` cleanup task not aborted on timeout
- **Where:** [lock_presence.rs:135](../src-tauri/src/sync/lock_presence.rs#L135)
- **Fix:** Call `cleanup.abort()` on `Err(Elapsed)`.

## 124. `register_conflict` uses stale scan-time mtimes vs disk
- **Where:** [drift_watcher.rs:340](../src-tauri/src/sync/drift_watcher.rs#L340)
- **Fix:** Re-stat local before building `ConflictRecord`, matching `pull_one` at L129.

## 125. `RemoteStateCache::save` re-locks for clone outside guard
- **Where:** [state/remote_state.rs:63](../src-tauri/src/state/remote_state.rs#L63)
- **Fix:** Accept `&HashMap<...>` like `save_locked` does; call inside `set`/`forget` while guard held.

## 126. `safe_profile_key` silently strips dots — collision on `foo` vs `foo.v2`
- **Where:** [state/paths.rs:30](../src-tauri/src/state/paths.rs#L30)
- **Fix:** Add `.` to allowed set OR `log::warn!` on cleaned-key mismatch.

## 127. `compute_sha1` blocking I/O on async executor
- **Where:** [state/sync_snapshot.rs:141](../src-tauri/src/state/sync_snapshot.rs#L141), called inline from async at [drift_watcher.rs:141](../src-tauri/src/sync/drift_watcher.rs#L141) + [auto_sync.rs:1803](../src-tauri/src/sync/auto_sync.rs#L1803)
- **Fix:** Wrap callers in `tokio::task::spawn_blocking`, or mark fn `blocking` w/ doc.

## 128. `atomic_write_json` orphans `.tmp` on write/sync failure
- **Where:** [state/paths.rs:59](../src-tauri/src/state/paths.rs#L59), cleanup only at [:83](../src-tauri/src/state/paths.rs#L83) rename-retry path.
- **Fix:** Closure-wrap inner write block w/ cleanup-on-fail, or `tempfile::NamedTempFile`.

## 129. `upload_bytes` missing SETSTAT 0664 (permission gap for non-atomic callers)
- **Where:** [sftp/transfer.rs:214-216](../src-tauri/src/sftp/transfer.rs#L214-L216) (vs `upload_atomic_via` at [:322-327](../src-tauri/src/sftp/transfer.rs#L322-L327))
- **Fix:** Append `set_metadata` call after `f.shutdown()`, OR doc that `upload_bytes` is internal-only (probe ephemeral).

## 130. Exec fast-path errors silently dropped — no degradation visibility
- **Where:** [sftp/list.rs:80-88](../src-tauri/src/sftp/list.rs#L80-L88)
- **Fix:** Match Err arm w/ `log::debug!("list exec fast-path failed for {root}, falling back to sftp: {e}")`.

## 131. `SftpClient` has no `Drop` impl — workers leak on panic unwind
- **Where:** [sftp/mod.rs:114-121](../src-tauri/src/sftp/mod.rs#L114-L121) (vs `SshTunnel` Drop at [tunnel/mod.rs:195-205](../src-tauri/src/tunnel/mod.rs#L195-L205))
- **Fix:** Add `Drop` that clears `self.workers` synchronously; log if `close()` was not called.

## 132. `convo_path`/`session_cwd_path` no length cap on `id`
- **Where:** [assistant/mod.rs:314-318](../src-tauri/src/assistant/mod.rs#L314-L318), [:331-337](../src-tauri/src/assistant/mod.rs#L331-L337)
- **Fix:** Add `|| id.len() > 64` to rejection condition.

## 133. `common_ancestor` falls back silently to roots[0] on non-existent path
- **Where:** [assistant/mod.rs:397-403](../src-tauri/src/assistant/mod.rs#L397-L403)
- **Fix:** `log::warn!` when `is_dir()` false so fallback visible in DiagBus.

## 134. `assistant_auth_probe` two-spawn TOCTOU window for CLI replacement
- **Where:** [assistant/mod.rs:592-658](../src-tauri/src/assistant/mod.rs#L592-L658)
- **Fix:** Single `claude auth status --version` call if CLI supports it; OR parallel via `tokio::join!`.

### INFO (4 — actionable docs/comments only)

## 135. `force_push_now` promotion log emitted after flush (out-of-order)
- **Where:** [auto_sync.rs:802-806](../src-tauri/src/sync/auto_sync.rs#L802-L806)
- **Fix:** Move log call before `flush_all_now(...)` at L814.

## 136. `apply_selected` emits no final `DriftScanResult` — spinner never closes
- **Where:** [auto_sync.rs:1493-1508](../src-tauri/src/sync/auto_sync.rs#L1493-L1508) (vs `force_pull_now` at [:1737-1763](../src-tauri/src/sync/auto_sync.rs#L1737-L1763))
- **Fix:** Emit final `DriftScanResult` w/ dispatched counts after `h.await`.

## 137. `walk_local` runs `should_ignore` twice per file (name then rel-path)
- **Where:** [drift_scanner.rs:521](../src-tauri/src/sync/drift_scanner.rs#L521) + [:533](../src-tauri/src/sync/drift_scanner.rs#L533)
- **Fix:** Apply name-only check only on dirs (before recursion); rely on rel-path check for files.

## 138. Sync-snapshot count-under invariant undocumented (`listing_files` vs snapshot keys)
- **Where:** [drift_scanner.rs:267](../src-tauri/src/sync/drift_scanner.rs#L267)
- **Fix:** Comment documenting snapshot-keys-are-files invariant on `count_under`.

---

## Audit 2026-05-20 — Wave 2 (frontend deep audit)

> 8 parallel `operator` agents over `src/lib/`. Reports persisted at `state/audit-2026-05-20/{L..S}-*.md`; synthesis at `SYNTHESIS-wave2.md`. Two agents (O bail-recovered; S wrote to wrong path then was relocated). Same compressed format as Wave 1.

### HIGH (4)

## 139. `drainTick` rAF callback runs on dropped tab — writes to dead `TabState`

> **SHIPPED v0.4.14-alpha S113 (uncommitted)** — `dropTab` at `assistant.svelte.ts:1146-1158` now fetches the tab via `this.tabs.get(convoId)`, calls `tab.flushPendingText()` (which internally cancels the rAF + drains any pending text), THEN removes from map. `flushPendingText` promoted from `private` → public on TabState to make it reachable. No more self-perpetuating rAF chain on orphaned state.

- **Where:** [assistant.svelte.ts:725](../src/lib/state/assistant.svelte.ts#L725)
- **Symptom:** A `TabState` dropped via `dropTab` mid-stream still has an outstanding `requestAnimationFrame`. Next frame fires `drainTick` → `appendText` → `mutateStreaming` on orphaned state. Tab no longer in `this.tabs.map` but rAF callback holds direct reference.
- **Fix:** `dropTab` must `cancelAnimationFrame(tab.drainHandle)` before removing the entry. `flushPendingText` should also be reachable from `dropTab` (currently only from `onError`).

## 140. `confirmMirrorApply` dispatches `local_path` for `to_delete_remote` bucket — semantically wrong / stale-cache risk

> **SHIPPED v0.4.14-alpha S113 (uncommitted)** — code inspection at `sync-page.svelte.ts:542-558` confirmed the function ALREADY re-filters from live `this.entries` at dispatch (`.filter((e) => e.bucket === "to_delete_remote")`); no captured open-time count is used. Audit recommendation was already satisfied. Added a WHY comment documenting the invariant so a future change doesn't reintroduce the race. No behavior change.

- **Where:** [sync-page.svelte.ts:546-548](../src/lib/state/sync-page.svelte.ts#L546-L548)
- **Symptom:** `to_delete_remote` entries collected by `e.local_path` (file already deleted locally). Lookup works against current snapshot, but if a scan rebuckets between `openMirrorConfirm` and `confirmMirrorApply`, stale paths dispatch with wrong intent.
- **Fix:** Re-filter entries at dispatch (assert `e.bucket === "to_delete_remote"`); don't use the captured count from open-time.

## 141. `lastFeedLen` plain-`let` mutation inside `$effect` — stale-closure bug

> **SHIPPED v0.4.14-alpha S113 (uncommitted)** — Wave-1 Agent C, clean. `ActivityFeed.svelte:39` converted `let lastFeedLen = 0` → `let lastFeedLen = $state(0)`. Read inside the effect wrapped in `untrack(() => lastFeedLen)` so writes don't retrigger the effect; added `if (delta === 0) return` early-return as belt-and-suspenders against the hidden-tab skip case.

- **Where:** [ActivityFeed.svelte:38,98](../src/lib/components/activity/ActivityFeed.svelte#L98)
- **Symptom:** Not a `$state` var; effect doesn't track. If effect skipped while tab hidden (display:none), next run computes inflated `delta` → spurious burst-mode entry.
- **Fix:** `$state(0)` w/ untracked read via `untrack(() => lastFeedLen)`, OR add `delta === 0` early-return after the existing `Math.max(0, ...)` clamp.

## 142. `recentArrivals` plain array `.push()` — invisible to Svelte reactivity

> **SHIPPED v0.4.14-alpha S113 (uncommitted)** — Wave-1 Agent C, clean. `ActivityFeed.svelte:40` converted `let recentArrivals: number[] = []` → `let recentArrivals = $state<number[]>([])`. Svelte 5's deep proxy now tracks `.push()` mutations directly; burst-detection thresholds see every arrival.

- **Where:** [ActivityFeed.svelte:39,84](../src/lib/components/activity/ActivityFeed.svelte#L84)
- **Symptom:** Declared `let recentArrivals: number[] = []` (no `$state`). `.push(now)` is silent — only the `filter(...)` reassignment publishes updates. Burst-detection thresholds may miss arrivals.
- **Fix:** `let recentArrivals = $state<number[]>([])` so `.push()` is tracked via deep proxy.

### MED (35)

## 143. `convoCreatedAt` + `currentCliSessionId` are store-level but carry per-tab meaning
- **Where:** [assistant.svelte.ts:1233,1236](../src/lib/state/assistant.svelte.ts#L1233)
- **Symptom:** Async gap (e.g., 700ms `scheduleSave`) sees whichever tab is active when the timeout fires, not the originating tab. Violates HANDOFF.md's "per-tab state belongs on TabState class" rule.
- **Fix:** Move `convoCreatedAt`, `currentCliSessionId`, `convoTitle` to TabState; store-level getters delegate to `activeTab`.
> SHIPPED v0.4.14-alpha S114 (uncommitted) — `convoCreatedAt`/`convoTitle`/`cliSessionId` now `$state` on TabState; store provides delegating getter+setter pairs. send()/openTab/newTab/closeTab reordered so `ensureTab` precedes per-tab field writes (3 sites previously clobbered the freshly-set neighbor `cliSessionId` via store setter null-writes).

## 144. `closeTabsToRight` + `closeOtherTabs` skip `dropTab`/`pruneTabUi` for removed tabs
- **Where:** [assistant.svelte.ts:1921-1931](../src/lib/state/assistant.svelte.ts#L1921-L1931) (right), [:1892-1899](../src/lib/state/assistant.svelte.ts#L1892-L1899) (others)
- **Symptom:** `tabDrafts`/`tabAttachments`/`tabScroll` Maps + `tabs` Map grow unbounded over a long session. Re-opening from History resurrects ghost drafts.
- **Fix:** Iterate removed ids → `dropTab(id)` + `pruneTabUi(id)` before updating `openTabs`. (L3+L4+M3+M4 dupes.)
> SHIPPED v0.4.14-alpha S114 (uncommitted) — both methods now loop removed ids → `dropTab` + `pruneTabUi` before swapping `openTabs`. `dropTab` already flushes pendingText (S113 #139), so rAF drainTick can't write to a dropped tab.

## 145. `scheduleSave` / `flushNow` / `deriveTitle` all read `this.messages` (active-tab getter)
- **Where:** [assistant.svelte.ts:1569-1578](../src/lib/state/assistant.svelte.ts#L1569-L1578) (deriveTitle), [:1599](../src/lib/state/assistant.svelte.ts#L1599) (flushNow), [:1618](../src/lib/state/assistant.svelte.ts#L1618) (doSave)
- **Symptom:** Background-tab turn-complete fires `scheduleSave`; 700ms later `doSave` reads `this.messages` → returns ACTIVE tab's messages. Saved record gets wrong content + wrong title. `beforeunload` only flushes active tab.
- **Fix:** Snapshot `{ id, messages, ... }` at scheduleSave call time; pass tab as arg through to deriveTitle/doSave. `flushNow` iterate `this.tabs` for all unsaved.
> SHIPPED v0.4.14-alpha S114 (uncommitted) — `saveTimer` moved to TabState (per-tab debounce slot). `scheduleSave` captures `(tab, convoId)` at call time; closure-bound `doSave` reads from that tab regardless of who's active 700ms later. `deriveTitle(tab)` + new `buildSaveRecord(convoId, tab)` helper. `flushNow` now iterates `this.tabs` so beforeunload saves every dirty tab.

## 146. `mutateStreaming` rebuilds full messages array on every delta
- **Where:** [assistant.svelte.ts:578](../src/lib/state/assistant.svelte.ts#L578)
- **Symptom:** `.map(...)` over entire array per text/thinking/tool-result update; high-velocity streaming causes meaningful GC pressure + frame drops in long convos.
- **Fix:** Cache `streamingMsgId` index at `beginTurn`; direct index-replace instead of full map.

## 147. `ensureThinkingFromEnvelope` `b === existing` always false on `$state` proxies
- **Where:** [assistant.svelte.ts:677](../src/lib/state/assistant.svelte.ts#L677)
- **Symptom:** Svelte 5 proxies are not referentially equal across read sites. Guard always false → every call appends a NEW thinking block instead of merging into the existing one.
- **Fix:** Use stable key — `b.type === "thinking" && b.startedAt === existing.startedAt`.

## 148. `handleTurnComplete` `queueMicrotask` send-on-next-item races tab switch
- **Where:** [assistant.svelte.ts:2200-2203](../src/lib/state/assistant.svelte.ts#L2200-L2203)
- **Symptom:** User switches tabs between active-tab check and microtask firing; `send()` runs on new active tab's context, not original tab.
- **Fix:** `sendOnTab(tab, text)` overload, OR capture convoId + bail in microtask if `currentConvoId` differs.

## 149. `openTab` race against `deleteConversation` → blank TabState
- **Where:** [assistant.svelte.ts:1772-1779](../src/lib/state/assistant.svelte.ts#L1772-L1779)
- **Symptom:** Brief window between `deleteConversation` removing from disk and `refreshConversations` clearing the metadata list. `openTab` sees the convo as loadable → `loadConversation` throws → tab left in error state.
- **Fix:** After `deleteConversation` calls `refreshConversations`, also explicitly close any tab pointing at the deleted id.

## 150. `asstApiKeyDraft`/`asstMaxBudgetDraft` $effect overwrites unsaved edits
- **Where:** [Settings.svelte:156-161](../src/lib/components/settings/Settings.svelte#L156-L161)
- **Symptom:** N flagged "stale on first visit" (writes before async init resolves); O flagged "overwrites mid-edit" (re-fires on every store mutation). Both are real — async init eventually fires the effect, but it also re-fires on later store changes, clobbering local drafts.
- **Fix:** Wrap store-read assignments in `untrack(() => { ... })`; OR separate effect that tracks ONLY a section-entry signal, not the store fields themselves.
> SHIPPED v0.4.14-alpha S114 (uncommitted) — `$effect` now depends only on `section`; store reads are inside `untrack(() => assistantStore.init().then(...))` so init's resolution still populates the drafts (fixes "stale on first visit") but later store mutations no longer re-fire the effect (fixes "overwrites mid-edit").

## 151. Theme picker + STT lang picker missing `role="radiogroup"` / `role="radio"`
- **Where:** [Settings.svelte:539-555](../src/lib/components/settings/Settings.svelte#L539-L555) (theme), [:835-847](../src/lib/components/settings/Settings.svelte#L835-L847) (STT)
- **Symptom:** Screen readers announce buttons w/o selection state; keyboard users can't navigate as radio groups. Cursor (L435) and Bell (L490) segmented controls already do this correctly.
- **Fix:** Wrap grids in `role="radiogroup" aria-label="..."`; add `role="radio" aria-checked={...}` per option.

## 152. `srv-card` is `<div role="button">` containing nested `<button>` Edit/Delete — invalid ARIA
- **Where:** [Settings.svelte:931-958](../src/lib/components/settings/Settings.svelte#L931-L958)
- **Symptom:** ARIA 1.1 forbids interactive descendants in `role="button"`. Nested Edit/Delete are unreachable as focusable children via AT.
- **Fix:** Convert outer `<div>` to `<button type="button">` and restructure Edit/Delete as siblings, OR drop `role="button"` on the div + use a dedicated left-column select button.

## 153. `syncNow()` busy clears before trailing rescan settles — 1.2s race window
- **Where:** [sync-page.svelte.ts:431,462-464](../src/lib/state/sync-page.svelte.ts#L431)
- **Symptom:** Double-click Sync within 1.2s starts second `syncNow()` while prior `rescan()` is still pending → overlapping reconciles.
- **Fix:** Chain `rescan()` into the same promise chain (keep `busy=true` until it returns), OR debounce Sync button for 1.5s post-success.

## 154. `groupSelectionState`/`selectAllIn`/`clearSelectionIn` omit `to_delete_remote`
- **Where:** [SyncPage.svelte:166-178](../src/lib/components/sync/SyncPage.svelte#L166-L178), [sync-page.svelte.ts:378-395](../src/lib/state/sync-page.svelte.ts#L378-L395)
- **Symptom:** "Select all" never picks remote-delete entries; group checkbox shows "all selected" while remote-delete rows remain unselected.
- **Fix:** Add `...g.to_delete_remote` to items array; mirror in store helpers.

## 155. Mirror confirm modal — no focus trap / autofocus / Tab containment
- **Where:** [SyncPage.svelte:631-683](../src/lib/components/sync/SyncPage.svelte#L631-L683)
- **Symptom:** Focus stays in sync list when modal opens; Tab escapes the dialog into background controls.
- **Fix:** `autofocus` on confirm input, OR programmatic `.focus()` on modal mount. Optional focus-trap on first/last focusable.

## 156. `WatchedFoldersTable` diag listener TOCTOU on rapid remount
- **Where:** [WatchedFoldersTable.svelte:33-43](../src/lib/components/sync/WatchedFoldersTable.svelte#L33-L43)
- **Symptom:** Async `onMount` awaits `listen()`; if component unmounts before resolve, cleanup variable never set → listener orphaned → remount stacks duplicates → `refresh()` double-fires.
- **Fix:** `aborted` flag set in `onDestroy`, checked after `listen()` resolves. Or move to synchronous `$effect` w/ cleanup return.

## 157. `relPathLabel` falls back to absolute `local_path` when `rel_path` is empty string
- **Where:** [SyncPage.svelte:134-136](../src/lib/components/sync/SyncPage.svelte#L134-L136)
- **Symptom:** Empty-string `rel_path` (backend derivation fallback) → drift list shows full FS path `C:/fxserver/resources/...` — exposes user's local layout.
- **Fix:** Fall back to `basename(local_path)`; normalize empty → `null` in `refresh()`.

## 158. `.rift-conflict.` copies in `to_push` bucket get no visual distinction
- **Where:** [SyncPage.svelte:571](../src/lib/components/sync/SyncPage.svelte#L571), [DriftSummaryCard.svelte:69-73](../src/lib/components/sync/DriftSummaryCard.svelte#L69-L73)
- **Symptom:** Frontend surface of Wave 1 #42. If conflict copies leak into `to_push`, users may unknowingly push them to remote.
- **Fix:** Detect `/.rift-conflict\./` in `relPathLabel`/render; show warning chip. (Rust-side ignore-rule fix is the authoritative path per #42.)

## 159. `AssistantHeader` pulse setTimeout not cleaned up
- **Where:** [AssistantHeader.svelte:37-45](../src/lib/components/assistant/AssistantHeader.svelte#L37-L45)
- **Symptom:** Workspace-switch during 700ms pulse → callback runs on detached closure; `lastSeenUpdate` stays stale.
- **Fix:** Return cleanup from `$effect` that clears the handle.

## 160. `Composer` `onblur` kills mention picker before mousedown — pick-on-click broken
- **Where:** [Composer.svelte:648](../src/lib/components/assistant/Composer.svelte#L648)
- **Symptom:** Click on mention suggestion: textarea blur fires first, sets `mentionState = null`, menu disappears, mousedown never lands on item.
- **Fix:** Guard blur — `if (!mentionState) return; requestAnimationFrame(() => mentionState = null);` (standard combobox blur-cancel pattern).

## 161. `MessageBubble` tick `setInterval` can double-register on flag co-flip
- **Where:** [MessageBubble.svelte:147-157](../src/lib/components/assistant/MessageBubble.svelte#L147-L157)
- **Symptom:** Interval-start branch lacks `!tickHandle` guard. Reactive batch can trigger two effect runs → two intervals → double-rate tick.
- **Fix:** Unconditionally `clearInterval(tickHandle); tickHandle = null;` at effect-start, then conditionally restart.

## 162. `Markdown` checklist sync $effect fires on every streaming token
- **Where:** [Markdown.svelte:202-204](../src/lib/components/assistant/Markdown.svelte#L202-L204)
- **Symptom:** `pinTasksFromChecklist` called per delta. `processed` derives via DOMPurify + 2 DOM-template walks each tick. 200 streaming tokens → ~200 full DOM parses.
- **Fix:** Debounce, OR equality-check `JSON.stringify(processed.items)` against ref before calling.

## 163. `UpdateDialog` uses banned `scrollbar-gutter: stable` — WebView2 arrow-button leak

> **SHIPPED v0.4.14-alpha S113 (uncommitted)** — `UpdateDialog.svelte:409` `scrollbar-gutter: stable` line deleted from `.upd-body` block. Closes the DON'T-TOUCH violation per HANDOFF.md CRITICAL DON'T-TOUCH section.
- **Where:** [dialogs/UpdateDialog.svelte:409](../src/lib/components/dialogs/UpdateDialog.svelte#L409)
- **Symptom:** Exact pattern HANDOFF.md CRITICAL DON'T-TOUCH bans on `.scroll`/`.strip`. Native arrow buttons leak top-right of dialog body.
- **Fix:** Drop `scrollbar-gutter: stable`. Use `scrollbar-width: none` + `::-webkit-scrollbar { display: none }`.

## 164. `Terminal.svelte` init/teardown race — `term_spawn` resolves after teardown
- **Where:** [Terminal.svelte:226-230](../src/lib/components/terminal/Terminal.svelte#L226-L230)
- **Symptom:** `visible` flips false (or onDestroy fires) during in-flight `term_spawn`; teardown clears `sessionId`; spawn resolves and sets `sessionId` on disposed terminal → orphaned backend process.
- **Fix:** `mounting` guard; after await check whether `term` is still the original instance or `sessionId` was cleared → immediately `term_kill` the orphan.

## 165. `SearchAddon` not explicitly disposed before `term.dispose()`
- **Where:** [Terminal.svelte:218-219](../src/lib/components/terminal/Terminal.svelte#L218-L219)
- **Symptom:** `search` + `fit` nulled but `.dispose()` never called. `SearchAddon.onDidChangeResults` emitter can outlive terminal in some xterm versions.
- **Fix:** `search?.dispose(); fit?.dispose();` before `term?.dispose()`.

## 166. `TerminalFindBar` debounce $effect lacks return cleanup
- **Where:** [TerminalFindBar.svelte:44-58](../src/lib/components/terminal/TerminalFindBar.svelte#L44-L58)
- **Symptom:** Effect manually clears `searchTimer` at entry, but no `return () => clearTimeout(...)` cleanup. Stale timer from prior run can fire after rapid query changes that skip the setTimeout branch.
- **Fix:** Return cleanup function from $effect that clears `searchTimer` + nulls it.

## 167. `TerminalFindBar` `api.onResults` wired only on mount — tab-switch breaks counts
- **Where:** [TerminalFindBar.svelte:60-69](../src/lib/components/terminal/TerminalFindBar.svelte#L60-L69)
- **Symptom:** Component holds old `detachResults` after `api` prop changes (tab switch). Result-count display freezes for new tab.
- **Fix:** Move subscription into `$effect(() => { detachResults?.(); if (api) detachResults = api.onResults(...); })`.

## 168. `flash()` setTimeout leaks on unmount
- **Where:** [ActivityFeed.svelte:360-363](../src/lib/components/activity/ActivityFeed.svelte#L360-L363)
- **Symptom:** Workspace switch within 1500ms → callback writes `actionFlash` to unmounted component.
- **Fix:** Track timer id in `$state`; `onDestroy(() => { if (flashTimer) clearTimeout(flashTimer); })`.

## 169. `dialogs.svelte.ts` callbacks captured at script-init never cleared on AppShell destroy
- **Where:** [AppShell.svelte:97-100](../src/lib/components/AppShell.svelte#L97-L100)
- **Symptom:** Singleton holds destroyed-component closures. Route-level HMR remount stacks callbacks; calling stale ones operates on dead `$state`.
- **Fix:** Move assignments into `onMount`; reset to no-op in `onDestroy`.

## 170. `connection.autoReconnect()` bypasses `connecting` flag → concurrent manual connect possible
- **Where:** [connection.svelte.ts:465-478](../src/lib/state/connection.svelte.ts#L465-L478)
- **Symptom:** Auto-reconnect uses separate `reconnecting` boolean; `connect()` guard checks only `connecting`, not `reconnecting`. User toggle during auto-reconnect → two concurrent `start_autosync` IPC calls.
- **Fix:** Set `this.connecting = true` in `autoReconnect` w/ `finally` clear, OR check both flags in `connect()` guard.

## 171. `connecting` flag stuck `true` if TOFU modal dismissed without confirm/cancel
- **Where:** [connection.svelte.ts:207-233](../src/lib/state/connection.svelte.ts#L207-L233)
- **Symptom:** Fingerprint-probe branch `return`s early without `finally` clearing `connecting`. AppShell `fingerprintHandled` effect normally pipes back the result, but if `askConfirm` never resolves (component unmount mid-dialog), `connecting` stays true → StatusBar button locked.
- **Fix:** Verify `cancelFingerprint` reaches every dismiss path; OR wrap in try/finally that clears `connecting` if neither callback fired.

## 172. `connection.wireEvents()` guard allows double-bind after `disposeEvents()`
- **Where:** [connection.svelte.ts:289-291](../src/lib/state/connection.svelte.ts#L289-L291)
- **Symptom:** Concurrent `retryWire()` (double-click) before `wiring` flips → race between partial-init rollback and re-entry. Listeners survive.
- **Fix:** Verify `finally { this.wiring = false }` placement happens after all `unlisteners.push()` completes. Stress-test.

## 173. `updates.svelte.ts` Tauri listeners never unregistered (HMR memory leak)
- **Where:** [updates.svelte.ts:171-185](../src/lib/state/updates.svelte.ts#L171-L185)
- **Symptom:** `progressUnlisten`/`downloadedUnlisten` set once via `ensureListeners`; no teardown. HMR reload leaves old listeners live, firing into stale store.
- **Fix:** Expose `dispose()` calling both unlistens; wire into AppShell `onDestroy`.

## 174. `AppShell` has two independent "alive" booleans (`alive` + `shellAlive`)
- **Where:** [AppShell.svelte:109,289](../src/lib/components/AppShell.svelte#L109)
- **Symptom:** Confusing parallel state; future edit removing one will diverge them silently.
- **Fix:** Consolidate to one `$state(true)` shared by both effects.

## 175. `stt.svelte.ts` `this.recognition` dual-role — handle + commit-flag
- **Where:** [stt.svelte.ts:287](../src/lib/state/stt.svelte.ts#L287)
- **Symptom:** `onEnd` checks `this.recognition` to decide whether to commit; `cancel()` nulls `recognition` to suppress commit. Intent works but is undocumented + fragile to future edits.
- **Fix:** Explicit `cancelRequested: boolean` field; separate handle role from intent.

## 176. `Titlebar.svelte` server picker missing ARIA + Escape-key + roles
- **Where:** [Titlebar.svelte:56-104](../src/lib/components/shell/Titlebar.svelte#L56-L104)
- **Symptom:** No `aria-expanded` on trigger, no `aria-haspopup`, no `Escape` key handler, no `role="menu"`. Tab focus can drift into the hidden menu.
- **Fix:** Add `aria-expanded={menuOpen}` + `aria-haspopup="listbox"`; keydown for Escape + arrow nav.

## 177. `beforeunload` listener leak in assistant store (HMR-only)
- **Where:** [assistant.svelte.ts:1422-1423](../src/lib/state/assistant.svelte.ts#L1422-L1423)
- **Symptom:** Anonymous listener registered in `init()`, no removal path. Singleton survives prod fine; HMR re-init in dev stacks duplicates.
- **Fix:** Store handler in class field; provide `destroy()` calling `removeEventListener`.

### LOW (28)

## 178. `applyTodoWrite` id generation `todo-${i}-${slice}` not stable across calls
- **Where:** [assistant.svelte.ts:756-763](../src/lib/state/assistant.svelte.ts#L756-L763)
- **Symptom:** Same content at same index → same id (good); but reorder/insert → all downstream ids change. Keyed `{#each}` unmounts everything, visible flash.
- **Fix:** Content-based hash, OR preserve existing ids on update when content matches.

## 179. `stop()` doesn't flush pendingText before clearing `streamingMsgId`
- **Where:** [assistant.svelte.ts:2211-2217](../src/lib/state/assistant.svelte.ts#L2211-L2217)
- **Symptom:** Stop click → `streamingMsgId = null` first → next rAF tick `mutateStreaming` early-returns → buffered text silently dropped.
- **Fix:** Call `tab.flushPendingText()` before `tab.streamingMsgId = null`.

## 180. `init()` re-entrance guard skips fresh listeners on HMR
- **Where:** [assistant.svelte.ts:1341](../src/lib/state/assistant.svelte.ts#L1341)
- **Symptom:** `unlistens.length > 0` skips re-init; stale module-eval closures handle events with outdated refs.
- **Fix:** Add `destroy()` that calls all unlistens + resets; wire to `import.meta.hot.dispose`.

## 181. `restoreTabs` `persistTabs()` not in finally — partial state written on throw
- **Where:** [assistant.svelte.ts:1744-1747](../src/lib/state/assistant.svelte.ts#L1744-L1747)
- **Fix:** Wrap body in try/finally; call `persistTabs()` in finally.

## 182. Post-done orphaned non-JSON CLI lines silently dropped
- **Where:** [assistant.svelte.ts:874](../src/lib/state/assistant.svelte.ts#L874)
- **Fix:** Add `console.debug("[assistant] orphaned non-JSON line (post-done)", raw.slice(0, 80))` for observability.

## 183. `cacheBustHintShown` plain non-reactive — HMR resets in dev (INFO bordering LOW)
- **Where:** [assistant.svelte.ts:1285](../src/lib/state/assistant.svelte.ts#L1285)
- **Fix:** Gate via `sessionStorage` key in addition to in-memory flag (dev ergonomics only).

## 184. `send()` doesn't clear `storeLastError` — stale error banner persists into first turn
- **Where:** [assistant.svelte.ts:2029](../src/lib/state/assistant.svelte.ts#L2029)
- **Fix:** `this.lastError = null;` after `this.lastNotice = null;` (setter routes correctly).

## 185. `retryLast` no re-entrancy guard — fast double-call pops two pairs
- **Where:** [assistant.svelte.ts:2356-2364](../src/lib/state/assistant.svelte.ts#L2356-L2364)
- **Fix:** `let retrying = false` field, or check msg-count after `await stop()`.

## 186. `diagCopied` setTimeout leaks on workspace switch
- **Where:** [Settings.svelte:109](../src/lib/components/settings/Settings.svelte#L109)
- **Fix:** Store timer in field; clear in `onDestroy` / effect cleanup.

## 187. `loadAboutPaths()` fires unconditionally on every Settings mount
- **Where:** [Settings.svelte:190](../src/lib/components/settings/Settings.svelte#L190)
- **Fix:** Gate in `$effect(() => { if (section === "about") ... })`.

## 188. `connection.loadServers()` no idempotency guard
- **Where:** [connection.svelte.ts:180](../src/lib/state/connection.svelte.ts#L180), called from [Settings.svelte:191](../src/lib/components/settings/Settings.svelte#L191)
- **Fix:** Early-exit `if (servers.length > 0) return;` OR move to lazy `$effect`.

## 189. Nav buttons missing `aria-current` — active section not announced
- **Where:** [Settings.svelte:205](../src/lib/components/settings/Settings.svelte#L205)
- **Fix:** `aria-current={section === s.id ? "page" : undefined}`.

## 190. `aria-checked` on "Use full Claude config" switch misrepresents persisted state
- **Where:** [Settings.svelte:595](../src/lib/components/settings/Settings.svelte#L595)
- **Symptom:** `aria-checked={useFullConfig && !apiKey}` announces false even when stored pref is true (API-key override). Misleads AT.
- **Fix:** `aria-checked={useFullConfig}` to reflect stored pref; rely on `disabled` + adjacent text for override context.

## 191. Outside-click dropdown $effect can leave stale listener attached
- **Where:** [Settings.svelte:41](../src/lib/components/settings/Settings.svelte#L41)
- **Fix:** Split per-dropdown effects, or unconditional add/remove on mount/destroy.

## 192. Shell + font dropdowns: `role="listbox"` with `<button>` children, no `aria-activedescendant`
- **Where:** [Settings.svelte:287](../src/lib/components/settings/Settings.svelte#L287) (shell), [:381](../src/lib/components/settings/Settings.svelte#L381) (font)
- **Symptom:** Mixed-role widget; arrow-key listbox navigation absent.
- **Fix:** Add ids + `aria-activedescendant`; keydown for Up/Down/Home/End/Enter/Escape.

## 193. `{#key section}` open dropdowns not reset on section change
- **Where:** [Settings.svelte:212](../src/lib/components/settings/Settings.svelte#L212)
- **Fix:** Reset `shellDdOpen`/`fontDdOpen` to false in nav `onclick` before updating `section`.

## 194. `{#key section}` + `out:fade`/`in:fly` overlap on rapid nav
- **Where:** [Settings.svelte:212-216](../src/lib/components/settings/Settings.svelte#L212-L216)
- **Fix:** `out` duration 0, OR debounce section change.

## 195. Edit/Delete server buttons have static `aria-label` regardless of server
- **Where:** [Settings.svelte:950-954](../src/lib/components/settings/Settings.svelte#L950-L954)
- **Fix:** `` aria-label={`Edit ${s.name}`} `` / `` `Delete ${s.name}` ``.

## 196. `stt.init()` $effect re-evaluates on every section change
- **Where:** [Settings.svelte:144-148](../src/lib/components/settings/Settings.svelte#L144-L148)
- **Fix:** Early-return on `section !== "speech"`, OR move to nav button onclick.

## 197. "Clear" budget button visible-flash on click
- **Where:** [Settings.svelte:632-634](../src/lib/components/settings/Settings.svelte#L632-L634)
- **Fix:** Disable button during `asstMaxBudgetSaving`.

## 198. `selBreakdown` rebuilds Map per derivation tick from full entries list
- **Where:** [SyncPage.svelte:117-132](../src/lib/components/sync/SyncPage.svelte#L117-L132)
- **Fix:** Derive from `syncPage.groups` (already grouped) instead of `entries`.

## 199. `fmtRel` in RecentActivityCard has no null guard (vs WatchedFoldersTable parity)
- **Where:** [RecentActivityCard.svelte:47-56](../src/lib/components/sync/RecentActivityCard.svelte#L47-L56) vs [WatchedFoldersTable.svelte:66](../src/lib/components/sync/WatchedFoldersTable.svelte#L66)
- **Fix:** `if (!iso) return "—"` first line.

## 200. `DriftSummaryCard` re-groups `entries` independently of `syncPage.groups`
- **Where:** [DriftSummaryCard.svelte:18-36](../src/lib/components/sync/DriftSummaryCard.svelte#L18-L36)
- **Fix:** Derive from `syncPage.groups` to eliminate parallel derivation path.

## 201. `.conflicts-inline-chev` no transition — chevron snaps vs animates
- **Where:** [SyncPage.svelte:1259-1267](../src/lib/components/sync/SyncPage.svelte#L1259-L1267) (CSS)
- **Fix:** Add `transition: transform 140ms cubic-bezier(0.4, 0, 0.2, 1)`.

## 202. `scanAgeLabel` is dead code or non-reactive (renders stale)
- **Where:** [SyncPage.svelte:201-207](../src/lib/components/sync/SyncPage.svelte#L201-L207)
- **Fix:** If used: wrap in `$derived` w/ 30s tick; if unused: delete.

## 203. `countFor()` O(N×9) per render — not memoized
- **Where:** [ActivityFeed.svelte:144-146](../src/lib/components/activity/ActivityFeed.svelte#L144-L146)
- **Fix:** `$derived` map `Record<Group, number>` once per feed update.

## 204. Group-header `{#each}` key includes `rows.length` — forces destroy/recreate on each event
- **Where:** [ActivityFeed.svelte:506](../src/lib/components/activity/ActivityFeed.svelte#L506)
- **Fix:** Remove `rows.length` from key; let derived `rendered` update existing nodes.

## 205. `HistoryDrawer.focusOnMount` action returns nothing — Svelte 5 API mismatch
- **Where:** [HistoryDrawer.svelte:49-52](../src/lib/components/assistant/HistoryDrawer.svelte#L49-L52)
- **Fix:** Return `{ destroy() {} }` to silence warning.

## 206. `StepGroup` collapsible `role="button"` div w/o `aria-label`
- **Where:** [StepGroup.svelte:64-70](../src/lib/components/assistant/StepGroup.svelte#L64-L70)
- **Fix:** `` aria-label={`Toggle step ${stepNum}: ${headerText}`} ``.

## 207. `Composer` hint popover `role="dialog"` — wrong role, no focus management
- **Where:** [Composer.svelte:634](../src/lib/components/assistant/Composer.svelte#L634)
- **Fix:** Use `role="tooltip"` (passive surface) + link via `aria-describedby`.

## 208. `ChatTabsBar` drag state not reset on `dragcancel` / workspace-switch
- **Where:** [ChatTabsBar.svelte:88-91](../src/lib/components/shell/ChatTabsBar.svelte#L88-L91)
- **Fix:** Add `ondragcancel={onDragEnd}` + effect cleanup that resets drag state.

## 209. `Markdown` code-copy `<span role="button">` not keyboard-activatable
- **Where:** [Markdown.svelte:148-150](../src/lib/components/assistant/Markdown.svelte#L148-L150)
- **Fix:** Add `onkeydown` for Enter/Space on `.md` wrapper, OR replace `<span>` with `<button>`.

## 210. `UpdateToast` timer no-ops on rapid visibility re-trigger w/ hover
- **Where:** [UpdateToast.svelte:24-28](../src/lib/components/UpdateToast.svelte#L24-L28)
- **Fix:** Reset `hovering = false` after disarm to ensure re-trigger re-arms.

## 211. `TerminalPanel` global keydown listener fires for all workspaces
- **Where:** [TerminalPanel.svelte:163-166](../src/lib/components/terminal/TerminalPanel.svelte#L163-L166)
- **Fix:** Condition `$effect` on `isVisible`; acceptable as-is given cheap guard.

## 212. `ActivityBar` Settings tooltip shows `Ctrl+9` only, hides `Ctrl+,`
- **Where:** [ActivityBar.svelte:51](../src/lib/components/shell/ActivityBar.svelte#L51)
- **Fix:** Append `· Ctrl+,` to title for Settings id.

## 213. `PageHeader` `data-tone="neutral"` dims entire header w/ stacked opacity
- **Where:** [PageHeader.svelte:73](../src/lib/components/shell/PageHeader.svelte#L73)
- **Symptom:** `opacity: 0.35` on parent multiplies with `::after { opacity: 0.55 }` → stripe ~0.19. Header text, icon, badges all dimmed.
- **Fix:** Move `opacity: 0.25` onto `[data-tone="neutral"]::after` only.

## 214. `connection.autoReconnect` has unlimited retries, no backoff
- **Where:** [connection.svelte.ts:449-478](../src/lib/state/connection.svelte.ts#L449-L478)
- **Fix:** `reconnectAttempts` counter; cap at 5 or apply `2^n * 1000ms` backoff.

### INFO (5)

## 215. `StatusBar.app_version` duplicates `updates.currentVersion` IPC
- **Where:** [StatusBar.svelte:61-65](../src/lib/components/shell/StatusBar.svelte#L61-L65)
- **Fix:** `$derived(() => updates.currentVersion)` after `checkOnLaunch()` resolves.

## 216. `kindVariant "muted"` has no CSS coverage for `data-selected="true"` state
- **Where:** [ActivityFeed.svelte:332](../src/lib/components/activity/ActivityFeed.svelte#L332), CSS at [:818-821](../src/lib/components/activity/ActivityFeed.svelte#L818-L821)
- **Fix:** Add `[data-selected="true"][data-variant="muted"]` rule, OR exhaustive switch w/ `satisfies`.

## 217. `EmptyState.pick()` may not focus textarea if draft unchanged
- **Where:** [EmptyState.svelte:76-78](../src/lib/components/assistant/EmptyState.svelte#L76-L78)
- **Fix:** CDP-verify; if needed, add explicit `tick().then(() => ta?.focus())`.

## 218. AppShell `onResized` listener pattern is correct but uses parallel `alive`/`shellAlive`
- **Where:** [AppShell.svelte:106-122](../src/lib/components/AppShell.svelte#L106-L122)
- **Fix:** See #174 — consolidate aliveness flags.

---

## Audit 2026-05-20 — Wave 3 (cross-cutting)

> 8 parallel `operator` agents. Reports persisted at `state/audit-2026-05-20/{T..AA}-*.md`; synthesis at `SYNTHESIS-wave3.md`. Six of eight bailed skeleton-only and were SendMessage-recovered. Y was re-spawned tighter after the agent-dispatch-guard hook blocked the original 38-line prompt.

### Wave 1 #42 cross-verification — NOT A REAL BUG

Agent T verified via [auto_sync/watch.rs:245](../src-tauri/src/sync/auto_sync/watch.rs#L245) + [sync/ignore.rs:91-97,157](../src-tauri/src/sync/ignore.rs#L91-L157). `classify()` extracts basename via `rsplit('/').next()` before `.rift-conflict.` substring check. Absolute vs relative path makes no difference for the conflict-copy marker rule. **#42 is closed/INFO.** Frontend safety chip from #158 is still useful but not security-critical.

### HIGH (2)

## 219. No panic hook installed — Rust panics are silent to UI

> **SHIPPED v0.4.14-alpha S113 (uncommitted)** — `lib.rs:1745-1770` installs `std::panic::set_hook` immediately after `LogForwarder::install()` and before the Velopack hook. Hook extracts location + payload (handles `&str` + `String` payload variants), routes through `log::error!` (picked up by LogForwarder → bus) AND emits a `DiagStage::System` Error event so the Sync Inspector surfaces the panic.

- **Where:** [lib.rs:1743](../src-tauri/src/lib.rs#L1743) (LogForwarder install site, no `set_hook` adjacent)
- **Symptom:** Any async-task panic silently dies; no diag event, no toast, no log line. Users see the feature stop working w/ no error. Particularly damaging for `tokio::spawn(...)` calls that drop the JoinHandle.
- **Fix:** Add `std::panic::set_hook` in `run()` after `LogForwarder::install()` that calls `log::error!` with the panic info — flows into the bus naturally.

## 220. `session_id` accepted unvalidated → CLI args + sidecar path traversal

> **SHIPPED v0.4.14-alpha S113 (uncommitted)** — new `is_valid_session_id(&str)` in `assistant/mod.rs` enforces canonical 36-char UUID shape (8-4-4-4-12 hex w/ hyphens at fixed positions). Called at top of `assistant_send` before any CLI arg or sidecar path use; rejects non-UUID, leading-dash flag injection, and path-traversal segments. No new deps (no `uuid` crate pulled in — byte-level check).

- **Where:** [assistant/mod.rs:1195,1197](../src-tauri/src/assistant/mod.rs#L1195)
- **Symptom:** Renderer-supplied `session_id: String` flows verbatim to `--session-id`/`--resume` CLI args AND to `save_session_cwd` filename derivation. Non-UUID + path-traversal segments influence sidecar disk location.
- **Fix:** Validate as UUID (regex or `uuid` crate) before any use; one guard covers CLI arg and sidecar.

### MED (16)

## 221. `model` param accepted unvalidated → leading-dash flag injection into Claude CLI
- **Where:** [assistant/mod.rs:1039,1180](../src-tauri/src/assistant/mod.rs#L1039)
- **Symptom:** Model value starting with `-` interpreted by CLI's arg parser as a flag (`--model --some-flag`).
- **Fix:** Allowlist `"sonnet"|"opus"|"haiku"`, or reject leading dash via `[a-zA-Z0-9._-]+` w/o leading `-`.

## 222. `stderr_task.await.unwrap_or_default()` silently drops JoinError
- **Where:** [assistant/mod.rs:1405](../src-tauri/src/assistant/mod.rs#L1405)
- **Symptom:** Stderr drain panic (OOM) → blank stderr → user sees "claude exited with 1 — " w/ no diagnosis.
- **Fix:** `.unwrap_or_else(|e| format!("(stderr task panicked: {e})"))` + `log::error!`.

## 223. `create_dir_all` for download staging dirs silently ignored
- **Where:** [lib.rs:1066,1084,1090](../src-tauri/src/lib.rs#L1066)
- **Symptom:** Dir-create failure (perms, path length) → opaque "no such file" SFTP error w/ no upstream cause.
- **Fix:** `.map_err(|e| format!("mkdir: {e}"))?` or log + warn.

## 224. `try_read_lock` `.ok()?` conflates absent-lock vs SFTP-error
- **Where:** [lock_presence.rs:359](../src-tauri/src/sync/lock_presence.rs#L359)
- **Symptom:** SFTP failure → returns None → caller treats as "no lock held" → permits write that could collide w/ another user's active lock.
- **Fix:** Separate error cases; on SFTP error log warn + return sentinel → caller treats as unknown / skip write.

## 225. `eprintln!` in sync handlers + drift scanner bypass log + diag bus
- **Where:** [lib.rs:132,134,136,165,167,169,183,185,187,190,192](../src-tauri/src/lib.rs#L132) + [drift_scanner.rs:272](../src-tauri/src/sync/drift_scanner.rs#L272)
- **Symptom:** Stderr-only — never reach LogForwarder, diag bus, or UI. Shipped binaries produce no signal.
- **Fix:** Replace all 12 `eprintln!` with `log::info!` / `log::debug!`. Drift-scanner's `eprintln!` at L272 is also redundant w/ the `emit_with_fields` at L276 — delete it.

## 226. Broadcast bus lag silently counted, no log/diag event
- **Where:** [diagnostics/mod.rs:439-441](../src-tauri/src/diagnostics/mod.rs#L439-L441)
- **Symptom:** `RecvError::Lagged(n)` → `bus_lag_total += n` w/o emit; only visible via 500ms `diag://state` if Diagnostics tab open.
- **Fix:** Add `log::warn!("diag bus lagged: {n} events dropped")` after `record_bus_lag(n)`.

## 227. `scrub_log_message` misses `BEGIN ED25519 PRIVATE KEY` (and DSA)
- **Where:** [diagnostics/mod.rs:347-350](../src-tauri/src/diagnostics/mod.rs#L347-L350)
- **Symptom:** Three PEM headers guarded; Ed25519 (PKCS#8) + DSA pass through to renderer unredacted. Real risk b/c Ed25519 is most common SSH key type now.
- **Fix:** Add `|| out.contains("BEGIN ED25519 PRIVATE KEY") || out.contains("BEGIN DSA PRIVATE KEY")`, OR single regex `BEGIN .* PRIVATE KEY`.

## 228. `dialog:default` capability + plugin registered, never called from frontend
- **Where:** [capabilities/default.json:13](../src-tauri/capabilities/default.json#L13) + [lib.rs:1758](../src-tauri/src/lib.rs#L1758) + [Cargo.toml:20](../src-tauri/Cargo.toml#L20)
- **Symptom:** Native OS dialog plugin exposed to renderer with zero usage — XSS payload could call into OS dialogs. All dialogs are custom Svelte components.
- **Fix:** Remove `tauri_plugin_dialog::init()`, `"dialog:default"` cap entry, and `tauri-plugin-dialog` Cargo dep. Run `cargo check`.

## 229. `opener:default` too broad — only `openUrl`/`openPath`/`revealItemInDir` used
- **Where:** [capabilities/default.json:12](../src-tauri/capabilities/default.json#L12)
- **Symptom:** Full opener plugin surface granted; only 3 functions in actual use across 6 components.
- **Fix:** Replace `"opener:default"` with explicit `"opener:allow-open-url"`, `"opener:allow-open-path"`, `"opener:allow-reveal-item-in-dir"`. Complements #31.

## 230. `core:default` bundles unused `core:path`, `core:app`, `core:menu`, `core:resources`
- **Where:** [capabilities/default.json:7](../src-tauri/capabilities/default.json#L7)
- **Symptom:** Zero frontend imports of `@tauri-apps/api/path` or `@tauri-apps/api/app`; no menu/tray API usage.
- **Fix:** Enumerate `core:default` expansion; pin to explicit `core:event:default` + `core:window:default` only. Drops 4 unused sub-caps. Extends #30.

## 231. Cargo.toml version regex not anchored to `[package]` section
- **Where:** [scripts/release.ps1:34](../scripts/release.ps1#L34), [scripts/bump.ps1:57-63](../scripts/bump.ps1#L57-L63)
- **Symptom:** First `version = "..."` in Cargo.toml is currently the package — fragile to layout change. Workspace merge or dep-block-before-`[package]` would silently write the wrong field.
- **Fix:** Anchor pattern: `(?ms)\[package\].*?^\s*version\s*=\s*"([^"]+)"`.

## 232. `vpk upload github` missing `--channel` — implicit `win` default coupling
- **Where:** [scripts/release.ps1:166-178](../scripts/release.ps1#L166-L178) + [update_service.rs:173](../src-tauri/src/update_service.rs#L173) `UpdateManager::new(src, None, None)`
- **Symptom:** Both sides default to `win` but neither documents it. Multi-channel rollout would silently diverge w/o compile- or runtime-error.
- **Fix:** Explicit `--channel win` to vpk + `Some("win")` to `UpdateManager::new`.

## 233. `Releases/` blanket-ignored but `assets.win.json` + `releases.win.json` may be in working tree
- **Where:** [.gitignore:1](../.gitignore#L1) + `Releases/*.json` artifacts
- **Symptom:** Local pack regenerates feed files; if accidentally shipped from local rather than GitHub-uploaded, clients hit stale packages.
- **Fix:** Verify `git ls-files Releases/` empty; add comment in release.ps1 that local files are pack-state only, canonical feed is GitHub asset.

## 234. `mutateStreaming` O(n) message scan + array realloc per streaming token
- **Where:** [assistant.svelte.ts:580](../src/lib/state/assistant.svelte.ts#L580)
- **Symptom:** Hundreds of tokens/sec during streaming → hundreds of full message-array `.map(...)` + replace per turn. Frame drops in long convos. (Re-cite of Wave 2 #146 with HIGH-severity bump.)
- **Fix:** Cache `streamingMsgIdx` on `send()`, direct `messages[idx] = fn(messages[idx])`; reset in `onDone`.

## 235. `compute_sha1` blocks tokio executor in flush + rebaseline paths

> **SHIPPED v0.4.15-alpha (uncommitted)** — three sites wrapped in `tokio::task::spawn_blocking`: `flush.rs:471` (phantom-conflict SHA-equality check), `flush.rs:581` (post-upload baseline SHA capture), and `auto_sync.rs` rebaseline pass — entire `for (rel, ...) in &remote_rel { ... compute_sha1 ... }` loop moved into a single `spawn_blocking` closure that returns `(new_entries, local_only)`. SHA work no longer pins executor threads during flush or full-tree rebaseline.

- **Where:** [auto_sync/flush.rs:471](../src-tauri/src/sync/auto_sync/flush.rs#L471), [auto_sync.rs:1290-1308](../src-tauri/src/sync/auto_sync.rs#L1290-L1308)
- **Symptom:** `std::fs::File` + `BufReader::read` loop inside `process_entry_body` (a `tokio::spawn` future). Per-file SHA1 up to 64MiB blocks the executor thread.
- **Fix:** Wrap in `tokio::task::spawn_blocking(|| SyncSnapshot::compute_sha1(path)).await`. Same fix for rebaseline pass — move SHA inside the existing `spawn_blocking` closure.

## 236. N+1 SFTP downloads in `lock_presence::poll_once` + heartbeat

> **SHIPPED v0.4.15-alpha (uncommitted) — parallelize subset.** Both serial loops fanned out via `FuturesUnordered`: `refresh_my_locks` heartbeat upload (`lock_presence.rs:265-283`) and `poll_once` per-entry `try_read_lock` read (`:304-337`). 10 locks now complete in ~1 RTT wall-time instead of 10× serial. Post-processing (stale-delete + push into `found`) kept serial after each future resolves — simpler than racing with the delete side-effect. `try_read_lock` tmpdir thrash at `:353-364` left as-is (separate concern from the N+1 fan-out — `cat`-exec replacement deferred since `shell_quote` is `pub(super)` and a `cat_remote` helper on `SftpClient` is out of scope here).

- **Where:** [lock_presence.rs:304-337](../src-tauri/src/sync/lock_presence.rs#L304-L337), [:265-283](../src-tauri/src/sync/lock_presence.rs#L265-L283), [:353-364](../src-tauri/src/sync/lock_presence.rs#L353-L364)
- **Symptom:** Per-lock `try_read_lock` is serial: `download_file` → tmpdir → read → cleanup. 10 locks × 30-60ms Tailscale RTT = ~300ms blocked. Plus per-poll temp-dir churn.
- **Fix:** Replace `download_file` w/ `sftp.exec("cat {path}")` (avoid tmpdir), parallelize via `FuturesUnordered`. Same fan-out for heartbeat upload loop.

### LOW (28)

## 237. `thinking_effort` raw value log-injection vector
- **Where:** [assistant/mod.rs:1041-1043,1273-1276](../src-tauri/src/assistant/mod.rs#L1041-L1043)
- **Symptom:** Newlines/ANSI in renderer-supplied string land in log stream unescaped (CLI arg itself safe — normalized).
- **Fix:** Log `level` post-normalize, not raw `effort`.

## 238. `scrubUser` concrete Rust-side gaps in DiagBus emit
- **Where:** [auto_sync/watch.rs:71](../src-tauri/src/sync/auto_sync/watch.rs#L71); [diagnostics/mod.rs](../src-tauri/src/diagnostics/mod.rs)
- **Symptom:** Concrete site for #8 — `watch refused (ignored path): C:\Users\<username>\...` flows through DiagBus unredacted.
- **Fix:** Apply Rust-side `scrub_user()` in `LogForwarder` or at DiagBus emit. Completes #8.

## 239. `SESSION_PIDS`/`SESSION_STOPPED` `.lock().ok()` — re-confirmed
- **Where:** [assistant/mod.rs:44,51](../src-tauri/src/assistant/mod.rs#L44)
- **Note:** Dup of #63; T+U confirmed STILL present in v0.4.13. Promote priority.

## 240. `aborted_shrunk()` mutex-poison silently returns empty vec
- **Where:** [auto_sync.rs:1227](../src-tauri/src/sync/auto_sync.rs#L1227)
- **Symptom:** Poison → empty vec → rebaseline banner never shown.
- **Fix:** `.unwrap_or_else(|p| { log::error!(...); p.into_inner() })`.

## 241. MCP bridge socket `set_read_timeout`/`set_write_timeout` swallowed
- **Where:** [mcp_server.rs:323,324,411,412](../src-tauri/src/assistant/mcp_server.rs#L323)
- **Symptom:** `let _ = stream.set_read_timeout(...)` → hung remote_bash blocks stdio thread indefinitely.
- **Fix:** Log warn on failure at minimum.

## 242. MCP bridge `stream.flush().ok()` drops flush errors
- **Where:** [mcp_server.rs:328,418](../src-tauri/src/assistant/mcp_server.rs#L328)
- **Symptom:** Broken pipe → followup `read_line` blocks → misleading "bridge closed without response" instead of true write failure.
- **Fix:** Propagate via `.map_err(|e| format!("bridge flush: {e}"))?`.

## 243. STT `serde_json::from_slice(&bytes).unwrap_or_default()` accepts corrupt config
- **Where:** [stt/mod.rs:77](../src-tauri/src/stt/mod.rs#L77)
- **Symptom:** Partial-write/crash → config silently wiped to defaults; API key + model selection lost w/o warning.
- **Fix:** `.unwrap_or_else(|e| { log::warn!("stt-config parse failed ({e}), using defaults"); default })`.

## 244. `edit_trail.rs read_raw .ok()?` destroys trail history on SFTP error
- **Where:** [edit_trail.rs:81](../src-tauri/src/sync/edit_trail.rs#L81)
- **Symptom:** Download error → returns None → `append` overwrites file with only the new entry, silently destroying all prior trail history.
- **Fix:** Distinguish "not found" (normal first-write) from "download failed" (skip write cycle).

## 245. Terminal PTY session ID `unwrap_or(0)` → duplicate-key on clock skew
- **Where:** [terminal/mod.rs:186](../src-tauri/src/terminal/mod.rs#L186)
- **Symptom:** Pre-1970 clock skew (VM, embedded) yields `"term-0"` deterministic key; second spawn overwrites first → leaked PTY + reader thread.
- **Fix:** Use `AtomicU64` counter, or random fallback. Add existence check before insert.

## 246. Rate-limit critical bypass has no secondary ceiling
- **Where:** [diagnostics/mod.rs:415-425](../src-tauri/src/diagnostics/mod.rs#L415-L425)
- **Symptom:** 7 critical event types bypass 200/s cap. Pathological `RemoteScanResult` loop floods Svelte reactivity.
- **Fix:** Secondary `critical_emitted` counter cap (50/s), or document invariant.

## 247. No tracing spans on hot paths (flush, scan, SFTP)
- **Where:** [auto_sync/flush.rs:35](../src-tauri/src/sync/auto_sync/flush.rs#L35), [drift_scanner.rs:122](../src-tauri/src/sync/drift_scanner.rs#L122), [sftp/transfer.rs:20](../src-tauri/src/sftp/transfer.rs#L20)
- **Symptom:** No structured timing or hierarchical causality. Only `latency_ms` in `log_activity_rich`.
- **Fix:** Short-term: entry/exit `log::debug!` w/ timing. Long-term: `tracing` + `tracing-log` bridge.

## 248. Frontend connection errors never reach diag bus
- **Where:** [connection.svelte.ts:230,475](../src/lib/state/connection.svelte.ts#L230)
- **Symptom:** `connect failed` + `auto-reconnect failed` only `console.error`'d → Diagnostics panel never shows them.
- **Fix:** Add `diag_log_frontend_error` Tauri cmd publishing `DiagStage::System` / `DiagLevel::Error`.

## 249. `diag_state_pump` emits every 500ms regardless of subscribers
- **Where:** [lib.rs:343,386](../src-tauri/src/lib.rs#L343)
- **Symptom:** Persistent background serialization+emit even when Diagnostics tab closed.
- **Fix:** `diag_active: AtomicBool` toggled by subscribe/unsubscribe; OR pull model via `diag_get_state`.

## 250. STT console.debug calls — #22 partial regression
- **Where:** [stt.svelte.ts:104,216,280](../src/lib/state/stt.svelte.ts#L104)
- **Symptom:** ISSUES #22 listed `:104,:202,:266` as shipped. Current lines `104,216,280` suggest fix missed or line numbers shifted.
- **Fix:** Verify; remove or gate on `dev` flag.

## 251. Release staging copy hardcoded to 2 files — DLL/redistributable gap
- **Where:** [scripts/release.ps1:126-130](../scripts/release.ps1#L126-L130)
- **Symptom:** Only `rift-tauri.exe` + `icon.ico` copied. Future Tauri/WebView2 redistributables would be silently absent from `vpk pack` payload.
- **Fix:** Comment intent explicitly, OR add `*.dll` glob.

## 252. `GithubSource::get_release_feed` per_page=10 — pagination gap
- **Where:** [update_service.rs:229](../src-tauri/src/update_service.rs#L229)
- **Symptom:** `Releases/` has 14 tags already; if newest eligible doesn't carry `releases.win.json`, walker errors out instead of paginating.
- **Fix:** `per_page=50` + pagination loop following `Link: <url>; rel="next"`.

## 253. `release.ps1` `Read-Host` silently exits 1 in CI pipe (no TTY)
- **Where:** [scripts/release.ps1:86-92](../scripts/release.ps1#L86-L92)
- **Symptom:** Non-TTY stdin returns empty string → exit 1 w/ no log explaining why.
- **Fix:** Replace w/ `Write-Host` + `-Force` parameter; explicit error message.

## 254. `rendered` $derived.by in ActivityFeed — full O(n) regroup on every event
- **Where:** [ActivityFeed.svelte:218](../src/lib/components/activity/ActivityFeed.svelte#L218)
- **Symptom:** During sync burst, full re-derivation chain triggers every event.
- **Fix:** Stable base + burst accumulator; or debounce $derived via setTimeout batching.

## 255. `DriftSummaryCard` $derived.by — O(entries) Map rebuild per drift event
- **Where:** [DriftSummaryCard.svelte:18](../src/lib/components/sync/DriftSummaryCard.svelte#L18)
- **Fix:** Move group-by into store as derived field; component reads pre-aggregated.

## 256. `Diagnostics.svelte` two O(n) linear scans on every event push
- **Where:** [Diagnostics.svelte:127-128](../src/lib/components/diagnostics/Diagnostics.svelte#L127-L128)
- **Symptom:** `find(e => e.stage === "...")` from front; matching event always near tail.
- **Fix:** `findLast`, or maintain `Map<stage, at>` index in store.

## 257. `selBreakdown` rebuilds Map even when selection unchanged
- **Where:** [SyncPage.svelte:117](../src/lib/components/sync/SyncPage.svelte#L117)
- **Symptom:** Mixed dependency (`entries` + `selected`) → over-invalidates on entry stream-in.
- **Fix:** Memoize `byPath` map in store (changes only on entries).

## 258. 39 `format_push_string` clippy hits in MCP server
- **Where:** [mcp_server.rs:185](../src-tauri/src/assistant/mcp_server.rs#L185) (representative)
- **Symptom:** `out.push_str(&format!(...))` allocates intermediate `String` each call. 39 sites in tight loops.
- **Fix:** `write!(out, "...", ...)` from `std::fmt::Write` — zero intermediate alloc.

## 259. `compute_sha1` in drift scanner sequential per file (SSH exec round-trip)
- **Where:** [drift_scanner.rs:384,422,445](../src-tauri/src/sync/drift_scanner.rs#L384)
- **Symptom:** `get_remote_sha1` calls SSH exec serially within scan loop. Up to 10×RTT wall time per scan.
- **Fix:** Collect into batch; `FuturesUnordered` bounded by `hash_budget`.

## 260. Dead function `scanAgeLabel()`
- **Where:** [SyncPage.svelte:201](../src/lib/components/sync/SyncPage.svelte#L201)
- **Fix:** Delete L201-207.

## 261. setTimeout handles not stored in AssistantHeader pulse, ActivityFeed flash, MessageBubble copy
- **Where:** [AssistantHeader.svelte:43](../src/lib/components/assistant/AssistantHeader.svelte#L43), [ActivityFeed.svelte:362](../src/lib/components/activity/ActivityFeed.svelte#L362), [MessageBubble.svelte:242](../src/lib/components/assistant/MessageBubble.svelte#L242)
- **Note:** Re-confirmation of #159, #168 — plus new MessageBubble copy timer.
- **Fix:** Store handles; clear in `$effect` return / `onDestroy`. Pairs with existing Wave 2 entries.

## 262. SyncPage / Settings / Diagnostics use `onMount/onDestroy` instead of `$effect` (HMR-unsafe)
- **Where:** [SyncPage.svelte:66-67](../src/lib/components/sync/SyncPage.svelte#L66-L67), [Settings.svelte:43](../src/lib/components/settings/Settings.svelte#L43), [Diagnostics.svelte:39](../src/lib/components/diagnostics/Diagnostics.svelte#L39)
- **Symptom:** Lifecycle-correct but inconsistent w/ adopted `$effect`-return pattern. Three sites left.
- **Fix:** Migrate to `$effect(() => { ...add...; return () => ...remove... })`.

## 263. `UpdateStore` listeners are intentional singletons but undocumented
- **Where:** [updates.svelte.ts:171](../src/lib/state/updates.svelte.ts#L171)
- **Fix:** Add comment: `// intentional: app-lifetime singleton`.

## 264. `deleteThresholdHint()` single-use helper — inline candidate (INFO)
- **Where:** [SyncPage.svelte:195](../src/lib/components/sync/SyncPage.svelte#L195)
- **Fix:** Leave for readability or inline. No action required.

### Z — Test gap plan (single tracker entry)

## 265. Test strategy + priority ranking
- **Where:** `state/audit-2026-05-20/Z-test-gap.md` (full plan)
- **Symptom:** ISSUES #21 said "zero tests"; reality: 35 `#[test]` fns in 10 files (ignore.rs 14, auto_sync/path.rs 6, bootstrap 4, others). Uncovered HIGH-risk: `drift_scanner.rs`, `state/sync_snapshot.rs`, `sftp/transfer.rs`, `auto_sync/flush.rs`, `assistant/remote_bridge.rs`, `assistant/mod.rs`, `assistant.svelte.ts`, `sync-page.svelte.ts`, `path_guard.rs`, `drift_watcher.rs`.
- **Structural blockers:** No `SftpOps` trait (concrete `SftpClient`); `AutoSyncEngine` needs `AppHandle`; `SyncSnapshot::new` writes to `~/.rift/`; `BRIDGE: OnceLock` set-once. Recommended sequencing: Wave A (no infra) → B (`SyncSnapshot::for_path` + `tempfile`) → C (`SftpOps` trait + mock SFTP) → D (`DiagSink` trait + Tauri `mock_builder`).
- **Fix sketch:** Follow Wave A first — `path_guard` containment, `sync_snapshot` serialization, flush circuit-breaker math, `sync-page.svelte.ts` bucket display, `assistant.svelte.ts` usage accumulator. Updates #21.

### AA top-10 clippy perf lints (table for triage)

| # | Rule | Count | Representative | Fix |
|---|------|-------|----------------|-----|
| 1 | `format_push_string` | 39 | [mcp_server.rs:185](../src-tauri/src/assistant/mcp_server.rs#L185) | `write!(out, ...)` (see #258) |
| 2 | `redundant_closure_for_method_calls` | ~12 | [mcp_server.rs:205](../src-tauri/src/assistant/mcp_server.rs#L205) | Method-ref instead of closure |
| 3 | `map_unwrap_or` | 3 | [mcp_server.rs:174](../src-tauri/src/assistant/mcp_server.rs#L174) | `map_or(default, f)` |
| 4 | `needless_pass_by_value` | 3 | [assistant/mod.rs:447](../src-tauri/src/assistant/mod.rs#L447) | `&str` over `String` |
| 5 | `uninlined_format_args` | ~8 | [mcp_server.rs:188](../src-tauri/src/assistant/mcp_server.rs#L188) | Inline vars in format str |
| 6 | `unnecessary_sort_by` | 1 | [assistant/mod.rs:442](../src-tauri/src/assistant/mod.rs#L442) | `sort_by_key(\|b\| Reverse(...))` |
| 7 | `map_unwrap_or` (Option) | 1 | [assistant/mod.rs:428](../src-tauri/src/assistant/mod.rs#L428) | `map_or` |
| 8 | `redundant_else` | 1 | [drift_scanner.rs:245](../src-tauri/src/sync/drift_scanner.rs#L245) | Flatten else |
| 9 | `manual_let_else` | ~8 | [mcp_server.rs:164](../src-tauri/src/assistant/mcp_server.rs#L164) | `let Ok(x) = ... else { ... }` |
| 10 | `match_same_arms` | 1 | [mcp_server.rs:642](../src-tauri/src/assistant/mcp_server.rs#L642) | Merge arms |

---

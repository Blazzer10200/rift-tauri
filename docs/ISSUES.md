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

> **SHIPPED 2026-05-19 (uncommitted)** — per-turn semantics kept; `lastTurnUsage` now only updates on the `result` event, not on `envelope`. Pill sits on the previous turn's confirmed value through the in-flight turn and lands on the new value once result arrives -- single update per turn, no visible jump on complex turns where envelope+result diverge. S106 telemetry capture (`envelopeUsage` vs `resultUsage` on `currentTurnRecord`) is preserved so divergence metrics keep their signal. Verified clean via `npm run check` (0/0).

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

> **#3b SHIPPED 2026-05-19 (uncommitted)** — `stt.svelte.ts:131-149` `consume()` now calls `recognition.abort()` + resets `recording/transcribing/recognition`/timer. Likely also resolves #3c — verify next runtime. #3a (accuracy) remains.

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

> **9.1 + 9.2 SHIPPED 2026-05-19 (uncommitted).**
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

> **SHIPPED 2026-05-19 (uncommitted)** — new `require_pinned_fingerprint(server_key, Option<&str>)` helper in lib.rs returns an actionable error string. Guard wired into three entry points: `scan_drift`, `start_autosync`, and (most importantly) `open_sftp_for` — the latter funnels ~9 IPC commands (remote_list_dir, upload/download, edit_in_place, sync_*, detect_bootstrap, etc.) so the single guard there closes the silent-TOFU window for all of them. Dead `persist_fingerprint_if_new` function removed (only sanctioned trust path is now `probe_server_fingerprint` → user confirm → `set_server_fingerprint`). Verified via CDP — autosync correctly transitions to `watching` for the pinned-fingerprint server.

- **Where:** [src-tauri/src/lib.rs:393](../src-tauri/src/lib.rs#L393) (sync scan), [src-tauri/src/lib.rs:452](../src-tauri/src/lib.rs#L452) (`start_autosync`). Both contain: `if server.fingerprint.as_deref().unwrap_or("").is_empty() { persist_fingerprint_if_new(...) }` — empty fingerprint → silent accept-and-pin.
- **Symptom:** First connection to a new server accepts ANY host key without user confirmation. MITM-during-onboarding risk.
- **Confirmation flow already exists:** [src-tauri/src/lib.rs:1494](../src-tauri/src/lib.rs#L1494) `probe_server_fingerprint` IPC is used by the AddServer dialog — but sync entry paths bypass it.
- **Post-pin enforcement is strict:** Mismatched fingerprint correctly hard-rejects (`ssh_handler.rs:52-55`).
- **Fix sketch:** Route all first-connect fingerprint capture through `probe_server_fingerprint`. Don't allow sync entry paths to silently TOFU. AddServer dialog already does it right — reuse.

## 11. Settings page dead UI cluster

> **PARTIAL — SHIPPED 2026-05-19 (uncommitted):** removed unused `uiPrefs` import (L11), added `Palette` to lucide imports and assigned it to Appearance (kills the Sparkles dup), dropped dead `lg` modifier on `srv-dot` (L941), descriptive aria-labels on the three terminal toggles (`Blink cursor` / `Copy on select` / `Right-click paste`). **Remaining:** Appearance "More coming soon" placeholder (L246-251), empty SSH Keys section (L964-975), STT-namespaced font-picker class rename (L736-758) — all non-mechanical, deferred.

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

> **SHIPPED 2026-05-19 (uncommitted)** — `scripts/bump.ps1` accepts a semver arg, regex-replaces the first version line in all three files, post-bump cross-checks all three match. Patterns dry-ran clean (1 match each, current version). Usage: `pwsh ./scripts/bump.ps1 0.4.12-alpha`. Em-dashes replaced w/ `--` to avoid PS5.1 BOM-loss mojibake on future Edits.

- **Where:** [package.json](../package.json), [src-tauri/Cargo.toml](../src-tauri/Cargo.toml), [src-tauri/tauri.conf.json](../src-tauri/tauri.conf.json) — all three must match. [scripts/release.ps1:28-37](../scripts/release.ps1#L28-L37) detects mismatch and bails.
- **Symptom:** Per CLAUDE.md gotcha #5 + HANDOFF history, this is the #1 ship-attempt failure mode. v0.2.49's first ship attempt died here. The script catches the mistake but doesn't fix it.
- **Fix sketch:** `scripts/bump.ps1 <new-version>` that writes all three files in one shot (~25 lines of PowerShell). Optionally appends a CHANGELOG `## <version> — <date>` stub w/ blank `### Added / Changed / Fixed` headers. Optionally `release.ps1`'s preflight can offer to auto-bump on mismatch w/ a `-AutoBump` flag.

## 13. Release notes never flow into the GitHub release

> **SHIPPED 2026-05-19 (uncommitted)** — `release.ps1` now extracts the top `## v<version>` entry body from `docs/CHANGELOG.md` (only when the entry version matches the bumped version, else warns and skips), writes to `$env:TEMP/rift-release-notes-<version>.md`, passes `--releaseNotes` to `vpk pack` (verified flag — `vpk upload github` has no such flag), cleans up on success. Dry-run extracted v0.4.11-alpha body cleanly (2155 chars).

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

> **RESOLVED 2026-05-19 (uncommitted)** — verified [lib.rs:1554-1573](../src-tauri/src/lib.rs#L1554-L1573) `apply_updates` already stops autosync (engine.stop) + tunnel (t.stop) BEFORE `spawn_blocking(|| UpdateService::new().apply())`. The audit re-read mistook the boundary — frontend correctly does NOT stop anything; the Tauri command layer owns it. Tightened doc comment at `update_service.rs:82-88` to make explicit that direct callers of `UpdateService::apply` must do their own stop, but the `apply_updates` command handles it.

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

**Tier 0 — verify before anything else**
- ~~#19 `apply_updates` autosync-stop~~ — verified 2026-05-19, non-bug (Tauri cmd already handles it; doc tightened)

**Tier 1 — ship blockers / data safety**
- #21 Zero test coverage
- #9 Bridge token plaintext + IPC leak — 9.1+9.2 SHIPPED 2026-05-19; 9.3 (OS keyring) deferred to Phase 6
- #15 Unsigned Windows builds (adoption blocker)
- ~~#10 Silent TOFU on first sync~~ — SHIPPED 2026-05-19 (`require_pinned_fingerprint` guard)

**Tier 2 — recurring friction**
- ~~#12 Manual 3-file version bump~~ — shipped 2026-05-19 (`scripts/bump.ps1`)
- ~~#3b STT send doesn't stop recognizer~~ — shipped 2026-05-19 (`stt.svelte.ts` `consume()`)
- ~~#1 Context counter semantics + double-write~~ — shipped 2026-05-19 (per-turn, result-only render)
- #14 No CI (deferred — pairs w/ #15 signing)

**Tier 3 — UX + cleanup**
- #11 Settings dead UI cluster — PARTIAL shipped 2026-05-19 (mechanical bits done; placeholder card + SSH Keys empty + font-picker class rename remain)
- #2 Tool-block rendering rhythm
- #22 Console noise
- #6 Scrollbar collision
- #5 Status indicator placement
- ~~#13 Release notes auto-flow~~ — shipped 2026-05-19 (`release.ps1` `--releaseNotes`)

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

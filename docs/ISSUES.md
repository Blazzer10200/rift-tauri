# Rift — Issue Tracker

> Single source of truth for **open work only**. When something ships, **delete the block** — `git log -- docs/ISSUES.md` preserves history. Each block carries `Where` (file:line, may have drifted — re-grep before acting), `Symptom`, optional `Fix sketch`. Issue IDs are durable — never re-number, only append.
>
> Shipped Wave-1/2/3 audit blocks + clippy tables live in `docs/archive/audit-history.md`. Pruned 2026-06-04: the pure-assistant conversion (2026-06-03) removed the SFTP/sync/server/RCON stack, so every issue scoped to those subsystems was deleted here (history via `git log`).

---

## Active work — current sprint

> Live queue. HANDOFF.md = session state; this section = what's queued.

- **Steer feature — live-verify on a tool-using turn.** Mid-turn message injection shipped end-to-end (`assistant_steer` command, `STEER_TX` registry, `tokio::select!` reader, Alt+Enter trigger; brief in `docs/design/steer-and-queue.md`). Verified: compiles, `npm run check` clean, live CDP test accepted a mid-stream steer (`steer=steered`). Remaining: confirm a *visible* mid-turn redirect on a multi-step tool turn through the UI (pure-text turns complete before the steer lands — by design).
- **Permission round-trip — code-complete, needs live-verify.** Wired end-to-end: `--permission-prompt-tool stdio` (mod.rs) → `can_use_tool` handler → control-response write → `PermissionBar.svelte` Allow/Deny UI → `submitPermissionDecision()`. Remaining: live-verify with a throwaway repo — a git-write op in default/acceptEdits/plan mode should surface the Allow/Deny bar.
- **CR-UX (DECISION PENDING — user)** Trust segment is binary (Read-only/Standard) over a **ternary** backend enum (`readonly/standard/full`). Once clicked, `trust_level` pins and can't return to the derived state via UI; "full" (rank 2) is functionally identical to "standard" — only `"standard"` is gated for git writes. **Recommendation: collapse to a true 2-level enum** (drop dead "full"). Touches `mcp_server::trust_rank`/`trust_level`, `mod.rs::is_valid_trust_level`/`effective_trust_level`/git-write gate, serde, + persisted config migration. Held for user sign-off — security-relevant + persisted-config change.

### Active design briefs
- `docs/design/assistant-svelte-split.md` (#20 — M0-M7 shipped; M8 streaming + M9 send open)
- `docs/design/steer-and-queue.md` (steer/queue three-tier model — steer shipped; queue improvements + inline-bubble follow-ups open)

---

## 4. UI/UX consistency + navigability sweep (app-wide)

- **Scope:** Not a single bug — tracking the user's stated goal of an app-wide consistency pass. The Settings page is the densest control surface and the natural starting point.
- **Goal:** Every visible control is wired, every section is necessary, terminology + styling consistent. Navigation is intuitive.
- **Approach when actioned:** Per-page audit checklist (control → wired? necessary? consistent?). [src/lib/components/settings/SettingsPage.svelte](../src/lib/components/settings/SettingsPage.svelte) ~1064L (gutted of the old Server/RCON/SSH sections in the pure-assistant conversion) — audit still non-trivial.

## 14. No CI — release path local-only (CLOSED — by choice)

- `.github/workflows/check.yml` SHIPPED (cargo + svelte-check on PR). Release CI is **not being pursued** — it only made sense bundled with code-signing, which was **declined 2026-05-29** (SmartScreen friction not worth a recurring fee for a self-distributed alpha). Releases stay local via `scripts/release.ps1`. Reopen only if signing is reconsidered.

## 17. Two-repo split — historic, low-priority collapse

- **Where:** [scripts/release.ps1](../scripts/release.ps1) publishes to `Blazzer10200/rift-releases`; [src-tauri/src/update_service.rs](../src-tauri/src/update_service.rs) points Velopack's `GithubSource` at the same public repo.
- **Symptom:** Every release requires manual sync between the private source repo and the public releases repo. Forks/contributors can't test the update path against the real source.
- **Fix sketch:** Collapse to a single repo if the source repo goes public — a small change in `release.ps1` + the update source constant.

## 20. Hot files exceeding the 2000-line agent-split threshold

- **Where:** Per CLAUDE.md agent-routing guidance, files >2000 lines are agent-bail risks. Open targets (re-measured 2026-06-04):
  - [src-tauri/src/assistant/mod.rs](../src-tauri/src/assistant/mod.rs) — **~3331L (worst)**: Claude CLI spawn + auth + workspace + config + per-turn stream. Next backend split candidate.
  - [src/lib/state/assistant.svelte.ts](../src/lib/state/assistant.svelte.ts) — ~2479L (M0-M7 carved from 3356L; M8/M9 open).
- **Symptom:** Targeted edits become brittle, LSP slows, agents bail mid-emit on audit-shaped prompts.
- **Fix sketch:** `assistant.svelte.ts` next — design brief in [docs/design/assistant-svelte-split.md](design/assistant-svelte-split.md) (9-module extraction, ranked by blast radius). Then continue `assistant/mod.rs` extraction.
- **Status:** M0-M7 SHIPPED (`assistant.svelte.ts` 3356L → ~2479L). M8 (streaming pump) + M9 (send orchestrator) still open — the two highest-blast-radius extractions; deferred until a conversation-playback test harness exists.

## 21. Test coverage — thin after the pure-assistant rip

- **Where (2026-06-04):** ~9 Rust tests remain (`assistant/git_local.rs`, `stt/vad.rs`, `stt/whisper.rs`) + 1 vitest file (`src/lib/state/assistant.test.ts`, mocks Tauri IPC over the assistant store). The former 115-test lib suite + 7 live-SFTP `#[ignore]` integration tests + the `DriftScanner`/`SftpOps` mock layer were all removed with the sync engine.
- **Symptom:** The surviving high-risk surface — the per-turn stream/reader in `assistant/mod.rs` and the store orchestrator in `assistant.svelte.ts` — has no end-to-end coverage. A regression in the stream pump or the send/queue/steer path can break a turn silently.
- **Fix sketch:** Build a conversation-playback harness (feed recorded NDJSON frames through the reader + store) — also the unblocker for the #20 M8/M9 extractions. Then cover the git_local MCP tools against a throwaway repo.

## 29. CSP allows `style-src 'unsafe-inline'` (LOW)

- **Where:** [src-tauri/tauri.conf.json](../src-tauri/tauri.conf.json) `csp`.
- **Symptom:** Inline styles permitted — required by current Tailwind output, weakens CSP.
- **Fix sketch:** Switch to nonce/strict-dynamic once Tailwind supports hashed inline styles end-to-end.

---

## Priority tiers

**Tier 1 — ship blockers / data safety**
- #21 Test coverage — surviving stream/store paths uncovered after the rip.

**Tier 2 — needs live-verify (code-complete)**
- Steer mid-turn redirect on a tool turn · Permission round-trip Allow/Deny bar.

**Tier 3 — strategic / longer-term**
- #4 App-wide UX consistency sweep · #20 hot-file split M8-M9 · #17 two-repo collapse · CR-UX trust-enum decision (user sign-off).

**Tier 4 — backend LOW (opportunistic)**
- #29 CSP `style-src 'unsafe-inline'` (Tailwind-blocked) · Wave-1 LOWs #91-#134 — clippy/doc/perf nits (see `docs/archive/audit-history.md`).

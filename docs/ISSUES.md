# Rift — Issue Tracker

> **Single source of truth for open work only.** When something ships, **delete its block** — `git log -- docs/ISSUES.md` preserves history. Issue IDs are **durable**: never re-number, only append. Each block carries `Where` (file:line — may have drifted, re-grep before acting), `Symptom`, and an optional `Fix sketch`.
>
> Shipped Wave-1/2/3 audit blocks + clippy tables live in `docs/archive/audit-history.md`. Pruned 2026-06-04: the pure-assistant conversion (2026-06-03) removed the SFTP/sync/server/RCON stack, so every issue scoped to those subsystems was deleted here (history via `git log`).

### Conventions

- **Status** — `✅ resolved in-tree` (fixed, unshipped — block stays until `/git-ship` so `git log` keeps it) · `🚧 open` · `👤 needs your call` · `🧪 live-verify` (code-complete, needs runtime confirmation) · `🔒 blocked` (external dep) · `🗄 closed` (decided, kept for the record).
- **Tier** — `T1` ship-blocker / data-safety · `T2` code-complete needs live-verify · `T3` strategic / longer-term · `T4` LOW / cosmetic.

### Index

| ID | Title | Tier | Status |
|----|-------|------|--------|
| #21 | Test coverage thin after the pure-assistant rip | T1 | 🚧 open |
| Auth-Rec | In-app sign-in recovery for 401 failures | T2 | ✅ resolved in-tree |
| Steer | Mid-turn redirect on a tool-using turn | T2 | 🧪 live-verify |
| Permission | Allow/Deny round-trip bar | T2 | 🧪 live-verify |
| #4 | App-wide UX consistency + navigability sweep | T3 | 🚧 open |
| #20 | Hot files over the 2000-line split threshold | T3 | 🚧 open |
| #17 | Two-repo split → collapse | T3 | 🔒 blocked |
| CR-UX | Trust segment binary-vs-ternary enum | T3 | 👤 needs your call |
| #29 | CSP allows `style-src 'unsafe-inline'` | T4 | 🔒 blocked |
| UI-drift | App-update surfaces disagree (toast vs card) | T4 | 🚧 open |
| #14 | No release CI — local-only path | — | 🗄 closed |

---

## 🚧 Open issues

### Tier 1 — ship-blocker / data-safety

#### 21. Test coverage — thin after the pure-assistant rip

- **Where (re-measured 2026-06-07):** Rust lib suite now **47 tests** (was ~9). Added 2026-06-07 (`756c95b`, `6d3efc2`): 12 `git_local` integration tests (real `git` against throwaway temp repos — status/log/diff/commit + force-push/dirty-pull gates), 14 `mcp_server` tests (was **zero** — `resolve_under_roots` containment, read_file/list_dir/grep incl. SKIP_DIRS+binary+glob, `glob_to_regex`, `trust_rank`), 4 `mod.rs` pure-validator tests (semver/trust/perm-mode/compression). Plus the pre-existing `stt/*`, `swarm`, `usage::pricing` tests + 1 vitest file (`assistant.test.ts`).
- **Symptom (remaining):** the **per-turn stream/reader in `assistant/mod.rs`** and the **store orchestrator in `assistant.svelte.ts`** still have no end-to-end coverage. A regression in the stream pump or the send/queue/steer path can still break a turn silently. (The MCP tool surface + security gates are now covered — that half is done.)
- **Fix sketch (remaining):** build a conversation-playback harness (feed recorded NDJSON frames through the reader + store) — also the unblocker for the #20 M8/M9 extractions. ~~Then cover the git_local MCP tools against a throwaway repo.~~ (done 2026-06-07)

### Tier 2 — code-complete, needs live-verify

#### Auth-Rec — in-app sign-in recovery for 401 failures (✅ resolved in-tree, cont.74)

- **Symptom (from a collaborator's screenshot 2026-06-08):** a 401 banner that dead-ended at "open a terminal and run `claude login`." Root cause: `claude auth status` reports `loggedIn:true` for a stale OAuth token the API later rejects, so the send-gate ([assistant.svelte.ts](../src/lib/state/assistant.svelte.ts) ~1995) passes but the turn 401s.
- **Fix (`9c468a4`+`2d72af8`):** backend `assistant_open_login(console)` spawns `<active claude> auth login` in its own console (creds land in the CLI's shared store → real fix, not just UI); `startLogin()` polls the probe then clears; `recheckAuth()` clears on Re-check; [AssistantPane.svelte](../src/lib/components/assistant/AssistantPane.svelte) renders an actionable banner — [Sign in] (login 401) / [Open Settings] (key 401) / [Re-check]. CDP-verified all states + nav (not the live login spawn).
- **Remaining:** confirm an end-to-end real sign-in on a genuinely-logged-out machine (the dev box stays authed, so the spawn path itself is compile/registration-verified only). **Strategic follow-ups** (not built): proactive re-probe before first send; auto-prefer an authed install when multiple exist; collapse the scattered 401 string-matching into one `AuthError` enum + DiagBus telemetry so failure frequency is measurable.

#### Steer — mid-turn redirect on a tool-using turn

- **Status:** mid-turn message injection shipped end-to-end (`assistant_steer` command, `STEER_TX` registry, `tokio::select!` reader, Alt+Enter trigger; brief in `docs/design/steer-and-queue.md`). Verified: compiles, `npm run check` clean, live CDP test accepted a mid-stream steer (`steer=steered`).
- **Remaining:** confirm a *visible* mid-turn redirect on a multi-step tool turn through the UI (pure-text turns complete before the steer lands — by design).

#### Permission — Allow/Deny round-trip bar

- **Status:** wired end-to-end — `--permission-prompt-tool stdio` (mod.rs) → `can_use_tool` handler → control-response write → `PermissionBar.svelte` Allow/Deny UI → `submitPermissionDecision()`.
- **Remaining:** live-verify with a throwaway repo — a git-write op in default/acceptEdits/plan mode should surface the Allow/Deny bar.

### Tier 3 — strategic / longer-term

#### 4. UI/UX consistency + navigability sweep (app-wide)

- **Scope:** not a single bug — tracks the stated goal of an app-wide consistency pass. The Settings page is the densest control surface and the natural starting point.
- **Goal:** every visible control is wired, every section is necessary, terminology + styling consistent, navigation intuitive.
- **Approach when actioned:** per-page audit checklist (control → wired? necessary? consistent?). [SettingsPage.svelte](../src/lib/components/settings/SettingsPage.svelte) ~1064L (gutted of the old Server/RCON/SSH sections in the pure-assistant conversion) — audit still non-trivial.

#### 20. Hot files exceeding the 2000-line agent-split threshold

- **Where:** per CLAUDE.md agent-routing guidance, files >2000 lines are agent-bail risks. Open targets (re-measured 2026-06-04):
  - [src-tauri/src/assistant/mod.rs](../src-tauri/src/assistant/mod.rs) — **~3331L (worst)**: Claude CLI spawn + auth + workspace + config + per-turn stream. Next backend split candidate.
  - [src/lib/state/assistant.svelte.ts](../src/lib/state/assistant.svelte.ts) — ~2479L (M0-M7 carved from 3356L; M8/M9 open).
- **Symptom:** targeted edits become brittle, LSP slows, agents bail mid-emit on audit-shaped prompts.
- **Fix sketch:** `assistant.svelte.ts` next — design brief in [docs/design/assistant-svelte-split.md](design/assistant-svelte-split.md) (9-module extraction, ranked by blast radius). Then continue `assistant/mod.rs` extraction.
- **Status:** M0-M7 SHIPPED (`assistant.svelte.ts` 3356L → ~2479L). M8 (streaming pump) + M9 (send orchestrator) open — the two highest-blast-radius extractions; deferred until a conversation-playback test harness exists (see #21).

#### 17. Two-repo split — historic, low-priority collapse (🔒 blocked)

- **Where:** [scripts/release.ps1](../scripts/release.ps1) publishes to `Blazzer10200/rift-releases`; [src-tauri/src/update_service.rs](../src-tauri/src/update_service.rs) points Velopack's `GithubSource` at the same public repo.
- **Symptom:** every release requires manual sync between the private source repo and the public releases repo. Forks/contributors can't test the update path against the real source.
- **Fix sketch:** collapse to a single repo **if the source repo goes public** — a small change in `release.ps1` + the update source constant. Blocked on that decision.

#### CR-UX. Trust segment binary-vs-ternary enum (👤 needs your sign-off)

- **Symptom:** the trust segment is binary (Read-only/Standard) over a **ternary** backend enum (`readonly/standard/full`). Once clicked, `trust_level` pins and can't return to the derived state via UI; "full" (rank 2) is functionally identical to "standard" — only `"standard"` is gated for git writes.
- **Recommendation:** collapse to a true 2-level enum (drop the dead "full"). Touches `mcp_server::trust_rank`/`trust_level`, `mod.rs::is_valid_trust_level`/`effective_trust_level`/git-write gate, serde, + persisted-config migration.
- **Held for sign-off:** security-relevant + persisted-config change.

### Tier 4 — LOW / cosmetic

#### 29. CSP allows `style-src 'unsafe-inline'` (🔒 blocked)

- **Where:** [src-tauri/tauri.conf.json](../src-tauri/tauri.conf.json) `csp`.
- **Symptom:** inline styles permitted — required by current Tailwind output, weakens CSP.
- **Fix sketch:** switch to nonce/strict-dynamic once Tailwind supports hashed inline styles end-to-end. Blocked on Tailwind.

#### UI-drift. App-update surfaces disagree (🚧 open)

- **Where:** the Velopack app-update surfaces — toast (`updates.svelte.ts`), Home/Settings status card(s). User screenshot (cont.64) showed the toast saying "update available v0.5.0 → v0.6.1" while another card read "Rift 0.5.0 · up to date" at the same time.
- **Symptom:** the surfaces read different slices of update state, so one can show "available" while another shows "up to date" — looks broken even when the updater is fine. (Functional bug was the apply file-lock, fixed in v0.6.2; this is the remaining *cosmetic* drift.)
- **Fix sketch:** mirror the cont.63 CLI-update unification — drive every app-update surface from one derived summary off the `updates` store so they can't diverge.

> Also parked: **Wave-1 LOWs #91–#134** — clippy/doc/perf nits, in `docs/archive/audit-history.md` (not tracked live here).

---

## 🗄 Closed — kept for the record

### 14. No CI — release path local-only (closed by choice)

- `.github/workflows/check.yml` SHIPPED (cargo + svelte-check on PR). Release CI is **not being pursued** — it only made sense bundled with code-signing, which was **declined 2026-05-29** (SmartScreen friction not worth a recurring fee for a self-distributed alpha). Releases stay local via `scripts/release.ps1`. Reopen only if signing is reconsidered.

---

## Investigated 2026-06-05 — NOT bugs (don't re-chase)

- **`ReferenceError: MessageCircle is not defined` in the console ring** — STALE. The symbol IS imported ([HarnessPage.svelte:4](../src/lib/components/workspaces/HarnessPage.svelte)) and used (~line 537); the error was a transient intermediate-HMR artifact from ~50min before the pass. Navigating to Harness throws nothing now.
- **Model-menu rows "don't switch on click"** — NOT a bug. Rows use `onmousedown` ([Composer.svelte:1209](../src/lib/components/assistant/Composer.svelte)) (fires before blur so the menu doesn't close first); a synthetic `click` simply doesn't trigger them. Real pointer + keyboard both work.

---

## Active design briefs

- `docs/design/assistant-svelte-split.md` (#20 — M0-M7 shipped; M8 streaming + M9 send open)
- `docs/design/steer-and-queue.md` (steer/queue three-tier model — steer shipped; queue improvements + inline-bubble follow-ups open)

---

## Last full-app verification

- **🔍 Full-app CDP stress pass 2026-06-05 (cont.58) — app healthy.** Walked every workspace + dialog live (Home · Chat · Harness · Settings · command palette · History drawer · Web-browser panel · Panels menu). Ran a real read-only backend turn end-to-end (CLI spawn → MCP `grep`/`glob`/`list_dir`/`read_file` → stream → cost/context/activity render — all correct). Stress: 12 rapid workspace switches + a 14.8K-char emoji/unicode/`<script>` composer paste (auto-grew to the 340px cap, inert, no XSS). **Console: 0 errors / 0 warnings the whole session.** Verified live: cont.57 model/effort capability matrix (Haiku hides slider + shows the no-effort caption), #31–#35 fixes, themeable accent incl. amber warm-hue with no oklch purple-wrap, Harness no-scroll + trust-gated git tools. One new defect found → #36 (now resolved). Could NOT live-exercise: #30 update toast/dialog (app up-to-date on v0.5.0 → state never renders) and first-run onboarding (next-launch only).

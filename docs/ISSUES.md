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
| Auth-Rec | In-app sign-in recovery for 401 failures | T2 | 🧪 live-verify |
| Steer | Mid-turn redirect on a tool-using turn | T2 | 🧪 live-verify |
| Rail-v2 | Pending rail v2 — steer chips + mode toggle | T3 | 🚧 open |
| Permission | Allow/Deny round-trip bar | T2 | 🧪 live-verify |
| #4 | App-wide UX consistency + navigability sweep | T3 | 🚧 open |
| #20 | Hot files over the 2000-line split threshold | T3 | 🚧 open |
| #17 | Two-repo split → collapse | T3 | 🔒 blocked |
| CR-UX | Trust segment binary-vs-ternary enum | T3 | 👤 needs your call |
| #29 | CSP nonce nullifies `'unsafe-inline'` — inline styles blocked at runtime | T4 | 🚧 open |
| UI-drift | App-update surfaces disagree (toast vs card) | T4 | ✅ resolved in-tree |
| #14 | No release CI — local-only path | — | 🗄 closed |

---

## 🚧 Open issues

### Tier 2 — code-complete, needs live-verify

#### Auth-Rec — in-app sign-in recovery for 401 failures (🧪 live-verify)

- **Status:** shipped in v0.8.9+ (`9c468a4`+`2d72af8`) — `assistant_open_login(console)` spawn + actionable 401 banner ([Sign in]/[Open Settings]/[Re-check]). CDP-verified all banner states; the live login spawn itself is compile/registration-verified only.
- **Remaining:** confirm an end-to-end real sign-in on a genuinely-logged-out machine (dev box stays authed). **Strategic follow-ups** (not built): proactive re-probe before first send; auto-prefer an authed install when multiple exist; collapse scattered 401 string-matching into one `AuthError` enum + DiagBus telemetry.

#### Steer — mid-turn redirect on a tool-using turn

- **Status:** mid-turn message injection shipped end-to-end (`assistant_steer` command, `STEER_TX` registry, `tokio::select!` reader, Alt+Enter trigger; brief in `docs/design/steer-and-queue.md`). Verified: compiles, `npm run check` clean, live CDP test accepted a mid-stream steer (`steer=steered`).
- **Remaining:** confirm a *visible* mid-turn redirect on a multi-step tool turn through the UI (pure-text turns complete before the steer lands — by design).

#### Permission — Allow/Deny round-trip bar

- **Status:** wired end-to-end — `--permission-prompt-tool stdio` (mod.rs) → `can_use_tool` handler → control-response write → `PermissionBar.svelte` Allow/Deny UI → `submitPermissionDecision()`.
- **Remaining:** live-verify with a throwaway repo — a git-write op in default/acceptEdits/plan mode should surface the Allow/Deny bar.

### Tier 3 — strategic / longer-term

#### Rail-v2 — pending rail v2: steer chips + mode toggle (🚧 open)

- **Scope:** steer chips + per-chip steer/queue mode toggle + pulse-on-inject — unifies the three-tier surface ([steer-and-queue.md](design/steer-and-queue.md) §6 #1). Makes steer discoverable (currently keyboard-only Alt+Enter). v1 rail shipped in v0.8.11.

#### 4. UI/UX consistency + navigability sweep (app-wide)

- **Scope:** not a single bug — tracks the stated goal of an app-wide consistency pass. The Settings page is the densest control surface and the natural starting point.
- **Goal:** every visible control is wired, every section is necessary, terminology + styling consistent, navigation intuitive.
- **Approach when actioned:** per-page audit checklist (control → wired? necessary? consistent?). [SettingsPage.svelte](../src/lib/components/settings/SettingsPage.svelte) ~1064L (gutted of the old Server/RCON/SSH sections in the pure-assistant conversion) — audit still non-trivial.

#### 20. Hot files exceeding the 2000-line agent-split threshold

- **Where:** per CLAUDE.md agent-routing guidance, files >2000 lines are agent-bail risks. Open targets (re-measured 2026-06-09):
  - [src-tauri/src/assistant/mod.rs](../src-tauri/src/assistant/mod.rs) — **2917L** (was 4331L): R1/R3/R4/R5/R7 extracted 2026-06-09 per [docs/design/assistant-mod-split.md](design/assistant-mod-split.md). **Remaining: R2 config · R6 oneshot · R8 turn** (`assistant_send` 917L — last).
  - [src/lib/components/assistant/Composer.svelte](../src/lib/components/assistant/Composer.svelte) — 2957L, next frontend target (needs its own brief).
- **Symptom:** targeted edits become brittle, LSP slows, agents bail mid-emit on audit-shaped prompts.
- **Status:** `assistant.svelte.ts` split **COMPLETE** (M0-M9, now 1700L — was 3356L; playback net held). mod.rs split **5/8 shipped** (`cli_install` · `convo_store` · `auth_update` · `env_checks` · `workspace`), each cargo-check zero-warnings + cargo test 95/95 per commit. Next bite: R2 (config) — biggest import surface, do before R6/R8.

#### 17. Two-repo split — historic, low-priority collapse (🔒 blocked)

- **Where:** [scripts/release.ps1](../scripts/release.ps1) publishes to `Blazzer10200/rift-releases`; [src-tauri/src/update_service.rs](../src-tauri/src/update_service.rs) points Velopack's `GithubSource` at the same public repo.
- **Symptom:** every release requires manual sync between the private source repo and the public releases repo. Forks/contributors can't test the update path against the real source.
- **Fix sketch:** collapse to a single repo **if the source repo goes public** — a small change in `release.ps1` + the update source constant. Blocked on that decision.

#### CR-UX. Trust segment binary-vs-ternary enum (👤 needs your sign-off)

- **Symptom:** the trust segment is binary (Read-only/Standard) over a **ternary** backend enum (`readonly/standard/full`). Once clicked, `trust_level` pins and can't return to the derived state via UI; "full" (rank 2) is functionally identical to "standard" — only `"standard"` is gated for git writes.
- **Recommendation:** collapse to a true 2-level enum (drop the dead "full"). Touches `mcp_server::trust_rank`/`trust_level`, `mod.rs::is_valid_trust_level`/`effective_trust_level`/git-write gate, serde, + persisted-config migration.
- **Held for sign-off:** security-relevant + persisted-config change.

### Tier 4 — LOW / cosmetic

#### 29. CSP nonce nullifies `'unsafe-inline'` — inline styles blocked at runtime (🚧 open)

- **Where:** [src-tauri/tauri.conf.json](../src-tauri/tauri.conf.json) `csp` (`style-src 'self' 'unsafe-inline'`). At runtime SvelteKit injects a `nonce-…` into the served CSP.
- **Symptom (observed v0.8.14, prod CDP):** per CSP spec, **a nonce makes `'unsafe-inline'` be ignored** — so Svelte's dynamically-applied inline styles get blocked. Console spams `Applying inline style violates ... style-src 'self' 'unsafe-inline' 'nonce-…'`. Real impact: Svelte transition styles (fly/fade) and `style="width:{progress}%"` on the update download progress-bar don't apply. **Cosmetic** — download/apply and all clicks still work; animations snap and the progress fill stays empty.
- **Fix sketch:** make the static CSP and SvelteKit's nonce agree. Either (a) configure SvelteKit `kit.csp` so the nonce also covers the styles Svelte injects, or (b) drop the nonce path so `'unsafe-inline'` actually takes effect, or (c) move the affected inline styles to classes. **App-wide blast radius** — verify every transition + `style:` binding across the app before shipping; deliberately kept out of the v0.8.14 update-fix release to avoid re-breaking the updater.

#### UI-drift. App-update surfaces disagree (✅ resolved in-tree, unshipped)

- **Was:** the Settings hero chip hard-coded `{version} · up to date` (green, unconditional) while the pill/titlebar derived from live state — the cont.64 "available vs up to date" screenshot.
- **Fix (2026-06-09, `6e7cb21`):** `UpdateStore.summary` — ONE derived `{kind,label}` (available/downloading/installing → warn `vX available` · checking → busy · error → danger · uptodate → ok · idle → neutral version-only) — and the chip renders exclusively from it (warn/danger `sb-chip` variants added). Pill/titlebar/dialog already derived from store state. Delete this block at next ship.

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

- `docs/design/assistant-mod-split.md` (#20 backend — R1-R8 ready to execute)
- `docs/design/assistant-svelte-split.md` (#20 frontend — COMPLETE, M0-M9 all shipped; kept until mod.rs split adopts its lessons)
- `docs/design/steer-and-queue.md` (steer/queue three-tier model — steer shipped; queue improvements + inline-bubble follow-ups open)

---

## Last full-app verification

- **🔍 Full-app CDP stress pass 2026-06-05 (cont.58) — app healthy.** Walked every workspace + dialog live (Home · Chat · Harness · Settings · command palette · History drawer · Web-browser panel · Panels menu). Ran a real read-only backend turn end-to-end (CLI spawn → MCP `grep`/`glob`/`list_dir`/`read_file` → stream → cost/context/activity render — all correct). Stress: 12 rapid workspace switches + a 14.8K-char emoji/unicode/`<script>` composer paste (auto-grew to the 340px cap, inert, no XSS). **Console: 0 errors / 0 warnings the whole session.** Verified live: cont.57 model/effort capability matrix (Haiku hides slider + shows the no-effort caption), #31–#35 fixes, themeable accent incl. amber warm-hue with no oklch purple-wrap, Harness no-scroll + trust-gated git tools. One new defect found → #36 (now resolved). Could NOT live-exercise: #30 update toast/dialog (app up-to-date on v0.5.0 → state never renders) and first-run onboarding (next-launch only).

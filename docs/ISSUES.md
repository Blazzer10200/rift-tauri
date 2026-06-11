# Rift — Issue Tracker

> **Single source of truth for open work only.** When something ships, **delete its block** — `git log -- docs/ISSUES.md` preserves history. Issue IDs are **durable**: never re-number, only append. Each block carries `Where` (file:line — may have drifted, re-grep before acting), `Symptom`, and an optional `Fix sketch`.
>
> Shipped Wave-1/2/3 audit blocks + clippy tables lived in `docs/archive/audit-history.md` — archive deleted 2026-06-09 (`1810c2e`), recover via `git log`. Pruned 2026-06-04: the pure-assistant conversion (2026-06-03) removed the SFTP/sync/server/RCON stack, so every issue scoped to those subsystems was deleted here (history via `git log`).

### Conventions

- **Status** — `✅ resolved in-tree` (fixed, unshipped — block stays until `/git-ship` so `git log` keeps it) · `🚧 open` · `👤 needs your call` · `🧪 live-verify` (code-complete, needs runtime confirmation) · `🔒 blocked` (external dep) · `🗄 closed` (decided, kept for the record).
- **Tier** — `T1` ship-blocker / data-safety · `T2` code-complete needs live-verify · `T3` strategic / longer-term · `T4` LOW / cosmetic.

### Index

| ID | Title | Tier | Status |
|----|-------|------|--------|
| Auth-Rec | In-app sign-in recovery for 401 failures | T2 | 🧪 live-verify |
| Permission | Allow/Deny round-trip bar | T2 | 🔒 blocked (trust gate) |
| #4 | App-wide UX consistency + navigability sweep | T3 | 🚧 open |
| #17 | Two-repo split → collapse | T3 | 🔒 blocked |
| CR-UX | Trust segment binary-vs-ternary enum | T3 | 👤 needs your call |
| #29 | CSP nonce nullifies `'unsafe-inline'` — inline styles blocked at runtime | T4 | 🚧 open |
| #30 | Workspace chip vs CLI cwd possible drift | T3 | 🚧 open |
| #31 | Deferred 2026-06-11 audit remainder (legacy provider cmds · 401-dup · Fable sunset sweep) | T3/T4 | 🚧 open |
| #32 | Ctx meter blank on restored conversations | T4 | ✅ resolved in-tree |
| #14 | No release CI — local-only path | — | 🗄 closed |

---

## 🚧 Open issues

### Tier 2 — code-complete, needs live-verify

#### Auth-Rec — in-app sign-in recovery for 401 failures (🧪 live-verify)

- **Status:** shipped in v0.8.9+ (`9c468a4`+`2d72af8`) — `assistant_open_login(console)` spawn + actionable 401 banner ([Sign in]/[Open Settings]/[Re-check]). CDP-verified all banner states; the live login spawn itself is compile/registration-verified only.
- **Remaining:** confirm an end-to-end real sign-in on a genuinely-logged-out machine (dev box stays authed). **Strategic follow-ups** (not built): proactive re-probe before first send; auto-prefer an authed install when multiple exist; collapse scattered 401 string-matching into one `AuthError` enum + DiagBus telemetry.

#### Permission — Allow/Deny round-trip bar (🔒 blocked on a trust-standard workspace)

- **Status:** wired end-to-end — `--permission-prompt-tool stdio` (mod.rs) → `can_use_tool` handler → control-response write → `PermissionBar.svelte` Allow/Deny UI → `submitPermissionDecision()`.
- **Live-verify attempt 2026-06-10 (cont.103):** switched to "Ask before edits" + asked for a `git_commit` in a derived-trust workspace — the MCP server correctly **doesn't expose git-write tools at derived trust**, so the prompt can never fire there. That confirms the trust gate works, but the bar itself remains unexercised. Verifying requires pinning `trust_level=standard` on a throwaway repo — deliberately not done unattended because the trust segment **pins one-way** (see CR-UX). Fold into the CR-UX decision: when the trust enum is reworked, verify the bar in the same pass.

### Tier 3 — strategic / longer-term

#### 4. UI/UX consistency + navigability sweep (app-wide)

- **Scope:** not a single bug — tracks the stated goal of an app-wide consistency pass. The Settings page is the densest control surface and the natural starting point.
- **Goal:** every visible control is wired, every section is necessary, terminology + styling consistent, navigation intuitive.
- **Progress (cont.105, 2026-06-10):** audit findings **#1-#6 + #8-#10 SHIPPED** and live-verified — Steps-rail `cd`-strip (`shellLabel` + vitest), slash menu → palette design language, empty-dock auto-collapse, scroll bottom padding, **per-chat model scoping** (`TabState.modelOverride` + `effectiveModel`; opening an old chat no longer rewrites the new-chat default or toasts), jump-back-in snippets + model chips (backend `last_snippet` on `ConversationMeta`), KPI zero-state unify + `opus· high` space fix, user-turn inset card, insight severity stripes. Audit's "Jump back in doesn't navigate" suspicion: **not a bug** (verified live).
- **Remaining from the audit:** #7 cost-chart sparse-data polish · #11 rich inline diff (design pass) · #12 tool-chip expand affordance · #13 tab strip into titlebar (lowest priority) · message hover actions discoverability. (`/history` fixed + live-verified 2026-06-11 — drawer opens on single Enter via store request flag.) Then the per-page Settings checklist.
- **Input:** [ui-audit-2026-06-09.md](design/ui-audit-2026-06-09.md) — live CDP audit of v0.8.14, 13 ranked findings (refinement tier, not redesign).

#### 31. Deferred remainder from the 2026-06-11 dead-code/debug audit

Three audits (backend, frontend, orphan files) shipped a sweep this session; these were found but deliberately deferred:

- **Legacy provider commands (✅ resolved in-tree 2026-06-11):** removed the four commands (config.rs + lib.rs registry) and the dead frontend plumbing (`baseUrl`/`providerModel` $state, init getters, `setBaseUrl`/`setProviderModel`) — zero component callers existed; the providers list is the only surface. Config struct fields + first-load migration KEPT so pre-2a configs still migrate.
- **401-detection duplicated in turn.rs (✅ resolved in-tree 2026-06-11):** extracted `is_auth_rejection()` + `auth_rejection_message()`; both the stdout result-frame and stderr-exit sites now share one detector + one remap (unified on the richer CLI-path message).
- **Blocking fs reads in async commands (T4):** `load_config()` in `assistant_send` / `read_oauth_token()` in `usage_rate_limits` do sync disk I/O on the tokio executor. Tiny local files — wrap in `spawn_blocking` only if it ever shows up in traces.
- **Fable sunset sweep (dated):** after Jun 22 (`FABLE_SUNSET_MS = Date.UTC(2026, 5, 23)` in `state/assistant/helpers.ts`), all Fable branches (`fableAvailable()` gates, `fableSunsetNoticed` toast in send.ts, `limited` rows in modelMatrix.ts) become permanently dead — sweep them out.
- **Optional split follow-ups (quality, not threshold — #20 closed at ship):** [messagebubble-split.md](design/messagebubble-split.md) B1-B6 + [chattabsbar-split.md](design/chattabsbar-split.md) T1-T6 stay mapped; `assistant_send` (917L fn inside turn.rs) can split internally later.

#### 17. Two-repo split — historic, low-priority collapse (🔒 blocked)

- **Where:** [scripts/release.ps1](../scripts/release.ps1) publishes to `Blazzer10200/rift-releases`; [src-tauri/src/update_service.rs](../src-tauri/src/update_service.rs) points Velopack's `GithubSource` at the same public repo.
- **Symptom:** every release requires manual sync between the private source repo and the public releases repo. Forks/contributors can't test the update path against the real source.
- **Fix sketch:** collapse to a single repo **if the source repo goes public** — a small change in `release.ps1` + the update source constant. Blocked on that decision.

#### CR-UX. Trust segment binary-vs-ternary enum (👤 needs your sign-off)

- **Symptom:** the trust segment is binary (Read-only/Standard) over a **ternary** backend enum (`readonly/standard/full`). Once clicked, `trust_level` pins and can't return to the derived state via UI; "full" (rank 2) is functionally identical to "standard" — only `"standard"` is gated for git writes.
- **Recommendation:** collapse to a true 2-level enum (drop the dead "full"). Touches `mcp_server::trust_rank`/`trust_level`, `mod.rs::is_valid_trust_level`/`effective_trust_level`/git-write gate, serde, + persisted-config migration.
- **Held for sign-off:** security-relevant + persisted-config change.

#### 30. Workspace chip shows current workspace, not the resumed tab's cwd (🚧 open — UX gap, root cause known)

- **Seen 2026-06-11 (cont.110, live CDP):** chips said `resume-project` while real turns read remotion-playground files. **Explained:** the turns resumed an old tab whose session was started under remotion-playground — `load_session_cwd` (convo_store) correctly pins a resumed session to its original cwd; the title-bar chip reflects the *currently selected* workspace only. Per-session cwd persistence working as designed.
- **The gap:** nothing in the UI tells you a tab is operating in a different folder than the chip shows. Fix sketch: per-tab cwd badge (tabsbar tooltip or composer notice) when `session_cwd != workspace.current`.

### Tier 4 — LOW / cosmetic

#### 29. CSP nonce nullifies `'unsafe-inline'` — inline styles blocked at runtime (🚧 open)

- **Where:** [src-tauri/tauri.conf.json](../src-tauri/tauri.conf.json) `csp` (`style-src 'self' 'unsafe-inline'`). At runtime SvelteKit injects a `nonce-…` into the served CSP.
- **Symptom (observed v0.8.14, prod CDP):** per CSP spec, **a nonce makes `'unsafe-inline'` be ignored** — so Svelte's dynamically-applied inline styles get blocked. Console spams `Applying inline style violates ... style-src 'self' 'unsafe-inline' 'nonce-…'`. Real impact: Svelte transition styles (fly/fade) and `style="width:{progress}%"` on the update download progress-bar don't apply. **Cosmetic** — download/apply and all clicks still work; animations snap and the progress fill stays empty.
- **Fix sketch:** make the static CSP and SvelteKit's nonce agree. Either (a) configure SvelteKit `kit.csp` so the nonce also covers the styles Svelte injects, or (b) drop the nonce path so `'unsafe-inline'` actually takes effect, or (c) move the affected inline styles to classes. **App-wide blast radius** — verify every transition + `style:` binding across the app before shipping; deliberately kept out of the v0.8.14 update-fix release to avoid re-breaking the updater.

#### 32. Ctx meter blank on restored conversations (✅ resolved in-tree 2026-06-11)

- **Fix shipped in-tree:** `buildSaveRecord` persists `lastTurnUsage` on the record (backend `Conversation.extra` flatten round-trips it, zero Rust changes); `loadConversation` hydrates it after `resetUsage()`. Block stays until `/git-ship`.

- **Where:** `ActivityPanel.svelte` ctx meter + composer gauge read `assistant.ctxTokensFor(tab)` → `tab.lastTurnUsage`, which is set only by live `recordTurnUsage` and never persisted by the conversation store.
- **Symptom:** a history-restored conversation shows no Context gauge (and 0% in the composer) until the first new turn completes — exactly when "how full is this old session?" matters.
- **Fix sketch:** persist the final turn's usage in conversation meta on save; hydrate `lastTurnUsage` in `loadConversation`. Found cont.113 (2026-06-11).

> Also parked: **Wave-1 LOWs #91–#134** — clippy/doc/perf nits, in the deleted `docs/archive/audit-history.md` (recover via `git log` if ever needed; not tracked live here).

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

- `docs/design/assistant-mod-split.md` (#20 backend — COMPLETE R1-R8, shipped v0.8.16; kept as the split pattern reference)
- `docs/design/composer-split.md` (#20 frontend — **COMPLETE C1-C7 2026-06-10**; kept as the component-split pattern reference — the `composer/` child headers cite it)
- `docs/design/messagebubble-split.md` + `docs/design/chattabsbar-split.md` (optional quality follow-ups — both files already under threshold)
- `docs/design/assistant-svelte-split.md` (#20 frontend — COMPLETE, M0-M9 all shipped; KEPT permanently — the `src/lib/state/assistant/*` module headers reference it)

---

## Last full-app verification

- **🔍 Composer + effort live pass 2026-06-10 (cont.103) — all green.** Effort ladder retuned to mirror the CLI 1:1 (5 stops; Smart=`--effort high` default; Deep=`xhigh`; Ultracode=`xhigh`+workflows; Sonnet caps at Smart; Haiku hidden) and proven end-to-end: spawn log showed `model=sonnet effort=high` on a real turn. C3-C7 extractions each CDP pixel-verified live: queue chip mid-stream + ✓Steered flash + a visible mid-turn redirect (REDIRECTED in transcript), live pills `0:07 · 382 tok/s`, enhance panel (real Haiku rewrite, Ground/Diff toggles, Discard w/ draft intact), slash menu (15 cmds), @mention fuzzy pick, settings menu digit-pick / drag-to-Ultracode / Opus→Sonnet clamp, perm menu portal pick + outside-close. Known CDP wart: `c.sh look`'s console-error list accumulates since cdp:serve boot (mid-HMR ReferenceErrors linger) — trust the screenshot/state, not the stale error tail, or restart cdp:serve.
- **🔍 Full-app CDP stress pass 2026-06-05 (cont.58) — app healthy.** Walked every workspace + dialog live (Home · Chat · Harness · Settings · command palette · History drawer · Web-browser panel · Panels menu). Ran a real read-only backend turn end-to-end (CLI spawn → MCP `grep`/`glob`/`list_dir`/`read_file` → stream → cost/context/activity render — all correct). Stress: 12 rapid workspace switches + a 14.8K-char emoji/unicode/`<script>` composer paste (auto-grew to the 340px cap, inert, no XSS). **Console: 0 errors / 0 warnings the whole session.** Verified live: cont.57 model/effort capability matrix (Haiku hides slider + shows the no-effort caption), #31–#35 fixes, themeable accent incl. amber warm-hue with no oklch purple-wrap, Harness no-scroll + trust-gated git tools. One new defect found → #36 (now resolved). Could NOT live-exercise: #30 update toast/dialog (app up-to-date on v0.5.0 → state never renders) and first-run onboarding (next-launch only).

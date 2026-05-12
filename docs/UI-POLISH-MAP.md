# UI Polish Map — post-S32/S33 sweep

Canon source: `docs/HANDOFF.md` → "Design system canon". Apply the same rhythm (tone-coded surfaces, hover scale, inset-stripe active state, click-blur on focus-within overlays, surface fills at 8-14% rest / 22% hover, color-keyed icons, fly/fade animations, single source of truth for any datum, hide-when-zero, tooltips on truncated content).

Status legend: ✅ done · 🟡 partial · ⬜ pending.

---

## Already on canon (S31-S33)
✅ AppShell · Titlebar · TabRail · StatusHero · StatusBar · CommandPalette · TwoPane · LocalPane · RemotePane · PathBreadcrumbs · LockBadge

---

## Pending — order of attack (quick wins → biggest scope)

### Tier 1 — small, mostly-aligned dialogs (warm-up) ✅
1. ✅ **Confirm.svelte** (82). Non-danger dialog-icon now `.info` (was neutral bg-elev-2). Esc/Enter kbd affordance hints in foot. (S34)
2. ✅ **Reupload.svelte** (82). Esc/Enter kbd affordance hints in foot. Already on-canon otherwise. (S34)
3. ✅ **FlashToast.svelte** (83) + **ActivityToast.svelte** (129). Already on-canon — `data-variant` w/ soft-bg chip, left-stripe accent, fade-in. No edits needed.
4. ✅ **app.css `.btn` skeleton lifted** — `.btn:active` translateY(1px) micro-bounce, `.btn:disabled` consistent state, `.btn.primary` font-weight 600 + accent-tinted hover shadow, `.btn.danger` danger-tinted hover shadow, added `.btn.warn` + `.btn.info` tones, added `.btn.lg` 38px primary-CTA size per canon. Every dialog/button in the app inherits. (S34)

### Tier 2 — sidebar pane (conflicts)
4. ⬜ **ConflictList.svelte** (151). The left aside on the Conflicts tab. Touch points: bulk-bar buttons should be tone-coded (`Use Remote for all` = info/pull tone, `Use Local for all` = warn/push tone — NOT identical ghost buttons). Selected row already has danger-soft bg + border-left; convert to `inset 2px 0 var(--danger)` per canon. Empty state could get the "title + hint" pair pattern.
5. ⬜ **ConflictResolver.svelte** (318). Right pane on Conflicts tab. Three-way Pick (`local | remote | save_copy`) — tone each pick card (local=warn, remote=info, save_copy=neutral). Action buttons row (`Apply pick`, `Skip`, `Edit in place`, `Copy path`) need primary/secondary hierarchy.

### Tier 3 — login flow (Blazzer flagged this explicitly)
6. ⬜ **Bootstrap.svelte** (283). First-run setup. This is the FIRST surface a new user sees — biggest "perception" win. Step indicators, key generate/import buttons, server form, connect button. Pay attention to: progress affordance, tone-coded step pips, the primary CTA at each step.
7. ⬜ **AddServer.svelte** (423). Server add/edit modal. Form layout, ed25519 key picker, advanced bridge fields collapsible, primary CTA. Server-add IS the login path post-bootstrap.
8. ⬜ **Keygen.svelte** (190). Standalone key generation modal. Generate / copy public key / cancel. Tone the copy action when content's ready, success flash.

### Tier 4 — modal flows
9. ⬜ **SyncModal.svelte** (560). Progress modal for Reconcile/Pull/Push. Already has variant mapping (`SyncActivityKind`). Verify chip tones map to canon (pull=info, push=warn, error=danger, drift=accent, system=neutral). Progress bar tone-keyed to mode. Cancel button affordance.
10. ⬜ **UpdateDialog.svelte** (160). Velopack update modal. Available/downloading/ready states each get tone (info → accent → ok). Progress + restart CTA.

### Tier 5 — page-level
11. ⬜ **Settings.svelte** (364, all 7 sections inline — Appearance, Tokens, Servers, Keys, Sync, Editor, About). Nav already uses inset-stripe ✅. Pending: tone the section icons (Servers=accent, Keys=warn, Sync=info, rest=neutral). Server cards (`.srv-card`) selected state currently uses border + accent-soft — convert to canon inset-stripe + 14% tone-mix. Edit/Delete ghost buttons could get warn/danger hover tints.
12. ⬜ **Diagnostics.svelte** (405). Virtualized event log. Already has `levelVariant` + `stageVariant` — verify mapping uses canon tones (info/warn/danger/muted), pause/play/clear toolbar tone-coded, copy-to-clipboard feedback flash.
13. ✅ **ActivityFeed.svelte** (875). Filter chip strip tone-coded (All=neutral, Sync=ok, Pull=info, Delete=warn, Drift=warn, Conflicts=danger, Bridge=info, Errors=danger, System=neutral) w/ inset-stripe active state + tone-tinted bg + tone fg + weight 600. Count pips inherit tone when non-zero. Filter input bumped to canon (border-strong + ring on focus + height matched). Pause button warn-tinted when paused (visual reminder feed is frozen). Empty state title+hint pair. Selected row gets inset stripe per canon. Group row bg tone-keyed by kind. Detail strip bg + stripe tone-keyed. Paused banner warn-tinted. (S36)

### Tier 6 — likely no-touch
- `routes/+page.svelte` (5 lines) — pass-through.
- `routes/+layout.svelte` (15 lines) — global wrapper, font/theme load only.

---

## Canon checklist (apply to every page)

- [ ] `data-tone` attr on every interactive surface that conveys category/meaning, mapped to a `--tone` CSS var (accent/info/warn/danger/neutral).
- [ ] Surface fills: rest 8-14% tone-mix, hover 22%. Border: rest 28%, hover 55%.
- [ ] Icons opacity 0.75 at rest (when category-coded), 1.0 + `scale(1.1-1.18)` w/ cubic-bezier(0.34, 1.56, 0.64, 1) on hover. `prefers-reduced-motion` guard.
- [ ] Active/selected: `box-shadow: inset 2px 0 var(--tone)`. NOT border-left swaps.
- [ ] Entries fade ~140-180ms quintOut, exits scale/fade 90-140ms.
- [ ] Click-blur pattern on any button inside a `:focus-within`-driven overlay.
- [ ] Empty states get "title + hint" pair (not just "No data").
- [ ] Hide-when-zero on counts/badges.
- [ ] Tooltips on truncated/abbreviated content (paths, fingerprints).
- [ ] Time displays pass `{ hour12: true }` to all `toLocaleTimeString`/`toLocaleString`.

---

## Don't-reintroduce list (intentional kills from S32)
OpRail · TopBar (merged into Titlebar) · rail kbd hints (`⌘1`) · StatusBar `⌘K` pip · titlebar Settings gear · StatusHero big H1 · S33 duplicate "watching" words.

---

## Workflow
Blazzer sends screenshot → I spot off-canon → propose + cook → commit + push (NOT `/git-ship`). One tier at a time, screenshot between tiers for sign-off. Background svelte-check runs per edit — only surface NEW fail-stamps (LocalPane/RemotePane :294 `<section>` warnings are pre-existing backlog).

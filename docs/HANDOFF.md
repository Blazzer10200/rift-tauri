# rift-tauri — Handoff

> Live handoff = current session block. Older sessions live in `git log -- docs/HANDOFF.md`.

## Session 36-37 — 2026-05-12 — Activity page polish + audit (Tier 5 done)

Frontend-only. Still on **v0.2.38-alpha-test**, no Rust changes, NOT shipped. Map at `docs/UI-POLISH-MAP.md`.

`ActivityFeed.svelte` was the largest unpolished surface (875 lines). Full canon pass landed in 5 commits:

- **Filter chip strip tone-coded** — was 9 neutral chips. Now `data-tone` per group (All/System=neutral, Sync=ok, Pull/Bridge=info, Delete/Drift=warn, Conflicts/Errors=danger). Active = tone-tinted bg 18% + tone fg + inset 2px tone stripe + weight 600. Count pips inherit tone when non-zero.
- **Filter input** — bumped to canon: 26px height, border-strong rest, accent border + 3px ring on focus.
- **Pause button** — `.btn warn` when paused (was always `.btn ghost`). Visual cue feed is frozen.
- **Empty state** title+hint pair ("No activity yet" / "Sync, pull, and bridge events will appear here…"). Same for filter-empty.
- **Rows + groups** — `data-variant={kindVariant(r.kind)}` drives: selected row tone-keyed bg + inset stripe matches the kind (clicking a warn row gets warn selection + warn strip, not jarring accent), group row bg tone-keyed at 72/28 amplitude w/ soft tone stripe so burst patterns cluster visually, detail strip bg + stripe tone-keyed to selected variant.
- **Column widths** — action col bumped 200 → 360px max so long messages no longer truncate mid-word ("BLOCKED — foreign lock held by trey@DESKTOP" fits clean). Time col 96 → 124px to fit relative format.
- **Time format** — `fmtTime` swapped from raw `toLocaleTimeString` to shared `fmtRelative` helper (Today/Yesterday/Mon/MM-DD). Tooltip carries the full localized timestamp via title attr.
- **Paused banner warn-tinted** (was neutral, easy to miss).

S37 stripped the dev seed (Sparkles button + IS_DEV gate + 11-row sample array) — was only there to preview tones during S36 polish. Wiring audited clean: every interactive handler traced to real backend or state (Pause/Clear/filter/chips/row-select/group-toggle/strip Open-Copy-Reveal/burst-pip).

### Verify
- svelte-check: 0 errors. Warnings unchanged from S35 (the 2 extra vs S33-baseline are from the parallel dashboard session's edits on LocalPane/RemotePane/TwoPane, not Activity).
- Zero Rust changes.

## Session 35 — 2026-05-12 — Browser page audit (canon plumbing check)

Re-audit pass on Browser before moving on. 3 surgical fixes: PathBreadcrumbs filter button had `data-active` attr w/o CSS rule (was silent bug — looked identical open vs closed); LocalPane + RemotePane empty states upgraded from `Empty.` to title+hint pair per canon. Rest of Browser audited as on-canon.

## Session 31-34 — compressed

S31-S32: post-WPF-retirement structural cleanup + initial canon push (sidebar overlay collapse, OpRail kill, Win11 tabstrip, StatusHero auto-hide, RTL breadcrumbs, Titlebar+TopBar merge, command palette polish, tone-coded TabRail). S33: StatusHero + StatusBar "watching" dedup. S34: `.btn` skeleton lifted in `app.css` (`.btn:active` translateY universal, `.btn.primary` weight 600 + accent hover shadow, `.btn.warn` + `.btn.info` tones, `.btn.lg` 38px CTA size) + Confirm/Reupload kbd hints. Full session diffs preserved in `git log -- docs/HANDOFF.md`.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri IS Rift. Path: `C:/AI Workflow/projects/rift-tauri/`. Version still **v0.2.38-alpha-test** (post-S37 work is frontend-only — no bump).

**Current state:** `origin/main` includes S31 → S37 UI overhaul. **NOT shipped** — Blazzer is dev-testing. No-release directive holds: don't trigger `/git-ship` or Velopack publish until cleared.

**Rhythm directive:** keep applying the Design system canon (codified in `docs/UI-POLISH-MAP.md`) to every remaining unpolished page. Tone system w/ `data-tone` attr + `--tone` CSS var (accent/info/warn/danger/neutral), surface fills 8-14% rest / 22% hover, hover-icon scale 1.1-1.18 w/ cubic-bezier overshoot + reduced-motion guard, active-state inset-stripe `box-shadow: inset 2px 0 var(--tone)`, click-blur on any focus-within overlay button, single-source-of-truth for any datum, hide-when-zero, title+hint pair on empty states, tooltips on truncated content.

**Polish state (per UI-POLISH-MAP.md):**
- ✅ Done: AppShell, Titlebar, TabRail, StatusHero, StatusBar, CommandPalette, TwoPane, LocalPane, RemotePane, PathBreadcrumbs, LockBadge, Confirm, Reupload, FlashToast, ActivityToast, ActivityFeed (Tier 5 done), `.btn` skeleton in app.css.
- ⬜ Pending — Tier 2 ConflictList + ConflictResolver; Tier 3 login flow Bootstrap + AddServer + Keygen; Tier 4 SyncModal + UpdateDialog; Tier 5 remainder Settings + Diagnostics.

**Parallel session note:** A separate Claude session has in-flight edits on `src/lib/components/browser/{LocalPane,RemotePane,TwoPane}.svelte` (the "dashboard" branch of work). Don't touch those 3 files until that session merges, to avoid stomping. svelte-check shows 8 warnings (6 pre-existing baseline + 2 from that session's edits) — not actionable here.

**Don't reintroduce:** OpRail, TopBar (merged into Titlebar), rail kbd hints `⌘1`, StatusBar `⌘K` pip, titlebar Settings gear, StatusHero big H1, S33 duplicate "watching" words, S37 dev seed (Sparkles button).

**Orphan file** `scripts/bg-backlog.sh` is from an accidental cross-chat — left untracked locally, NOT committed. Safe to delete if Blazzer doesn't want it.

### Flagged for v0.2.39+
- Pre-flight write probe on connect.
- Activity-feed row grouping for bulk reconciles (partial — group collapse landed S36).
- Remote `.rift-lock` cruft sweep on watch attach.
- Many-tabs horizontal scroll (currently `overflow: clip` — add fade-edge + arrow buttons when >10 tabs expected).
- `<section>` a11y warnings on LocalPane/RemotePane — backlog.

## Session 31-33 — pruned

Earlier UI overhaul + sync verification + structural cleanup. Full session diffs preserved in `git log -- docs/HANDOFF.md`. Design system canon now lives in `docs/UI-POLISH-MAP.md` (single source of truth, not duplicated here).

---

## RESUME HERE — first read every new session

**Project:** rift-tauri IS Rift. Path: `C:/AI Workflow/projects/rift-tauri/`. Version still **v0.2.38-alpha-test** (S31 + S32 were frontend-only, no Rust changes, no bump).

**Current state (post S32):** v0.2.38 source on `origin/main` w/ S31 + S32 UI overhauls committed (multiple commits, last was command palette polish). **NOT shipped** — Blazzer still dev-testing. No-release directive holds: don't trigger `/git-ship` or Velopack publish until cleared.

**Rhythm directive (Blazzer's explicit ask):** keep applying the "Design system canon" section above to every remaining unpolished page. The pages we DIDN'T touch yet in S32: `ActivityFeed.svelte`, `ConflictList.svelte`, `ConflictResolver.svelte`, `Diagnostics.svelte`, `Settings/*.svelte` sub-views (Appearance/Tokens/Servers/Keys/Sync/Editor/About), and the dialog stack (`AddServer.svelte`, `Bootstrap.svelte`, `Keygen.svelte`, `Confirm.svelte`, `Reupload.svelte`, `UpdateDialog.svelte`). Same tone-coding, same hover scale, same active-stripe pattern, same blur-on-click for any focus-within-driven overlay.

**Next session likely entry points:**
1. Pick the next unpolished page from the list above, apply the canon.
2. Blazzer feedback on S32 fine-tuning — sometimes screenshots arrive w/ specific spots that "don't feel right" — same approach: identify, propose, polish.
3. v0.2.38 dev-test continues — does the auto-sync rip kill the `[world]` ping-pongs in real use?
4. If both hold + Blazzer signs off on visual pass: bump to v0.2.39 + ship pipeline (`/git-ship` user-invoked only).

**Orphan file** `scripts/bg-backlog.sh` is from an accidental cross-chat — left untracked locally, NOT committed. Safe to delete if Blazzer doesn't want it.

## CRITICAL DON'T-TOUCH
- russh `ring` backend + reqwest `rustls` features only (NASM blocks aws-lc-rs)
- `~/.rift/*.json` compat — don't change rename rules; don't drop `serde(flatten) extra`
- `VelopackApp::build().run()` first call in `lib.rs::run()`
- `bundle.targets: ["nsis"]` while `-alpha`/`-beta` (MSI rejects non-numeric semver)
- DriftWatcher conflict-rename guard — never overwrite dirty local
- `.rift-trail.jsonl` ignore rule — pull→push loop reappears w/o it
- `GITHUB_OWNER`/`GITHUB_REPO` point at public `rift-releases`, NOT source repo
- `path_guard.rs` API frozen (`validate_remote_child`, `validate_local_child`) — `edit/in_place.rs` + lib cmds depend
- `rename_via` is strict (user-facing); `rename_overwriting_via` is ONLY for atomic upload tmp-swap
- **Source `.secrets/env.sh` first on ship/auth tasks** — Claude Code bash is non-interactive, won't auto-load
- **`last_scan_entries` is std::sync::Mutex** (NOT tokio) — `kick_drift_reconcile` is sync and called from notify event handler; tokio Mutex `blocking_lock` panics there. Don't "fix" it. (`current_scan_cancel` removed in v0.2.38 — no longer applies.)
- **`force_pull_now` does an inline drift scan + dispatches with the guard in front** (post-v0.2.38). Pre-v0.2.38 it dispatched from cache (drift_watcher's 10s tick kept cache fresh). With the tick deleted, cache freshness isn't guaranteed → Pull Now must do its own scan. Don't "optimize" back to cache-only dispatch — that's how stale tombstones could fire mass deletes.
- **NEVER use `FileAttributes::default()` for SETSTAT** — it sends `size: Some(0)`, `mtime: Some(0)`, `atime: Some(0)`, `uid/gid: Some(0)` which the server honors → file truncation + epoch mtime. Always use `FileAttributes::empty()` and explicitly set only the fields you want to change. See v0.2.27 post-mortem.
- **`SftpClient::delete` routes by remote stat** — dirs go through `delete_recursive_via`. Don't shortcut back to `remove_file` for "files only" — the push pipeline can't distinguish file from dir deletes ahead of time. See v0.2.29.
- **`mkdir_p_via` chmods each segment to 2775** — setgid + group-writable is required for shared-group teammates to push into each other's dirs. Don't drop the SETSTAT call — backlog gets healed too via `heal_owned_dirs` on watch attach. See v0.2.31.
- **Upload pre-flight SHA-collapse before raising CONFLICT** — when sizes all match + baseline SHA exists, hash local first (cheap), then remote via SSH exec. If both match baseline, refresh baseline mtime + drop the push. Mtime jitter (npm builds, SETSTAT, git checkout) flooded Trey w/ 53 phantom conflicts in v0.2.31; v0.2.32 fixed. See `auto_sync.rs:1522`.
- **`DriftBucket::ToDelete` is the tombstone path** — `local + no remote + has_baseline` MUST classify as `ToDelete`, NOT `ToPush`. Without it, deletes from teammates leave ghost files locally + risk accidental resurrection (autosync re-uploads on next touch). Dispatcher routes ToDelete → `drift_watcher::delete_local_one`, which guards on foreign-lock + dirty-local (skip unflushed edits — never blow away user's work). Empty-parent-dir cleanup walks up post-delete. See v0.2.33 post-mortem.
- **All time displays use `hour12: true`** — Blazzer requires 12-hour everywhere. Any new `toLocaleTimeString`/`toLocaleString` call MUST pass `[], { hour12: true }` explicitly (locale-default emits 24-hour on non-US machines). See v0.2.34 audit.
- **Mass local-delete circuit breaker lives in `force_pull_now`** (post-v0.2.38). Same `(file_count * 0.30).clamp(5, 25)` formula, same `BLOCKED — N local-deletes` activity row, same `kind=block`. Don't relocate this guard — Pull is the ONLY path that propagates tombstones since auto-sync was ripped, so the guard MUST sit there. See v0.2.36 + v0.2.38.
- **DO NOT restore `drift_watcher::spawn`, `run_tick`, `flush_cycle`, `auto_flush_enabled`, `remote_scan_interval_secs`, `drift_watcher_task`, `flush_task`, `LOOP_TICK_MS`, `track_pull_handle`, `register_scan_cancel`, `clear_scan_cancel`** — all deleted intentionally in v0.2.38. Auto-sync ping-ponged `pulled → removed locally → pulled → removed locally` on `[world]` resources at 10s tick because drift_watcher classified entries as `ToDelete` one tick + `ToPull` the next while baseline/devbridge raced. The fix was to delete the auto path entirely. Push/Pull buttons only. Watcher still runs to populate dirty queue (StatusBar pending count) but nothing flushes until user clicks. See v0.2.38 post-mortem.

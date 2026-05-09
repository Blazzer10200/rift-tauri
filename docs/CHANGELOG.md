# rift-tauri — Changelog

> Live changelog = current version only. Older entries archive to `archive/CHANGELOG-archive.md` on bump.

## v0.2.0-alpha — 2026-05-09 — Linear-precise UI port: phases 0–11 complete

Full Claude Design "Rift App UI" deliverable ported into the codebase. **All 12 phases done in one session arc** (0–3 from S12, 4–11 this session). Backend untouched. Dev-only — public ship still gated on user say-so.

### Phase 4 — Activity feed
Segmented filter w/ live count pips per kind, semantic kind badges (RefreshCw/Download/Trash2/AlertTriangle/etc via lucide w/ ok/warn/danger/info backgrounds), time gutter, pause/resume w/ "N new since pause" banner, clear button. `connection.clearActivity()` added.

### Phase 5 — Drift tab
Sub-toolbar w/ side filter (All / Local wins / Remote wins, count pips) + grouping (Dir / Flat) + Auto-resolve safe button. Sticky bulk-action bar appears only on selection (Apply combined push/pull). Dir-grouped expandable rows w/ chevron, side pill, inline size stat, expandable peek (paths, sizes, mtimes, snapshot, conflict→Conflicts-tab note). Conflicts shown un-selectable w/ red bg.

### Phase 6 — Dialogs
Shared `.dialog-overlay/.dialog-shell/.dialog-head/.dialog-body/.dialog-foot/.stepper/.step` primitives in `app.css`. All 5 dialogs refactored to tokens + lucide icons: **Confirm** (variant icon, Don't ask), **Reupload** (now auto-pops from `dirtyEdits` queue — skip/re-upload/always w/ per-server localStorage pref), **Keygen** (key blob + fingerprint preview + copy), **Bootstrap** (variant icons per detection state, top-level folder chips, progress bar), **AddServer** (3-step stepper Connection→Workspace→Bridge w/ live ssh preview + summary card).

### Phase 7 — Command palette
Fuzzy scoring (prefix > substring > group/subtitle), group column, kbd shortcut chips. Auto-scroll selected into view. Command registry in AppShell now tagged w/ `group` field.

### Phase 8 — Conflicts
ConflictList w/ count pip + active-row red border. ConflictResolver: click-to-pick side meta cards (info-bordered local / warn-bordered remote), diff peek (paths, size delta, last-known sync), action toolbar (Skip / Save copy + pull / Apply primary). Per-hunk merge deferred to backend ticket per S12 decision.

### Phase 9 — Polish
StatusHero: data-variant border tones, 24px LED w/ pulse-soft, lucide icons in card labels. ActivityToast: lucide icons, semantic borders, click-to-dismiss. Density persistence via new `state/ui-prefs.svelte.ts` (localStorage `rift.ui.density.v1`, init in `+layout.svelte` mount). Reduced-motion already gated.

### Phase 10 — Verify
- `npm run check` — 3933 files, 0 errors, 1 advisory (intentional Settings:18)
- `cargo clippy --lib --tests` — clean
- `cargo test --lib` — 47 passed, 2 ignored

### Tauri 2 fix
`core:window:allow-{start-dragging,minimize,toggle-maximize,close}` added to `default.json` capability — `data-tauri-drag-region` was silently no-op'ing.

### Versions
`Cargo.toml`, `package.json`, `tauri.conf.json` → 0.2.0-alpha. v0.1.6-alpha archived.

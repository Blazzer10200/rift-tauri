# rift-tauri — Handoff

> Live handoff = current session block. Older sessions flow to `archive/HANDOFF-archive.md`.

## RESUME HERE — first read every new session

**Project:** WPF→Tauri migration of Rift. Sibling repo to `Blazzer10200/rift` (WPF v13.55.x ships from there). User's daily app = WPF Rift (`%LOCALAPPDATA%\Rift\current\rift-app.exe`). This = the v14.0.0 future, installed alongside as `rift-tauri.exe`.

**🚨 PATH SANITY CHECK:** `pwd` = `C:/AI Workflow/rift-tauri/`. The WPF repo at `C:/AI Workflow/Rift Project/` is **stable port-from reference** — DO NOT refactor unless user says "WPF" or "v13.x". When user says "rift" → default to `rift-tauri`.

**Current state (post Session 13, v0.2.0-alpha):** Linear-precise UI port complete (12/12). Backend untouched since S11. Installer built + installed at `%LOCALAPPDATA%\Rift\rift-tauri.exe`, desktop shortcut placed. Sessions 11+12+13 staged uncommitted — `/git-ship` lands when user says.

**Release policy:** No public ship until user explicitly says go.

## Verified phase status

| Phase | Scope | Status |
|---|---|---|
| 0–5 | Backend + UI shell + browser + sync surfaces + dialogs | ✅ |
| 5.5 | UI redesign foundation | ✅ S10 |
| 5.6 | Backend audit (85 findings) | ✅ S11 |
| 5.7 | Linear-precise UI port — 12/12 phases | ✅ S12+S13 |
| 6 | v14.0.0 public ship | ⏳ deferred |

## Session 13 — 2026-05-09 — UI port phases 4–11 + 0.2.0-alpha ship

**Tauri 2 fix first:** `data-tauri-drag-region` was silently no-op'ing — `core:default` permission set does NOT include `core:window:allow-start-dragging`. Added explicit grants in `capabilities/default.json` for start-dragging/minimize/toggle-maximize/close. Memory: `~/.claude/memory/reference_tauri2_drag_region.md`.

**Phases 4–11** all /check-clean:
- **P4** Activity: segctl + pips + lucide kind badges + pause/clear (`connection.clearActivity()`).
- **P5** Drift: dir grouping + sticky bulk action bar + expandable peek + auto-resolve safe.
- **P6** Dialogs: shared `.dialog-*` primitives in `app.css`. All 5 refactored. Reupload now auto-pops from `dirtyEdits` queue (skip / re-upload / always w/ per-server localStorage).
- **P7** Palette: fuzzy scoring + group column + kbd chips. AppShell registry got `group` field.
- **P8** Conflicts: click-to-pick side meta cards + diff peek. Per-hunk merge = future backend ticket per S12 decision.
- **P9** Polish: StatusHero LED + pulse, ActivityToast restyle, density persistence via new `state/ui-prefs.svelte.ts`.
- **P10** Verify: `npm run check` 3933/0/1 (intentional Settings:18 advisory), `cargo clippy --lib --tests` clean, `cargo test --lib` 47/47.
- **P11** Ship: bumped 0.1.6→0.2.0-alpha across `Cargo.toml`/`package.json`/`tauri.conf.json`. CHANGELOG/HANDOFF rotated.

### Build + install
- `tauri.conf.json`: `productName: "Rift"` (was `rift-tauri`), `bundle.targets: ["nsis"]` (MSI rejected `-alpha` semver), NSIS perUser install, custom installer icon.
- Icons: extracted 256×256 frame from WPF `rift.ico`, upscaled to 1024, regenerated all Tauri sizes via `npx tauri icon`.
- Bundle: `Rift_0.2.0-alpha_x64-setup.exe` (5.7 MB) → silent installed → `%LOCALAPPDATA%\Rift\rift-tauri.exe` (21 MB) + desktop shortcut. Icon cache wiped + explorer restarted.

### Next session pickup
Smoke-walk the installed binary (5 tabs / dialogs / Cmd+K / density persist / drag). If clean → `/git-ship` lands S11+12+13 as one commit. If issues → fix in P9.5/P12.

## CRITICAL DON'T-TOUCH (carries forward)
- russh `ring` backend (NASM blocker on aws-lc-rs)
- reqwest `rustls` features only
- npm runner, NOT pnpm
- File-format compat w/ WPF `~/.rift/*.json` — never change rename rules; never drop `serde(flatten) extra` on `RiftConfig`
- Velopack `VelopackApp::build().run()` first call in `run()`
- shadcn-svelte components own themselves in `src/lib/components/ui/`
- Linear-precise dark only (light/raycast-glass/sublime cut)
- **Tauri 2 capability gotcha:** `core:default` lacks window:allow-* grants — explicit perms required for custom titlebar
- **MSI bundler rejects non-numeric prereleases** — keep `bundle.targets: ["nsis"]` while versions carry `-alpha`/`-beta` suffixes

# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-14 (cont.126) — v0.9.4 bridge release SHIPPED + R2 wiring gap found/fixed

Applied BUILD §3 cutover, shipped **v0.9.4** (bridge), then caught that the R2 dual-publish never fired. Commits: `feat(dist): cut updater to R2 HttpSource feed — v0.9.4 bridge release` (`5fc77ab`, tagged `v0.9.4`) + a follow-up wiring fix (release.yml + CTA).

- **§3 cutover applied** — `update_service.rs` `resolve_manager()` now `HttpSource::new(UPDATE_FEED_URL)` against `https://pub-4fb26c0fc8df484488e4415f112f2d28.r2.dev` (arity confirmed vs velopack 1.2.0: `new<S: AsRef<str>>(url)`). `RIFT_UPDATE_FEED` FileSource hatch untouched. `cargo check` EXIT=0. Version lockstep ×3 + Cargo.lock @ 0.9.4.
- **CI run `27489143952` = success.** GitHub release `v0.9.4` published to `rift-releases` (assets `Rift-win-Setup.exe` 14.2MB, `Rift-0.9.4-full.nupkg`, `releases.win.json`, `RELEASES`). Existing GithubSource clients update from GitHub as designed.
- **⚠️ R2 dual-publish SILENTLY no-op'd** — `release.yml` Release step `env:` only exported `RELEASES_TOKEN`, NOT the 4 `R2_*` secrets, so `$env:R2_ACCESS_KEY_ID` was empty → §2 hit its `else`. CI still green. **R2 bucket is still EMPTY; site CTA 404s.**
- **FIXED for next release:** added `R2_ACCESS_KEY_ID/SECRET_ACCESS_KEY/ENDPOINT/BUCKET` to the Release step `env:` (`release.yml:92`). Also fixed site CTA filename `Setup.exe`→`Rift-win-Setup.exe` (vpk's real artifact name) in `web/index.html`. NOT yet committed at handoff-write — committing now.

### RESUME HERE — what's still pending
1. **Populate R2** — the release.yml fix only applies to the NEXT tag (v0.9.4's tag points at the pre-fix commit; re-running uses old YAML). Options: ship a quick **v0.9.5** to dual-publish into R2, OR manually `vpk upload s3` the v0.9.4 artifacts. Until R2 has objects: site download 404s AND v0.9.4 clients see no further updates.
2. **Roll 2 exposed tokens** (pasted cont.125 chat): R2 S3 token + `cfut_` Pages token. Optional hygiene.

**Decisions (locked):** D1 R2+Pages · D2 domain DEFERRED · D5 single `win` channel. Plan: `docs/design/self-hosted-distribution.md`. Cloudflare provisioned cont.125 (bucket `rift-releases`, CORS GET/HEAD `*`, Pages `rift-5hr.pages.dev`, 4 CI secrets exist on repo).

### Carried-over from cont.123 (v0.9.3 — tail still open)
- **RR-5/#29** CSP prod-verify: install v0.9.3, confirm animations + update bar + 0 CSP violations. · **RR-8** Allow/Deny needs `trust_level=standard`. · **Open:** RR-11 code-signing? RR-12 repo collapse (#17). · **Polish:** single/double-node group card; composer bug re-point.

## Prior arcs — detail in git log + CHANGELOG
cont.123 release-readiness ship-blockers + robustness → **v0.9.3** (tag CI run `27481935945`; RR-1 auth dead-end, RR-2 crash file, T2/T4 sweep). cont.122 re-verify + Activity dock removal (`ced39af`). cont.121 Concept-D tool-group cards → v0.9.2. cont.120 UI Polish → v0.9.1. cont.119 minimal-core strip (−7,407 → 3 workspaces) → v0.9.0. **§7 Harness rebuild still OPEN**. cont.94 Fable 5 (Jun 22 sunset gate). PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH
- **Activity dock is GONE** (cont.122) — don't reintroduce `assistant.ui.dockOpen`/`dockWidth`. Live readout = composer LivePills; context = composer gauge + tabsbar ctx-pill; diff = Ctrl+Shift+D.
- **Tool-group grouping (cont.121):** `coalesceToolGroups` absorbs quick thoughts; threshold = TOOL count. Open = `expandedGroups.has(key) !== defaultOpen` (XOR), stores FLIPPED-from-default keys. Card + left status-rail (`::after`), NOT spine bullet — don't re-add a spine bullet to groups (steps-numbering unify kept this).
- **Live TabState authoritative over disk** — never re-add `stop()` to `loadConversation`.
- **Trust enum 2-level** — `full` rejected for new writes, MIGRATE read-side.
- **Effort mapping lockstep** (`effortToFlag` ↔ `turn.rs` ↔ `modelMatrix.ts` + vitest). **Right-click ownership** (`preventDefault()`).
- **Accent via `--accent-h`** (emerald 163); tint `in oklab`. Surface tiers: page .142 · card .215 · wells .178 · field .25 · track .175.
- **IA: 3 workspaces** (Home·Chat·Settings). **Versions lockstep ×3 + Cargo.lock** — only at ship. **v0.9.2 stands.**

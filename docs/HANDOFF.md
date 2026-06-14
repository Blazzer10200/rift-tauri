# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-13 (cont.125) — Self-hosted distribution BUILD §1+§2 (web site + conditional R2 dual-publish, no ship)

Executed `docs/design/self-hosted-distribution-BUILD.md` §0→§2 autonomously. **No release tagged/shipped, no prod killed, `update_service.rs` UNTOUCHED.** One commit `feat(dist): R2 download site + conditional S3 dual-publish`.

- **§1 — `web/` static download site** (plain HTML/CSS/JS, no build; Pages serves as-is): `index.html`·`styles.css` (Rift tokens)·`app.js` (feed fetch + fallback)·`_headers`·`README.md`. Hero "Claude Code, with a real UI." + 6-feature grid + CTA.
- **§2 — `release.ps1` conditional R2 dual-publish** after GitHub `vpk upload` (`:295-296`), before portable drop. Fires `vpk upload s3` only when R2 env present; else no-op. GitHub path untouched.
### Cloudflare PROVISIONED + serve-path PROVEN (BUILD §H done, same session)
- **Account ID** `1cf0273eb938093158d2c7246719fea8`. **Bucket** `rift-releases` live, public dev URL: `https://pub-4fb26c0fc8df484488e4415f112f2d28.r2.dev`. Smoke-tested: put→public GET = **200** (brief propagation delay possible right after enabling); bucket left **empty/clean**.
- **CORS set on bucket** (`GET`/`HEAD`, origins `*`) so the Pages site's `app.js` feed fetch works cross-origin — verified `Access-Control-Allow-Origin: *` returns. (Updater itself is native Rust = no CORS.)
- **CI secrets on `Blazzer10200/rift-tauri`**: `R2_ACCESS_KEY_ID/SECRET/ENDPOINT/BUCKET`. **vpk `upload s3` flags verified** against live `vpk upload s3 --help` (`-o`/`--channel`/`--keyId`/`--secret`/`--endpoint`/`--bucket` all valid; vpk defaults path-style, R2-OK).
- **web/ placeholders FILLED**; **Pages DEPLOYED** via wrangler → **https://rift-5hr.pages.dev** (project `rift`, branch main, output `web/`). Verified: renders, headers applied, assets 200.

### RESUME HERE — what's still pending
1. **Roll 2 exposed tokens** (both pasted in cont.125 chat): the R2 S3 token + the `cfut_` Pages token. Optional hygiene; rotate when convenient.
2. **Setup.exe in bucket** — NOT yet uploaded (no local build existed). Auto-published to R2 by the next `release.ps1`/CI run via the dual-publish block. Until then the site CTA 404s.
3. **Apply BUILD §3 staged diff** (`update_service.rs` `GithubSource`→`HttpSource`, set URL=`https://pub-4fb26c0fc8df484488e4415f112f2d28.r2.dev`, confirm `HttpSource::new` arity vs velopack 1.2.0, `cargo check` dev-quit), then ship the **bridge release** via the GitHub path — that binary is the first to read R2.

**Decisions (locked, full in plan):** D1 R2+Pages · D2 domain DEFERRED (r2.dev, baked-in URL → domain later = 2nd bridge) · D5 single `win` channel. Plan: `docs/design/self-hosted-distribution.md`.

### Carried-over from cont.123 (v0.9.3 shipped — tail still open)
- **RR-5/#29** CSP prod-verify: install v0.9.3, confirm animations + update bar + 0 CSP violations. · **RR-8** Allow/Deny round-trip needs `trust_level=standard`. · **Decisions:** RR-11 code-signing? RR-12 repo collapse (#17). · **Deferred polish:** single/double-node group card. · **Composer bug** above composer — re-point.

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

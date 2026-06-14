# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-13 (cont.125) — Self-hosted distribution BUILD §1+§2 (web site + conditional R2 dual-publish, no ship)

Executed `docs/design/self-hosted-distribution-BUILD.md` §0→§2 autonomously. **No release tagged/shipped, no prod killed, `update_service.rs` UNTOUCHED.** One commit `feat(dist): R2 download site + conditional S3 dual-publish`.

- **§1 — `web/` static download site created** (plain HTML/CSS/JS, no framework/build; Cloudflare Pages serves dir as-is). Files: `index.html` · `styles.css` (Rift tokens: emerald `--accent-h:163`, surface tiers .142/.215/.178/.25, oklch) · `app.js` (fetches `releases.win.json` for version, try/catch fallback) · `_headers` (nosniff/DENY/strict-origin) · `README.md`. Hero "Claude Code, with a real UI." + 6-feature grid + CTA → `https://pub-REPLACE.r2.dev/Setup.exe`. **Verified:** `npx serve` rendered, HTML + CTA present, styles.css/app.js → 200, server killed by PID.
- **§2 — `release.ps1` conditional R2 dual-publish** inserted after the GitHub `vpk upload` block (now `:295-296`), before the portable-zip drop. Fires `vpk upload s3` only when `R2_ACCESS_KEY_ID`+`R2_SECRET_ACCESS_KEY`+`R2_ENDPOINT` env present; else DarkGray no-op. GitHub path untouched. **Verified:** `[ScriptBlock]::Create` parse = OK.
- **Placeholders left for human:** `pub-REPLACE` in `web/index.html` (CTA href) + `web/app.js` (feed URL).

### RESUME HERE — human Cloudflare click-ops (BUILD §H, ~10 min, cannot be automated)
1. Create R2 bucket `rift-releases`; note Account ID. 2. Enable public access → copy `pub-<hash>.r2.dev` URL. 3. Mint scoped S3 API token (Read&Write) → save Key ID + Secret. 4. Upload current `Setup.exe` to bucket. 5. Connect Pages → repo, build output dir = `web/`, no build cmd. 6. Fill placeholders: `pub-REPLACE` → real hash in `index.html` + `app.js`; add CI secrets `R2_ACCESS_KEY_ID`/`R2_SECRET_ACCESS_KEY`/`R2_ENDPOINT=https://<accountid>.r2.cloudflarestorage.com`/`R2_BUCKET=rift-releases`. 7. **Apply BUILD §3 staged diff** (`update_service.rs` `GithubSource`→`HttpSource`, set real URL, `cargo check`), then ship the bridge release via GitHub path.

**Decisions (locked):** D1 R2+Pages · D2 domain DEFERRED (ship $0 on r2.dev, throttle accepted; baked-in URL → domain later = 2nd bridge) · D5 single `win` channel · D4 site in-repo `web/`. Plan: `docs/design/self-hosted-distribution.md`.

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

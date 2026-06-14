# Self-hosted distribution — BUILD SHEET (autonomous execution)

> Deterministic build checklist for cont.125. **Transcribe, don't re-decide** —
> all architecture decisions are locked in `self-hosted-distribution.md` (read it
> first, + `docs/HANDOFF.md`). This sheet is the ordered work. Every decision
> (D0-D5) is RESOLVED; do not re-litigate. Goal: do all CODE work hands-off,
> verify green, commit, and leave the human a minimal Cloudflare checklist (§H).

## Ground rules (read before touching anything)

- **Do NOT tag or ship a release.** No `git tag`, no `release.ps1` run, no
  `tauri build`. Code + verify + commit only.
- **Do NOT touch `update_service.rs` production source yet.** The `GithubSource`
  → `HttpSource` flip is the *bridge cutover* — it needs the real `r2.dev` URL
  (which doesn't exist until the human makes the bucket) and is a ship action.
  The exact ready-to-apply diff is staged in §3 for the human/next session. Leave
  `update_service.rs` on `GithubSource`.
- **PID-only process kills, NEVER by image name.** `rift-tauri.exe` = both dev
  AND the user's installed app. Don't `taskkill /IM`. (project CLAUDE.md)
- **Don't run `cargo check` while `tauri dev` is alive** (collides). This session
  shouldn't need cargo at all (no Rust prod edits).
- Scope discipline: build exactly §1 + §2. No adjacent refactors.

## 0. Preflight (verify current state matches assumptions)

1. Read `docs/design/self-hosted-distribution.md` (the plan) + `docs/HANDOFF.md`.
2. Confirm `scripts/release.ps1` still has the `vpk upload github` block (grep
   `vpk` / `upload github` / `--repoUrl`). Line refs in this sheet (`:282-296`)
   are from cont.124 — re-anchor by snippet if they drifted.
3. Confirm no `web/` dir exists yet (`Glob web/**`). If it does, STOP and report.

## 1. `web/` static download site (fully autonomous)

**Stack: plain static HTML + CSS + minimal JS. NO framework, NO build step.**
Cloudflare Pages serves the dir as-is (zero-config). Rationale: one marketing
page doesn't need SvelteKit; zero-build = nothing to break in CI.

**Files to create:**
- `web/index.html` — single-page download site.
- `web/styles.css` — Rift design language (see tokens below).
- `web/app.js` — fetches `releases.win.json` to show current version + notes
  (graceful fallback to a static version string if fetch fails / CORS).
- `web/_headers` — Pages headers file (security headers; see below).
- `web/README.md` — one-paragraph "this deploys to Cloudflare Pages, root = `web/`".

**Content (from cont.124 copy work):**
- Hero: **"Claude Code, with a real UI."** + subtext: *"Rift is a native desktop
  coding assistant powered by Claude. It works directly in your local project
  folder — reading files, searching code, running git, all on your machine. No
  servers, no sync. Just a fast, polished app that turns Claude Code into
  something you'd want to look at all day."*
- Primary CTA button: **"Download for Windows"** → href to the R2 `Setup.exe`
  URL. Use placeholder `https://pub-REPLACE.r2.dev/Setup.exe` and a clear
  `<!-- HUMAN: replace pub-REPLACE with real bucket hash (§H) -->` comment.
- Feature grid (6, from cont.124): Works in your real workspace · Multi-tab chats
  (model + effort per chat) · In-app browser dock · Voice input (STT) · Fully
  local · Auto-updating.
- Footer: version (filled by `app.js` from feed), GitHub link, year 2026.

**Design tokens (match Rift — project CLAUDE.md "Accent via `--accent-h`"):**
- Accent hue `--accent-h: 163` (emerald). Use `oklch(L 0.13 163)` for accent.
- Dark theme. Surface tiers (oklch lightness): page `0.142`, card `0.215`,
  wells `0.178`, field `0.25`. Text near-white, muted-grey secondary.
- System font stack; generous spacing; rounded corners; one accent CTA. Tasteful,
  not flashy. Mobile-responsive (single column < 720px).

**`web/_headers` content:**
```
/*
  X-Content-Type-Options: nosniff
  X-Frame-Options: DENY
  Referrer-Policy: strict-origin-when-cross-origin
```

**Verify §1:**
- All files well-formed; no broken relative links.
- Open locally: `npx --yes serve web -l 4321` (background) → fetch
  `http://localhost:4321` with `curl -s | head`, confirm HTML renders + CTA
  present. Kill the server (PID you spawned). If `npx serve` unavailable, validate
  HTML by reading it back; don't block on the server.
- No console-error-inducing JS (wrap the feed fetch in try/catch).

## 2. `release.ps1` — conditional S3 dual-publish (fully autonomous, safe)

**Add, don't replace.** Keep the existing `vpk upload github` block working.
Append a SECOND publish to R2 that fires **only when R2 env vars are present**, so
the current pipeline is unaffected until the human wires secrets.

After the GitHub upload block (`:282-296`, after its `if ($LASTEXITCODE...)`),
insert:
```powershell
# --- Optional: dual-publish to Cloudflare R2 (self-hosted feed) -------------
# Fires only when R2 creds are present (CI secrets). Until then, no-op — the
# GitHub path above remains the live feed. See docs/design/self-hosted-distribution.md.
if ($env:R2_ACCESS_KEY_ID -and $env:R2_SECRET_ACCESS_KEY -and $env:R2_ENDPOINT) {
    Write-Host '=== vpk upload s3 (Cloudflare R2) ===' -ForegroundColor Cyan
    $r2Args = @(
        'upload', 's3',
        '-o', 'Releases',
        '--bucket', $(if ($env:R2_BUCKET) { $env:R2_BUCKET } else { 'rift-releases' }),
        '--endpoint', $env:R2_ENDPOINT,
        '--keyId', $env:R2_ACCESS_KEY_ID,
        '--secret', $env:R2_SECRET_ACCESS_KEY,
        '--channel', 'win'
    )
    & vpk @r2Args
    if ($LASTEXITCODE -ne 0) { throw 'vpk upload s3 (R2) failed' }
} else {
    Write-Host 'R2 env not set — skipping S3 dual-publish (GitHub feed only).' -ForegroundColor DarkGray
}
```
Notes:
- Single `win` channel (D5). No `--pre` / channel-select logic.
- Reuses the same `-o Releases` output dir the GitHub upload already produced —
  pack already ran once; this just uploads the same artifacts to a second target.
- Keep it AFTER the GitHub upload + BEFORE the portable-zip drop step.

**Verify §2:**
- PowerShell syntax check (no execution):
  `powershell -NoProfile -Command "$null = [ScriptBlock]::Create((Get-Content -Raw scripts/release.ps1)); 'OK'"`.
  Must print `OK`. (Parses without running.)
- Re-read the edited region to confirm the block sits in the right place and the
  surrounding blocks are intact.

## 3. STAGED (do NOT apply) — `update_service.rs` cutover diff for the human

This is the bridge-release change. Apply ONLY after the R2 bucket exists and its
`pub-<hash>.r2.dev` URL is known (§H). Recorded here so it's a copy-paste later.

Replace (imports `:29-37`):
```rust
use velopack::sources::GithubSource;
const GITHUB_REPO_URL: &str = "https://github.com/Blazzer10200/rift-releases";
const ALLOW_PRERELEASE: bool = true;
```
with:
```rust
use velopack::sources::HttpSource;
const UPDATE_FEED_URL: &str = "https://pub-REPLACE.r2.dev"; // ← real bucket URL
```
And in `resolve_manager()` (`:335`) replace:
```rust
let src = GithubSource::new(GITHUB_REPO_URL, None, ALLOW_PRERELEASE);
```
with:
```rust
let src = HttpSource::new(UPDATE_FEED_URL);
```
Keep the `RIFT_UPDATE_FEED` FileSource hatch (`:314-332`) untouched. Then
`cargo check` (dev quit), bump version, ship the bridge release via the **GitHub**
path (existing clients still read GitHub) — that shipped binary is the first to
read R2. ⚠️ Confirm `HttpSource::new` arity against `velopack` 1.2.0 at apply time.

## H. HUMAN HANDOFF — Cloudflare click-ops (~10 min, cannot be automated)

Do these in the Cloudflare dashboard, then fill the placeholders the build left.

1. **Create R2 bucket** `rift-releases` (R2 → Create bucket). Note the **Account
   ID** (R2 overview / right sidebar).
2. **Enable public access** → Settings → "Public Development URL" → Allow. Copy
   the `https://pub-<hash>.r2.dev` URL.
3. **Mint S3 API token** (R2 → Manage R2 API Tokens → Create, Object Read & Write,
   scoped to `rift-releases`). Save **Access Key ID** + **Secret**.
4. **Upload current `Setup.exe`** to the bucket (drag-drop) so the download page
   has something to link immediately.
5. **Connect Pages** → Workers & Pages → Create → Pages → connect the repo (or
   "Direct Upload"), set **build output dir = `web/`**, no build command. Deploy.
6. **Fill placeholders the autonomous build left:**
   - `web/index.html` — `pub-REPLACE` → real bucket hash (CTA href).
   - CI secrets (GitHub Actions, on the repo running `release.yml`): add
     `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`,
     `R2_ENDPOINT=https://<accountid>.r2.cloudflarestorage.com`,
     `R2_BUCKET=rift-releases`. (Can use `gh secret set` if `gh` is authed.)
7. **Apply §3 diff** (`update_service.rs`), set `UPDATE_FEED_URL` to the real
   `pub-<hash>.r2.dev`, `cargo check`, then ship the bridge release.

## STATUS — cont.125 (autonomous build + human §H, same session)

§1+§2 built & verified. §H Cloudflare click-ops then completed live (user pasted
creds; assistant scripted the rest): bucket + public URL up, 4 CI secrets set,
web placeholders filled, Pages **deployed → https://rift-5hr.pages.dev**, serve
path smoke-tested (200), vpk `upload s3` flags verified vs live `--help`.

**Extra step not in the original sheet — bucket CORS.** `app.js` fetches
`releases.win.json` cross-origin (pages.dev → r2.dev); R2 public URLs send no CORS
headers by default, so the version display would silently fall back. Fixed: set
bucket CORS (`GET`/`HEAD`, origin `*`), verified `Access-Control-Allow-Origin: *`.
(The native Rust updater is unaffected — CORS is browser-only.) If the bucket is
ever recreated, re-apply this CORS policy.

**Still pending (human/next session):** `Setup.exe` auto-lands on next CI release;
§3 `update_service.rs` cutover ships as the bridge release; roll the 2 creds
pasted in chat.

## Done-criteria for the autonomous session

- [x] `web/` site created, all files well-formed, renders locally, CTA + features
      present, design tokens applied.
- [x] `release.ps1` has the conditional S3 dual-publish block; `[ScriptBlock]`
      parse = OK; GitHub path untouched.
- [x] `update_service.rs` UNCHANGED (cutover staged in §3 only).
- [x] No release tagged/shipped; no prod process killed.
- [x] CHANGELOG.md gets a one-line entry; HANDOFF.md updated (cont.125) within the
      600-word cap; this BUILD sheet's done-criteria checked.
- [x] Single commit: `feat(dist): R2 download site + conditional S3 dual-publish`.
- [x] Final report lists exactly what's left for the human (§H), nothing more.

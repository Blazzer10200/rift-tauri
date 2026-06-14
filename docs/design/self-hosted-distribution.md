# Self-hosted distribution — own the download page + update feed (Cloudflare R2 + Pages)

> Status: **PLAN / RESEARCHED (2026-06-13, rev.2). Architecture decided:**
> Cloudflare **R2** (update feed + binaries) + Cloudflare **Pages** (download
> website). **$0/month** — no VPS, no home-server exposure, no CGNAT gate. Off
> GitHub for delivery; build compute stays on the existing self-hosted CI runner.
> Optional ~$10/**year** domain removes the one soft limit (r2.dev throttle).
> **Supersedes rev.1** (pure home self-host / VPS — both demoted to §Rejected).

## 0. Why this shape

Goal unchanged from rev.1: move Rift's download + auto-update delivery **off
GitHub** for independence + a real product front door. GitHub has no current cost
path (private repos free, CI on self-hosted runner, release bw free) — this is
about ownership, not a bill.

rev.1 chose pure home self-host (CGNAT gate, home IP exposed, home-uplink
bandwidth) and a VPS was floated ($4.50/mo). The constraint is now **$0/month**.
R2 + Pages is not a downgrade to hit that number — it is **strictly better** than
both: free, zero-maintenance, datacenter reliability, no home IP exposed, no
CGNAT, and TOS-clean for large binaries (the plain Cloudflare-CDN/Tunnel route is
*not* — see §Rejected).

## 1. The free-tier math (verified, primary sources)

**R2 free tier** (per month, [r2/pricing](https://developers.cloudflare.com/r2/pricing/), updated 2026-05-28):

| Resource | Free allotment | Rift's realistic usage |
|---|---|---|
| Storage | **10 GB-month** | ~1–2 GB (a few recent full `.nupkg`s + Setup.exe, rotated) |
| Class A ops (writes/uploads) | **1M / mo** | a handful per release |
| Class B ops (reads/GET) | **10M / mo** | a few thousand (update polls + downloads) |
| **Egress (data out)** | **Free, always — every tier** | the whole game |

Egress is what makes binaries expensive everywhere else; on R2 it is free on every
tier. A desktop updater for a small user base sits in the free tier
**indefinitely** — the only documented way to blow Class B is *hundreds of millions*
of reads/mo (CF's own "asset hosting" example: 300M reads/mo → $104), which an
updater never approaches.

**Cloudflare Pages free plan** ([pages/platform/limits](https://developers.cloudflare.com/pages/platform/limits/), updated 2026-04-21): unlimited
bandwidth/requests, 500 builds/mo, 20,000 files/site, **25 MiB max per file**,
1 concurrent build. The 25 MiB cap is irrelevant — binaries live on R2, not Pages.
CF's own docs say: *"To serve larger files, consider uploading them to R2."*

## 2. Target architecture

```
  ┌─ Download site ──────────────────────────────────────────────────────────┐
  │  Cloudflare Pages   rift.pages.dev  (free, unlimited bw, deploy from web/) │
  │     "Download for Windows" → links the Setup.exe on R2 (below)             │
  └───────────────────────────────────────────────────────────────────────────┘
  ┌─ Update feed + binaries ─────────────────────────────────────────────────┐
  │  Cloudflare R2 bucket  "rift-releases"                                     │
  │    • SERVE (read, client):  https://pub-<hash>.r2.dev   (public, unauth)   │
  │        └ releases.win.json + *.nupkg + *Setup.exe  (flat dir)             │
  │    • WRITE (publish, CI):   https://<accountid>.r2.cloudflarestorage.com   │
  │        └ S3 API + access keys, used by `vpk upload s3`                     │
  └───────────────────────────────────────────────────────────────────────────┘
              ▲ publish (S3)                         ▲ read (HTTPS, HttpSource)
  ┌───────────┴──────────────┐              ┌────────┴───────────────────────┐
  │ Home Proxmox CI runner    │              │ Rift client                     │
  │  on tag: vpk pack         │              │  HttpSource::new(pub-…r2.dev)   │
  │       → vpk upload s3      │              │  → releases.win.json → .nupkg   │
  │  (full-only, --delta None)│              │  → existing check/download/apply│
  └───────────────────────────┘              └─────────────────────────────────┘
```

**Two URLs, one bucket — do not conflate:**
- **Read/serve URL** (baked into the app): the *public* bucket URL
  `https://pub-<hash>.r2.dev` (unauthenticated, rate-limited — §5) or a custom
  domain. This is what Velopack `HttpSource` fetches.
- **Write/publish URL** (CI only): the *S3 API* endpoint
  `https://<accountid>.r2.cloudflarestorage.com` with an R2 access key/secret.
  `vpk upload s3` writes here. Never baked into the client.

## 3. The app code change — `src-tauri/src/update_service.rs`

Today (imports `:29-37`; `resolve_manager()` `:314-339`, swap point `:335`):
```rust
use velopack::sources::GithubSource;                                    // :29
const GITHUB_REPO_URL: &str = "https://github.com/Blazzer10200/rift-releases"; // :34
const ALLOW_PRERELEASE: bool = true;                                    // :37
// …
let src = GithubSource::new(GITHUB_REPO_URL, None, ALLOW_PRERELEASE);   // :335
```
After:
```rust
use velopack::sources::HttpSource;
const UPDATE_FEED_URL: &str = "https://pub-<hash>.r2.dev"; // or https://updates.getrift.com once a domain is bound
// …
let src = HttpSource::new(UPDATE_FEED_URL);
```
Notes (all verified against `velopack` crate `=1.2.0`):
- `HttpSource::new<S: AsRef<str>>(url)` — **single arg**, base URL of the bucket
  root. No prerelease bool, no token. ([docs.rs/velopack/1.2.0](https://docs.rs/velopack/1.2.0/velopack/sources/struct.HttpSource.html))
- It fetches `{base}/releases.{channel}.json` then `{base}/{asset}`. ⚠️ The
  rustdoc summary saying it hits `{base}/RELEASES` is **stale** (legacy Squirrel
  endpoint) — the actual `http.rs` source builds `releases.{channel}.json`.
- **Prerelease semantics change:** `GithubSource(prerelease:bool)` → `HttpSource`
  selects by **channel** baked in at `vpk pack --channel <name>` time. Keep
  alpha/beta by publishing a `win-pre` channel (§4) and pointing opted-in clients
  at it via `UpdateOptions { ExplicitChannel }` (D5). The `ALLOW_PRERELEASE`
  const is dropped.
- **Keep the `RIFT_UPDATE_FEED` → `FileSource` dev/test hatch** (`:321-332`,
  gated `#[cfg(any(debug_assertions, feature = "update-test-feed"))]`) exactly
  as-is — `scripts/test-update.ps1` depends on it.
- `UpdateInfoDto` / `info.TargetFullRelease` DTO mapping unchanged.
- Verify: `cargo check` **with dev quit** (project rule — tauri dev's watcher
  collides with an external cargo check).

## 4. Publish pipeline (CI runner) — one swap in `release.ps1`

The pack stage stays **byte-for-byte identical** — `release.ps1` already runs
`vpk pack -u Rift -v <ver> -p <staging> -e rift-tauri.exe -c win --packTitle Rift
--packAuthors Blazzer --icon … --delta None -o Releases` (`release.ps1:239-253`).
The **only** change is the publish target: replace the `vpk upload github` block
(`release.ps1:282-296`) with `vpk upload s3`. Velopack's `vpk … s3` provider takes
`--endpoint` for any S3-compatible store (docs give BackBlaze B2; R2 is the same
shape — endpoint `https://<accountid>.r2.cloudflarestorage.com`, verified
[r2/api/s3](https://developers.cloudflare.com/r2/api/s3/api/)). ([velopack deploy-cli](https://docs.velopack.io/distributing/deploy-cli))

```bash
# (pack stage unchanged — release.ps1:239-265, still --delta None -o Releases)

# Replaces the `vpk upload github` block:
vpk upload s3 --bucket rift-releases \
  --endpoint https://<accountid>.r2.cloudflarestorage.com \
  --keyId $R2_ACCESS_KEY_ID --secret $R2_SECRET_ACCESS_KEY \
  --channel win
```
- ⚠️ **Deltas are OFF today** (`--delta None`, `release.ps1:252` — full-only by
  deliberate choice). So the docs' `vpk download s3` (pull-priors-first) step is
  **NOT required** for Rift: with full-only, clients just fetch the newest full.
  Add a `vpk download s3 …` before `pack` *only* if you (a) re-enable deltas, or
  (b) want the regenerated `releases.win.json` to retain older-version entries.
  For a clean full-only feed, `pack` + `upload s3` alone is sufficient.
- **Prerelease handling changes:** today `release.ps1:292-294` appends `--pre`
  for `-alpha/-beta/-rc` (GitHub prerelease flag). `HttpSource` has no prerelease
  bool — prereleases become a separate **channel** (`-c win-pre` at pack +
  `--channel win-pre` at upload). So the `--pre` branch is replaced by
  channel-selection logic (D5). Stable stays `win`.
- R2 region is `auto` (empty/`us-east-1` alias to it) — `--endpoint` is what
  matters, no `--region` needed.
- Keys = an R2 API token scoped to this bucket only, passed as a CI secret (same
  pattern as today's `RELEASES_TOKEN`). Never inline; never in the client. Every
  `vpk` flag also has an env-var form (`vpk -H`).
- **Backup manual path** if `vpk … s3` ever misbehaves: `vpk pack` locally, then
  `wrangler r2 object put` per file (supports **up to 315 MB/file**, one at a
  time — covers our ~100–150 MB binaries; rclone for bulk). S3 path is primary.

## 5. The one $0 caveat — r2.dev throttle

The free public bucket URL `https://pub-<hash>.r2.dev` is **rate-limited** and CF
labels it *"development use only … variable rate limit"*
([public-buckets](https://developers.cloudflare.com/r2/buckets/public-buckets/),
[limits](https://developers.cloudflare.com/r2/platform/limits/)). For a small user
base it works to launch; the throttle is a soft cap, not a wall.

Removing it requires a **custom domain** bound to the bucket, which needs a domain
on Cloudflare (free DNS) — i.e. owning a domain (~$10/**year**, the only money in
this whole plan, and optional):
- **$0 today:** ship on `pub-<hash>.r2.dev`, accept the soft throttle.
- **Later (~$10/yr):** register a domain, bind `updates.getrift.com` →
  bucket; throttle gone, nicer URL; point Pages at `getrift.com` too.

⚠️ The base URL is **baked into the client binary**, so switching r2.dev → custom
domain later needs a **bridge release** (§6) — the same one-time dance as any
updater-source swap. If a domain is even remotely likely, prefer registering it
*before* the first `HttpSource` ship to avoid a second bridge.

## 6. Migration — one-time bridge (same shape as every prior updater swap)

Existing clients fetch via `GithubSource`/`rift-releases`; they won't see the R2
feed until running a binary built with `HttpSource`. Ship **one final release
through the current GitHub path** whose binary contains the `HttpSource` switch;
from then on clients self-update off R2. Keep `rift-releases` read-only for a few
versions for stragglers. (Identical to the Velopack↔tauri-plugin-updater bridges —
`velopack-auto-update.md` §5.)

## 7. Channels (D5)

`releases.{channel}.json`; default channel is the OS short name (`win`) when
`--channel` is omitted. Stable + prerelease run **side by side** from the same
bucket: pack/upload with `-c win` and `-c win-pre`; each gets its own
`releases.<ch>.json`. By default a client reads the channel baked in at its own
`vpk pack` time. To let users opt into `win-pre`, override at runtime —
documented via `ExplicitChannel` + `AllowVersionDowngrade` on `UpdateOptions`
(the docs show C#/JS; ⚠️ **confirm the exact Rust `UpdateOptions` field names
against [docs.rs/velopack/1.2.0](https://docs.rs/velopack/1.2.0/velopack/) before
coding** — Rust-side docs are thin). Keep the channel name lockstep across
`release.ps1`, the feed filename, and the client.
([channels](https://docs.velopack.io/packaging/channels))

## 8. Phased rollout (each phase independently shippable)

1. **R2 bucket** — create `rift-releases`, enable public access (note the
   `pub-<hash>.r2.dev` URL), mint a scoped S3 API token. Manually upload the
   *current* Setup.exe to smoke-test public reads.
2. **Download site** — build static site in a new in-repo `web/` dir (Rift design
   language: emerald `--accent-h` 163, surface tiers per project CLAUDE.md);
   deploy to Pages; "Download for Windows" → the R2 Setup.exe. Visible win, zero
   app risk.
3. **CI publish** — add the `vpk download/pack/upload s3` steps to `release.yml`
   (R2 token as a repo secret); **dual-publish** to GitHub + R2 for 1–2 releases;
   validate via `scripts/test-update.ps1` pointed at the r2.dev URL.
4. **(Optional) domain** — if buying one, register + bind custom domains **before**
   the bridge so the baked-in URL is final (§5).
5. **Flip the app** — land the `HttpSource` change (§3); ship the **bridge
   release** through GitHub (§6).
6. **Decommission** — once clients are on R2, drop the GitHub publish step (keep
   `rift-releases` repo as read-only backup).

## 9. Risks

- **R1 — r2.dev throttle.** Soft cap on the free URL; mitigated by a custom
  domain (§5). Not a hard blocker at small scale.
- **R2 — baked-in URL → bridge cost.** Switching the serve URL later needs a
  bridge release (§6). Decide r2.dev-vs-domain before first `HttpSource` ship.
- **R3 — Cloudflare dependence.** Trades GitHub-dependence for CF-dependence
  (still "someone else's cloud"). Accepted: it's off GitHub, free, TOS-clean, and
  a real front door. True hardware ownership = home self-host (§Rejected), which
  costs the CGNAT/exposure/uptime tradeoffs we're declining.
- **R4 — channel migration.** Prerelease bool → channel string; keep lockstep
  (§7).
- **R5 — bootstrap.** `HttpSource` only engages after clients run the bridge
  build (§6). Don't kill `rift-releases` early.
- **R6 — free-tier ceiling.** Only Class B (reads) could theoretically bill at
  extreme scale; nowhere near reachable for a desktop updater. Storage stays
  under 10 GB by rotating old releases (full-only — prune stale `.nupkg`s).

## 10. Decisions

**Resolved (2026-06-13 rev.2):**
- **D1 — hosting = Cloudflare R2 (feed/binaries) + Pages (site).** Free,
  TOS-clean, no home exposure, no CGNAT. (Supersedes rev.1 home/VPS.)
- **D0 — CGNAT check = N/A.** No longer a blocker; nothing is home-hosted.
- **D3 — Gitea deferred** (source stays on GitHub).
- **D4 — website home = in-repo `web/`.**
- **D5 — single `win` channel.** Matches current `release.ps1` (`-c win`); least
  to build; `win-pre` is non-breaking to add later (new feed + opt-in toggle) if
  beta testers ever materialize. No second channel from day one.

**Still open:**
- **D2 — domain.** Deferred — ship $0 on `pub-<hash>.r2.dev` for now (user
  decision 2026-06-13). Optional ~$10/yr later removes the r2.dev throttle (§5);
  adding it costs a 2nd bridge release (URL is baked in — risk R2).

## Rejected alternatives (was rev.1's plan + others considered)

- **Pure home self-host** (Caddy on home Proxmox, port-forward :80/:443):
  free + fully owned, but CGNAT can hard-block it, exposes the home IP, ties
  uptime to home power/ISP, and caps download speed at the home uplink. R2 erases
  all four for $0.
- **Cheap VPS** (Hetzner CX22 ~$4.50/mo): clean and reliable, but costs money and
  needs OS/Caddy maintenance + disk rotation. R2 is free and zero-maintenance.
- **Home + VPS relay** (frp/WireGuard): strictly dominated — same VPS cost, more
  moving parts, still home-uplink-capped.
- **Tailscale Funnel:** undocumented bandwidth throttle, beta, forces a `*.ts.net`
  URL. Not a production distribution channel.
- **Cloudflare CDN / Tunnel for the binaries (non-R2):** **TOS-prohibited** —
  CF's Service-Specific Terms (updated 2026-06-09) bar serving large files through
  the CDN without a paid product. R2 *is* the compliant product, and it has a free
  tier — which is exactly why this plan uses R2, not a CF-proxied origin.

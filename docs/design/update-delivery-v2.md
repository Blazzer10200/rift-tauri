# Update delivery v2 — channels, smart feed, break-glass publish

> Status: **proposed** (phase 1 shipped 2026-07-16). Current release behavior is
> documented in `docs/DEVELOPING.md#releases`.

## Where we are (phase 1 — DONE)

- Live update feed = Cloudflare R2 public bucket (`update_service.rs::UPDATE_FEED_URL`);
  GitHub Releases is only the human download page.
- `release.ps1` publishes **feed-first**: R2 before GitHub, GitHub upload retried 3×,
  delta baseline pulled from the R2 feed itself (`vpk download http`). The client-critical
  path is fully GitHub-free. Motivation: the 2026-07-16 GitHub outage killed the
  GitHub-first pipeline *before* the R2 step, stranding clients on v0.117.0 with a
  green build sitting on the runner.

## Phase 2 — proposed, in priority order

Each item needs owner action in the Cloudflare dashboard (the local
`CLOUDFLARE_API_TOKEN` in `.secrets/env.sh` has **no R2 permission** — verified
2026-07-16, R2 API returns auth error).

### 2a. Break-glass local publish (~5 min owner setup, do first)

Create an R2 API token (Cloudflare dash → R2 → Manage API Tokens → Object Read & Write
on `rift-releases`) and add to `C:/AI Workflow/.secrets/env.sh`:
`R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `R2_ENDPOINT`
(`https://<account_id>.r2.cloudflarestorage.com`). `release.ps1` already reads them —
a local run then publishes the real feed from the dev box. This is the "GitHub is down
AND the runner is down" escape hatch; during the v0.118.0 outage a local publish was
attempted and blocked ONLY by these missing creds.

### 2b. Release channels (beta → stable)

Velopack supports channels natively (`vpk pack -c win-beta`; client picks via
`UpdateOptions::ExplicitChannel`). Flow: every tag ships to `win-beta` first; owner
smoke-tests on the real install; a cheap promote step (copy nupkg + regenerate
`releases.win.json` for `win`) graduates it. App side: a "Beta updates" toggle in
Settings mapping to the channel name. Owner becomes the canary instead of users.
Cost: one extra manual step per release; pipeline stays one workflow.

### 2c. Smart feed — Cloudflare Worker in front of R2

Custom domain (e.g. `updates.rift.dev`) → Worker → R2 bucket. The Worker serves
`releases.win.json` and can therefore decide per-request:
- **Kill switch**: bad release → flip a KV flag → feed instantly reverts to the previous
  version, no CI run, no re-tag.
- **Staged rollout**: hash the client IP/install-id → only N% see the new version on day 1.
- **Adoption stats**: count update checks per version (currently zero visibility).
Client change: `UPDATE_FEED_URL` → the custom domain (keeps `pub-*.r2.dev` reachable as
fallback for old builds). Ship 2c only after 2b — a kill switch matters most once
releases go straight to everyone, and channels reduce that pressure first.

### Explicitly rejected

- Moving CI off GitHub Actions (Gitea/Buildkite/etc.): build half already runs on our
  own hardware for free; migration = churn, no new capability.
- Package managers (winget/scoop) as *update* path: fine as discovery/install channels
  later, but Velopack must stay the single updater (two updaters fight over `current/`).

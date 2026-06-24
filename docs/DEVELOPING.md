# Developing + running Rift

One doc for everything between "I want to use this" and "I want to contribute." Sections:

1. [End-user install (Onboarding)](#1-end-user-install-onboarding)
2. [Building from source (Contributing)](#2-building-from-source-contributing)
3. [Claude Code w/ Rift on the Pro plan](#3-claude-code-w-rift-on-the-pro-plan)
4. [Releases](#4-releases)
5. [Configuration & environment variables](#5-configuration--environment-variables)

---

## 1. End-user install (Onboarding)

For someone handed a `Setup.exe` who wants to start coding with Claude against a local folder.

### Install

- Run `Rift-Setup.exe`. Per-user install, no admin required. Lands at `%LOCALAPPDATA%\Rift\rift-tauri.exe` w/ a Start-menu shortcut.
- SmartScreen flag → **More info → Run anyway** (no code-signing cert yet).

### Sign in to Claude

Rift drives the `claude` CLI, so it uses whatever auth the CLI has.

- Easiest: install Claude Code (`npm install -g @anthropic-ai/claude-code`), run `claude` once, and complete the browser login (Pro/Max/Team) — see §3.
- Or add an `ANTHROPIC_API_KEY` in **Settings → API key**; Rift passes it to the CLI per turn.
- The auth pill in the composer goes green when the CLI can reach Claude.

### Pick a workspace + chat

- **Open a folder** (the workspace picker). Everything Claude does is scoped to that folder via Rift's local MCP server.
- Type in the composer and send. Claude can `read_file` / `list_dir` / `grep` and run local git (`git_status` / `diff` / `log` / `pull` / `commit` / `push`) against the folder — all on your machine, no remote connections.
- **Per-turn controls** sit on the composer: model (sonnet/opus/haiku), permission mode, thinking effort.
- **Permission modes** — ask-before-edits, edit-automatically, plan, auto, or bypass. In the asking modes a gated tool surfaces an Allow/Deny bar before it runs.
- **Tabs / panes** — open multiple concurrent chats; each carries its own model + permission mode + effort.

### Updates

Rift self-updates via Velopack. It checks on launch + every ~6h; when a build is available you click **Update** once to consent, then it downloads in the background and applies on the next restart — unattended from there.

### Trouble?

- The CLI not found / not signed in → the auth pill explains it; install/parse `claude` (§3) or add an API key in Settings.
- CLI logs/auth live under `~/.claude/`. Rift's own config is managed in-app via Settings.

---

## 2. Building from source (Contributing)

### Prerequisites

- **Windows 11** (primary target). macOS / Linux build but aren't packaged.
- **Rust** stable (latest) via [rustup](https://rustup.rs/). CI tracks `@stable` (no pinned minimum).
- **Node.js** 20+ via [`nvm-windows`](https://github.com/coreybutler/nvm-windows).
- **`npm`** — *not* pnpm (lockfile is npm).
- **Git Bash** for shell scripts.

### Clone + bootstrap

```bash
git clone https://github.com/Blazzer10200/rift-tauri.git
cd rift-tauri
npm install
```

First `cargo` build is a few minutes cold (Tauri + WebView2 toolchain). Incremental: seconds.

### Run dev

```bash
scripts/run-dev.bat        # red-tinted icon, separate WebView2 profile from installed Rift; also exposes CDP on localhost:9222
# or
npm run tauri dev
```

Dev watches `src/` + `src-tauri/src/` and hot-reloads. **Don't run `cargo check` while dev is alive** — it collides with dev's incremental rebuild and kills the running dev process. Quit dev first, or rely on the dev console (it IS the Rust verifier while running).

### Project layout

| Where | What |
|---|---|
| `src/` | SvelteKit frontend (Svelte 5 runes, Tailwind 4) |
| `src-tauri/src/` | Rust backend — `assistant/` (Claude CLI spawn + MCP server + local git), `browser/`, `commands/`, `diagnostics/`, `state/`, `stt/`, plus `lib.rs` / `update_service.rs` / `secrets.rs` |
| `src-tauri/capabilities/` | Tauri 2 permission grants |
| `docs/` | Live state — read `HANDOFF.md` first each session |
| `scripts/` | Dev launcher + release pipeline + CDP helpers |

### Before opening a PR

- `npm run check` clean
- `cargo check --manifest-path src-tauri/Cargo.toml` clean (dev quit first)
- `cargo test` if you touched anything testable
- Don't bump versions — the release pipeline handles `package.json` / `Cargo.toml` / `tauri.conf.json` in lockstep.

---

## 3. Claude Code w/ Rift on the Pro plan

Pro-plan-optimized setup for Claude Code (the CLI Rift drives via MCP). ~10 min.

### Install + auth

```bash
npm install -g @anthropic-ai/claude-code
claude  # first run: opens browser for Pro auth
```

Verify: `claude --version` v2.1.111+ and `claude config` shows `model: claude-sonnet-4-6`.

### `~/.claude/settings.json`

```json
{
  "$schema": "https://json.schemastore.org/claude-code-settings.json",
  "model": "claude-sonnet-4-6",
  "autoUpdatesChannel": "stable",
  "effortLevel": "medium",
  "env": {
    "CLAUDE_CODE_GIT_BASH_PATH": "<your Git bash.exe, e.g. C:\\Program Files\\Git\\bin\\bash.exe>",
    "CLAUDE_CODE_SUBAGENT_MODEL": "claude-haiku-4-5",
    "CLAUDE_CODE_AUTO_COMPACT_WINDOW": "250000",
    "CLAUDE_AUTOCOMPACT_PCT_OVERRIDE": "80"
  },
  "skillOverrides": {
    "plan": "user-invocable-only",
    "quick-review": "user-invocable-only",
    "diagnose": "user-invocable-only"
  },
  "permissions": {
    "deny": ["Read(./.env)", "Bash(curl *)"]
  }
}
```

**Why each line:**
- Sonnet 4.6 default — handles 90%+ of coding at ~3× lower quota burn than Opus.
- Haiku for subagents — recon/grep agents fire at ~5% of Sonnet cost.
- `effortLevel: medium` — caps output ~2500 tok. Xhigh burns the 5-hour window ~3× faster.
- `CLAUDE_CODE_AUTO_COMPACT_WINDOW: "250000"` + `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE: "80"` — bound + trigger the CLI's own auto-compaction at 80% of a 250K window. Pro plan can't afford the 300K cliff. (This is the `claude` CLI's built-in compaction — Rift's in-app compaction UI was removed in the 2026-06-12 minimal-core strip.)
- Skill overrides — fork-mode skills user-invocable-only; auto-firing burns 10-30K tokens each.
- `autoUpdatesChannel: stable` — avoids regression releases (v2.1.89+ caused 3-50x quota burn for some).
- Opus on-demand only — every Opus turn competes w/ your Sonnet budget. `/model claude-opus-4-8` per-session, then `/model claude-sonnet-4-6` back.

### Usage tracking

- In-CLI: `/status` shows remaining allocation for the current 5-hour window.
- Web: `https://claude.ai/settings` → Usage (browser chat + CLI share the same pool).
- Limit resets on rolling 5-hour windows, NOT weekly.

### Rift + Claude integration

Rift's Assistant shells `claude` with `--mcp-config <rift.mcp.json>` + `--allowed-tools mcp__rift__*`, so Claude inherits `$HOME` and the Pro login works automatically. Per-turn it passes `--session-id` (first turn) / `--resume` (thereafter) so the CLI owns conversation state.

**Session resume:** auto-recovers from "No conversation found with session ID" — you'll see "Session was lost — retrying as a fresh start," that's expected.

### Common pitfalls

- Shared quota: claude.ai browser eats the same pool as the CLI.
- Silent fallback: over-limit may downgrade silently — check `/status` if output quality drops.
- Don't `EnterPlanMode` — use the `/plan` skill if you have it; otherwise describe + execute.

---

## 4. Releases

Maintainers only. Versions bumped manually (or via `/git-ship`) across all three files (`package.json` + `Cargo.toml` + `tauri.conf.json`) BEFORE `scripts/release.ps1` runs — preflight bails on any mismatch (and on a dirty tree, which also catches an un-committed `Cargo.lock` after a version bump).

`release.ps1` drives `tauri build` → Velopack pack (`vpk`) → publish to the public `Blazzer10200/rift` repo (renamed from `rift-releases` at v0.16.2), with a SHA256 round-trip verify. **The `vpk` CLI version MUST equal the `velopack` crate version** (both pinned `=1.2.0`) — bump them together (`dotnet tool update -g vpk` + the Cargo pin). Full update flow + lineage: `git log -- docs/design/velopack-auto-update.md` (arc doc retired after ship).

### Ship flow + guard rails

The tag-driven `release.yml` now **runs the full test suite (`cargo test` + `svelte-check` + `vitest`) before it builds/publishes** — a tag can no longer ship code whose tests are red (the v0.31.0 failure mode). Two optional helpers around a ship:

- `pwsh scripts/smoke-turn.ps1 -Model haiku` — **before** tagging, prove a real Claude turn still completes end-to-end (spawns `claude` with Rift's exact turn flags against a throwaway folder; ~a cent of quota). Covers the live-turn check that CDP can't.
- `pwsh scripts/ship-watch.ps1` — **after** `git push --tags`, blocks on the release run and reports green/red (exit-status mirrors the run). Replaces the manual "confirm CI landed next session" step.

---

## 5. Configuration & environment variables

**End users need ZERO environment variables.** Rift is self-contained: secrets (an optional `ANTHROPIC_API_KEY`) live in the OS keychain via Settings, never in env or files; app config is written to `~/.rift/`; downloaded Whisper STT models land in `~/.rift/models/`. Nothing reads a machine-specific path or port at runtime.

Every variable below is **optional** and scoped to development or release tooling:

| Variable | Scope | Purpose |
|---|---|---|
| `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` | dev | Set by `scripts/run-dev.bat` to expose CDP on `localhost:9222` for live-UI verification. Not used in prod. |
| `WEBVIEW2_USER_DATA_FOLDER` | dev | Isolates the dev WebView2 profile from an installed Rift so the two don't share cookies/state. |
| `RIFT_CDP_MAX_EDGE` | dev | Overrides the 1280px screenshot long-edge clamp in `scripts/cdp/serve.cjs`. Cosmetic. |
| `RIFT_MCP_SERVER` | internal | Set by Rift on itself when it re-spawns as the stdio MCP child for a turn. **Do not set manually.** |
| `RELEASES_TOKEN` | CI/release | Fine-grained PAT (`Blazzer10200/rift` Contents:write) used by the tag-driven release workflow. Repo secret, never local. |

CLI-side knobs (`CLAUDE_CODE_*`, `ANTHROPIC_API_KEY`) belong to the `claude` CLI, not Rift — see §3. Rift actively **strips** `ANTHROPIC_API_KEY` from the CLI's environment on every turn so the in-app keychain key (or the CLI's own browser login) is the single source of auth truth.

# Developing + running Rift

One doc for everything between "I want to use this" and "I want to contribute." Sections:

1. [End-user install (Onboarding)](#1-end-user-install-onboarding)
2. [Building from source (Contributing)](#2-building-from-source-contributing)
3. [Claude Code w/ Rift on the Pro plan](#3-claude-code-w-rift-on-the-pro-plan)
4. [Releases](#4-releases)

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
- **Rust** stable (1.78+) via [rustup](https://rustup.rs/).
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
    "CLAUDE_CODE_GIT_BASH_PATH": "C:\\Program Files\\Git\\bin\\bash.exe",
    "CLAUDE_CODE_SUBAGENT_MODEL": "claude-haiku-4-5",
    "DISABLE_AUTO_COMPACT": "0"
  },
  "skillOverrides": {
    "plan": "user-invocable-only",
    "quick-review": "user-invocable-only",
    "diagnose": "user-invocable-only",
    "dream": "user-invocable-only"
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
- `DISABLE_AUTO_COMPACT: "0"` — let Claude Code auto-compact at ~83% ctx. Pro plan can't afford the 300K cliff.
- Skill overrides — fork-mode skills user-invocable-only; auto-firing burns 10-30K tokens each.
- `autoUpdatesChannel: stable` — avoids regression releases (v2.1.89+ caused 3-50x quota burn for some).
- Opus on-demand only — every Opus turn competes w/ your Sonnet budget. `/model claude-opus-4-7` per-session, then `/model claude-sonnet-4-6` back.

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

`release.ps1` drives `tauri build` → Velopack pack (`vpk`) → publish to the public `rift-releases` repo, with a SHA256 round-trip verify. **The `vpk` CLI version MUST equal the `velopack` crate version** (both pinned `=1.2.0`) — bump them together (`dotnet tool update -g vpk` + the Cargo pin). See `docs/design/velopack-auto-update.md` for the full update flow and lineage.

# Developing + running Rift

One doc for everything between "I want to use this" and "I want to contribute." Sections:

1. [End-user install (Onboarding)](#1-end-user-install-onboarding)
2. [Building from source (Contributing)](#2-building-from-source-contributing)
3. [ChatGPT API access](#3-chatgpt-api-access)
4. [Claude Code w/ Rift on the Pro plan](#4-claude-code-w-rift-on-the-pro-plan)
5. [Releases](#5-releases)
6. [Configuration & environment variables](#6-configuration--environment-variables)

---

## 1. End-user install (Onboarding)

For someone handed a `Setup.exe` who wants to use Claude, ChatGPT, or both against a local folder.

### Install

- Run `Rift-win-Setup.exe`. Per-user install, no admin required. Lands at `%LOCALAPPDATA%\Rift\rift-tauri.exe` w/ a Start-menu shortcut.
- SmartScreen flag → **More info → Run anyway** (no code-signing cert yet).

### Connect a provider

At least one provider must be connected:

- **Claude:** install Claude Code (`npm install -g @anthropic-ai/claude-code`), run `claude` once, and complete browser login (Pro/Max/Team)—see §4.
- Or add an `ANTHROPIC_API_KEY` in **Settings → API key**; Rift passes it to the CLI per turn.
- **ChatGPT:** sign in through the Codex CLI for subscription-backed access, or add a key from the OpenAI API platform under **Settings → AI → ChatGPT API access**—see §3. Subscription and API billing are separate.
- Provider status appears in Settings and the model picker disables models whose provider is not ready.

### Pick a workspace + chat

- **Open a folder** in the workspace picker. Provider tools are scoped to that folder.
- Type in the composer and send. Both routes can use `read_file` / `list_dir` / `grep` and local git tools (`git_status` / `diff` / `log` / `pull` / `commit` / `push`). Network access occurs for the selected provider and any external tool action you approve.
- **GitHub (optional):** if the folder's `origin` is a GitHub repo and the [`gh` CLI](https://cli.github.com) is signed in, the branch chip shows CI/PR state and the assistant receives matching `gh_*` tools. Rift stores no GitHub token; calls use your existing `gh` login and stay pinned to `origin`.
- **Per-turn controls** sit on the composer: provider/model, permission mode, and reasoning effort. OpenAI model rows come from the account-visible model list with Rift defaults as a fallback.
- **Permission modes** — ask-before-edits, edit-automatically, plan, auto, or bypass. In the asking modes a gated tool surfaces an Allow/Deny bar before it runs.
- **Tabs / panes** — open multiple concurrent chats; each carries its own model + permission mode + effort.

### Updates

Rift self-updates via Velopack. It checks on launch + every ~6h; when a build is available you click **Update** once to consent, then it downloads in the background and applies on the next restart — unattended from there.

### Trouble?

- Claude unavailable → install/sign in to `claude` (§4) or add an Anthropic key in Settings.
- ChatGPT API unavailable → add a valid API key under Settings → AI (§3); a ChatGPT login alone does not authorize API requests.
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
| `src-tauri/src/` | Rust backend — `assistant/` (Claude CLI, OpenAI Responses API, shared tools + local git), `browser/`, `commands/`, `diagnostics/`, `state/`, `stt/`, plus `lib.rs` / `update_service.rs` / `secrets.rs` |
| `src-tauri/capabilities/` | Tauri 2 permission grants |
| `docs/` | Architecture, security model, release history |
| `scripts/` | Dev launcher + release pipeline + CDP helpers |

### Before opening a PR

- `npm run check` clean
- `cargo check --manifest-path src-tauri/Cargo.toml` clean (dev quit first)
- `cargo test` if you touched anything testable
- Or run `npm run verify` for the complete local gate. During live UI work, use `npm run verify:frontend`; `npm run verify` deliberately refuses to collide with an active Tauri dev process.
- `npm run doctor` reports the repo, toolchain, provider CLIs, Tauri dev, and CDP health without reading or printing credentials.
- Don't bump versions — the release pipeline handles `package.json` / `Cargo.toml` / `tauri.conf.json` in lockstep.

---

## 3. ChatGPT API access

Rift uses OpenAI's API directly; it does not automate the ChatGPT website and does not consume a ChatGPT Plus/Pro subscription.

1. Create an API key in the OpenAI platform and make sure the API account has billing/access configured.
2. Open **Settings → AI → ChatGPT API access**, paste the key, and save. Rift validates the key shape, stores it in Windows Credential Manager, and never returns it to the WebView.
3. Refresh models or open the composer model picker. Rift combines its supported GPT defaults with the GPT chat/reasoning models visible to that API account.
4. Select a GPT model for a conversation and send normally.

OpenAI turns use `/v1/responses` with streaming and `store: false`. Rift owns conversation persistence locally and sends the prior canonical Responses items with the next turn, including opaque encrypted reasoning, tool, and compaction state that cannot be recreated from display text. Image attachments, reasoning effort, cancellation, usage reporting, and permission-gated workspace tools use the same UI contract as Claude. Environment `OPENAI_API_KEY` values are detected only to explain setup; they are deliberately ignored until the user explicitly saves a key in Rift.

## 3.1 Codex / ChatGPT CLI connection

Settings → AI can inspect a standalone `codex` CLI and launch its official `codex login` browser flow. This uses the ChatGPT subscription authorized for Codex; it is separate from API-key billing. Rift never reads or copies Codex’s auth cache, and it rejects the packaged Windows Desktop helper because that executable is not a supported standalone CLI. The App Server turn adapter is intentionally not exposed as a model route until its streamed event and approval contracts have authenticated coverage.

For a real pre-release check, use the dev app: save a test key, refresh the model list, send one plain text turn, run one read-only workspace tool, then cancel a streaming turn. Never place a test key in source, logs, screenshots, or shell output.

---

## 4. Claude Code w/ Rift on the Pro plan

Optional power-user setup — Rift works fine with a plain `claude` login and zero config. This section is a Pro-plan-optimized tuning pass for Claude Code (the CLI Rift drives via MCP). ~10 min.

### Install + auth

```bash
npm install -g @anthropic-ai/claude-code
claude  # first run: opens browser for Pro auth
```

Verify: `claude --version` v2.1.111+ and `claude config` shows `model: claude-sonnet-5`. (The bare `sonnet` alias still resolves to Sonnet 4.6 on shipped CLIs — pin the explicit id.)

### `~/.claude/settings.json`

```json
{
  "$schema": "https://json.schemastore.org/claude-code-settings.json",
  "model": "claude-sonnet-5",
  "autoUpdatesChannel": "stable",
  "effortLevel": "medium",
  "env": {
    "CLAUDE_CODE_GIT_BASH_PATH": "<your Git bash.exe, e.g. C:\\Program Files\\Git\\bin\\bash.exe>",
    "CLAUDE_CODE_SUBAGENT_MODEL": "claude-haiku-4-5",
    "CLAUDE_CODE_AUTO_COMPACT_WINDOW": "250000",
    "CLAUDE_AUTOCOMPACT_PCT_OVERRIDE": "80"
  },
  "permissions": {
    "deny": ["Read(./.env)", "Bash(curl *)"]
  }
}
```

**Why each line:**
- Sonnet 5 default — handles 90%+ of coding at ~3× lower quota burn than Opus.
- Haiku for subagents — recon/grep agents fire at ~5% of Sonnet cost.
- `effortLevel: medium` — caps output ~2500 tok. Xhigh burns the 5-hour window ~3× faster.
- `CLAUDE_CODE_AUTO_COMPACT_WINDOW: "250000"` + `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE: "80"` — bound + trigger the CLI's own auto-compaction at 80% of a 250K window. Pro plan can't afford the 300K cliff. (This is the `claude` CLI's built-in compaction — Rift's in-app compaction UI was removed in the 2026-06-12 minimal-core strip.)
- `autoUpdatesChannel: stable` — avoids regression releases (v2.1.89+ caused 3-50x quota burn for some).
- Opus on-demand only — every Opus turn competes w/ your Sonnet budget. `/model claude-opus-4-8` per-session, then `/model claude-sonnet-5` back.

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

---

## 5. Releases

Maintainers only. Versions bumped manually across all three files (`package.json` + `Cargo.toml` + `tauri.conf.json`) BEFORE `scripts/release.ps1` runs — preflight bails on any mismatch (and on a dirty tree, which also catches an un-committed `Cargo.lock` after a version bump).

When dependencies change, regenerate the bundled license notices: `python scripts/gen-third-party-notices.py` rewrites `THIRD-PARTY-NOTICES.md` (shipped inside the installer next to the exe) — commit the result.

`release.ps1` drives `tauri build` → Velopack pack (`vpk`, delta baseline pulled from the R2 feed) → publish **feed-first**: Cloudflare R2 (the live update feed installed clients read) then the GitHub release on this repo (human download page, retried 3×; single-repo — the separate `rift` releases repo was retired when the source went public). **The `vpk` CLI version MUST equal the `velopack` crate version** (both pinned `=1.2.0`) — bump them together (`dotnet tool update -g vpk` + the Cargo pin).

### Ship flow + guard rails

The tag-driven `release.yml` now **runs the full test suite (`cargo test` + `svelte-check` + `vitest`) before it builds/publishes** — a tag can no longer ship code whose tests are red (the v0.31.0 failure mode). Two optional helpers around a ship:

- `pwsh scripts/smoke-turn.ps1 -Model haiku` — **before** tagging, prove a real Claude turn still completes end-to-end (spawns `claude` with Rift's exact turn flags against a throwaway folder; ~a cent of quota). Covers the live-turn check that CDP can't.
- `pwsh scripts/ship-watch.ps1` — **after** `git push --tags`, blocks on the release run and reports green/red (exit-status mirrors the run). Replaces the manual "confirm CI landed next session" step.

CI runs on a self-hosted runner. When a tagged release sits `queued` and never starts, or the **Verify published** step red-X's a release that actually shipped, the run usually just needs a cancel + rerun once the runner service is back.

---

## 6. Configuration & environment variables

**End users need zero environment variables.** Rift is self-contained: Anthropic and OpenAI API keys live in separate OS-keychain slots via Settings, never in config files or the WebView; app config is written to `~/.rift/`; downloaded speech models land in `~/.rift/models/`. Nothing reads a machine-specific path or port at runtime.

Every variable below is **optional** and scoped to development or release tooling:

| Variable | Scope | Purpose |
|---|---|---|
| `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` | dev | Set by `scripts/run-dev.bat` to expose CDP on `localhost:9222` for live-UI verification. Not used in prod. |
| `WEBVIEW2_USER_DATA_FOLDER` | dev | Isolates the dev WebView2 profile from an installed Rift so the two don't share cookies/state. |
| `RIFT_CDP_MAX_EDGE` | dev | Overrides the 2576px screenshot long-edge clamp in `scripts/cdp/serve.cjs`. Cosmetic. |
| `RIFT_MCP_SERVER` | internal | Set by Rift on itself when it re-spawns as the stdio MCP child for a turn. **Do not set manually.** |

CLI-side knobs (`CLAUDE_CODE_*`, `ANTHROPIC_API_KEY`) belong to the `claude` CLI, not Rift—see §4. Rift actively **strips** `ANTHROPIC_API_KEY` from the CLI environment on every turn so the in-app keychain key (or the CLI browser login) is the single source of Claude auth. The native OpenAI route likewise ignores `OPENAI_API_KEY` from the process environment; the explicit keychain value is its only credential source.

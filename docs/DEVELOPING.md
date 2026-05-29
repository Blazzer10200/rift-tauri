# Developing + running Rift

One doc for everything between "I want to use this" and "I want to contribute." Sections:

1. [End-user install (Onboarding)](#1-end-user-install-onboarding)
2. [Building from source (Contributing)](#2-building-from-source-contributing)
3. [Claude Code w/ Rift on the Pro plan](#3-claude-code-w-rift-on-the-pro-plan)
4. [FXServer authorized-keys ledger](#4-fxserver-authorized-keys-ledger)
5. [Releases](#5-releases)

---

## 1. End-user install (Onboarding)

For someone handed a `Setup.exe` who wants to start syncing.

### Install

- Run `Rift-Setup.exe`. Per-user install, no admin required. Lands at `%LOCALAPPDATA%\Rift\rift-tauri.exe` w/ a Start-menu shortcut.
- SmartScreen flag → **More info → Run anyway** (no code-signing cert yet).

### Generate / pick an SSH key

Rift needs an OpenSSH ed25519 keypair to talk to your FXServer.

- **Settings → SSH key**. If `~/.ssh/id_ed25519` exists, Rift uses it. Otherwise **Generate** writes to `~/.ssh/id_ed25519` + `.pub`.
- Send the **public** key to whoever runs the FXServer (see §4 for the ledger flow).

### Add a server

- **Sidebar → ＋ Add server**. Fill in: Name · Host / Port / User · Identity file (pre-filled) · Remote root (path to your `txData/<base>/resources`) · Local root.
- **Save**. First connect prompts for a fingerprint (TOFU) — confirm and pin.

### First sync

- **Sync** tab (Ctrl+2). Drift auto-populates as soon as connection is ready.
- **Sync** button does pull-then-push in one shot. Granular `Pull only` / `Push only` live under the `⋯` kebab.
- Conflicts surface as a badge on the **Sync** tab — click through to the conflict list and resolve per-file.
- Auto-rescan toggle (kebab → off / 30s / 1m / 2m / 5m / 10m) catches teammate pushes the watcher can't see.

### Updates

Rift checks for updates on launch. New build → dialog auto-pops → **Download** opens the Setup.exe in your browser — run it to upgrade.

### Trouble?

- `~/.rift/` = config + log dir. `rift.json` = profiles, `rift-autosync.log` = sync activity.
- Sync stuck → **Stop → Start** in AutoSync.

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

First `cargo` build is ~5 min cold (pure-Rust russh + Tauri). Incremental: seconds.

### Run dev

```bash
scripts/run-dev.bat        # red-tinted icon, separate from installed Rift
# or
npm run tauri dev
```

Dev watches `src/` + `src-tauri/src/` and hot-reloads. **Don't run `cargo check` while dev is alive** — it kills the running dev process via incremental-rebuild collision.

### Project layout

| Where | What |
|---|---|
| `src/` | SvelteKit frontend (Svelte 5 runes, Tailwind 4) |
| `src-tauri/src/` | Rust backend — Tauri commands + russh + auto-sync engine |
| `src-tauri/capabilities/` | Tauri 2 permission grants |
| `docs/` | Live state — read `HANDOFF.md` first each session |
| `scripts/` | Dev launcher + release pipeline |

### Before opening a PR

- `npm run check` clean
- `cargo check --manifest-path src-tauri/Cargo.toml` clean
- `cargo test` if you touched anything testable
- Don't bump versions — release pipeline handles `package.json` / `Cargo.toml` / `tauri.conf.json` in lockstep.

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

### `~/.claude/CLAUDE.md` — minimal

```md
# Global Instructions

**Shell:** Windows 11 + Git Bash. Forward-slash paths in bash; Windows paths fine in Read/Edit.
**Model:** Sonnet 4.6 default. Opus 4.7 only for multi-file architectural reasoning.

## Rules
- Never delete files without explicit instruction.
- Search before creating — grep / glob first.
- Fix what's asked, no adjacent refactors.
- Fail loud — no silent fallbacks.
- Verify before claiming done.
```

### Usage tracking

- In-CLI: `/status` shows remaining allocation for the current 5-hour window.
- Web: `https://claude.ai/settings` → Usage (browser chat + CLI share the same pool).
- Limit resets on rolling 5-hour windows, NOT weekly.

### Rift + Claude integration

Rift's Assistant tab shells `claude` with `--mcp-config <rift.mcp.json>` + `--allowed-tools mcp__rift__*`. Claude inherits `$HOME` so Pro login works automatically.

**Session resume:** v0.4.1+ auto-recovers from "No conversation found with session ID" — you'll see "Session was lost — retrying as a fresh start," that's expected.

### Common pitfalls

- Shared quota: claude.ai browser eats the same pool as the CLI.
- Silent fallback: over-limit may downgrade silently — check `/status` if output quality drops.
- `/compact` frequently — prompt caching has been buggy in v2.1.100+; compact at every major milestone.
- Don't `EnterPlanMode` — use the `/plan` skill if you have it; otherwise describe + execute.

---

## 4. FXServer authorized-keys ledger

Pubkey ledger for the shared `blazzer` Linux user on the FXServer (CT 120, `192.168.1.170:22`). Mirrors `/home/blazzer/.ssh/authorized_keys`. **Pubkeys only — never commit private halves.**

| Owner | Comment | Added | Public key |
|---|---|---|---|
| Blazzer | `blazzer@DESKTOP-GIT053H -> homelab` | 2026-04 (initial) | `ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAFahpilpOZr0krma/ag1MQJaEccbmfLyzX1CWJQyoeW` |
| Trey | `rift-TREYDAY@DESKTOP-N2AMAU5` | 2026-05-09 | `ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIKHaeCZR4xwBbLULdihAdkh5HrlYU89uoD2CMuZAm/Oc` |

### Adding a new dev

1. Dev runs `ssh-keygen -t ed25519 -C "rift-<handle>@$HOSTNAME"` (or via Rift's in-app keygen).
2. Dev sends the **public** key to Blazzer.
3. Blazzer appends on CT 120:
   ```bash
   ssh blazzer-labs "pct exec 120 -- bash -c '
     echo \"<pubkey>\" >> /home/blazzer/.ssh/authorized_keys &&
     chmod 600 /home/blazzer/.ssh/authorized_keys &&
     chown blazzer:blazzer /home/blazzer/.ssh/authorized_keys
   '"
   ```
4. Update this table.

### Revoking access

```bash
ssh blazzer-labs "pct exec 120 -- sed -i '/<comment-substring>/d' /home/blazzer/.ssh/authorized_keys"
```

Then update this table.

---

## 5. Releases

Maintainers only. Versions bumped manually (or via `/git-ship`) across all three files (`package.json` + `Cargo.toml` + `tauri.conf.json`) BEFORE `scripts/release.ps1` runs — preflight bails on any mismatch. `release.ps1` then drives `tauri build` → `gh release create` (NSIS Setup.exe only) → SHA256 round-trip verify against the public `rift-releases` repo.

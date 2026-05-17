# Trey — Setup Guide

Best-of-best setup for using Rift v0.4.1-alpha with Claude Code on the $20/mo Anthropic Pro plan. Read top to bottom, run the commands. ~10 min.

---

## 1. Update Rift to v0.4.1-alpha

Rift ships its own auto-updater (Velopack). Once v0.4.1-alpha publishes to `rift-releases` (Blazzer is shipping it now), launch Rift and accept the update prompt. The installer is `Setup.exe` — perUser NSIS, no admin needed.

After update lands, **don't turn Mirror back on yet** — see step 2.

## 2. Mirror gate (DO THIS FIRST)

Multi-user safety: Mirror stays **OFF** on your install until you've:

1. Updated to v0.4.1-alpha (step 1)
2. Done a fresh **Pull** from the remote to establish a clean baseline
3. Verified the drift table is empty (Sync panel)

Then re-enable Mirror in Settings → Sync. Skipping this risks the mass-delete circuit breaker firing on your first sync because your local snapshot doesn't match the remote.

## 3. Claude Code — install + login

```bash
npm install -g @anthropic-ai/claude-code
claude  # first run: opens browser for Pro auth
```

Verify: `claude --version` prints v2.1.111+ and `claude config` shows `model: claude-sonnet-4-6`.

## 4. ~/.claude/settings.json — Pro-optimized

Drop this in `~/.claude/settings.json`:

```json
{
  "$schema": "https://json.schemastore.org/claude-code-settings.json",
  "model": "claude-sonnet-4-6",
  "autoUpdatesChannel": "stable",
  "env": {
    "CLAUDE_CODE_GIT_BASH_PATH": "C:\\Program Files\\Git\\bin\\bash.exe",
    "CLAUDE_CODE_SUBAGENT_MODEL": "claude-haiku-4-5"
  },
  "permissions": {
    "deny": ["Read(./.env)", "Bash(curl *)"]
  }
}
```

Key points:
- **Sonnet 4.6 default** — handles 90%+ of coding; far lower quota burn than Opus.
- **Haiku for subagents** — recon/grep agents fire at ~5% the cost of Sonnet.
- **`autoUpdatesChannel: stable`** — avoids regression releases (v2.1.89+ caused 3-50x quota burn for some users).
- **Opus on-demand only** — Pro has no separate Opus pool; every Opus turn competes with your Sonnet budget. Use `/model claude-opus-4-7` per-session when you need it for hard architectural work, then `/model claude-sonnet-4-6` back.

## 5. ~/.claude/CLAUDE.md — minimal

Create `~/.claude/CLAUDE.md` (≤80 lines — it injects every session):

```md
# Trey — Global Instructions

**Shell:** Windows 11 + Git Bash. Forward-slash paths in bash, Windows paths fine in Read/Edit tools.

**Model:** Sonnet 4.6 default. Escalate to Opus 4.7 only for multi-file architectural reasoning.

**Active projects:**
- Rift workspace at `C:/path/to/your/rift-workspace/`

## Rules
- Never delete files without explicit instruction.
- Search before creating — `grep` / `glob` first.
- Fix what's asked, no adjacent refactors.
- Fail loud — no silent fallbacks.
- Verify before claiming done.
```

## 6. Usage tracking

- In-CLI: `/status` shows remaining allocation for the current 5-hour window.
- Web: `https://claude.ai/settings` → Usage (browser chat + CLI share the same Pro pool).
- Limit resets on rolling 5-hour windows, NOT weekly.

## 7. Rift + Claude Code integration

Rift's Assistant tab shells out to `claude` with `--mcp-config <rift.mcp.json>` + `--allowed-tools mcp__rift__*`. This gives Claude Code project-aware tools (file ops, sync queries) inside Rift.

**Auth:** Claude Code reads `~/.claude/.credentials.json`. Rift inherits `$HOME` so your Pro login works automatically inside the Assistant tab.

**Session resume:** v0.4.1-alpha auto-recovers from "No conversation found with session ID" (claude's `--resume` index sometimes drifts after long-idle tabs). You'll see "Session was lost — retrying as a fresh start" — that's expected, the prompt re-sends as a fresh first-turn automatically.

## 8. Common pitfalls

- **Shared quota:** browser claude.ai sessions eat from the same Pro pool as Claude Code. Heavy chat + CLI day = hitting limits.
- **Silent fallback:** when over limit, Claude Code may downgrade silently. If output quality drops mid-session, check `/status`.
- **`/compact` frequently** — prompt-caching has been buggy in v2.1.100+. Compacting every major milestone keeps token consumption sane.
- **Don't `EnterPlanMode`** — use the `/plan` skill if you have it; otherwise just describe + execute.

---

Questions: ping Blazzer. Welcome aboard.

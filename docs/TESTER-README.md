# Rift — First-run guide for testers

Rift is a local Windows coding assistant with Claude and ChatGPT. Connect either
provider or both; you do not need both CLIs.

## Before installing

Choose at least one route:

```powershell
# ChatGPT subscription
npm install -g @openai/codex

# Claude
npm install -g @anthropic-ai/claude-code
```

Both require Node.js. Optional OpenAI or Anthropic API access can instead be
configured inside Rift. ChatGPT subscriptions and OpenAI API billing are
separate.

After installing a CLI, restart Rift. If Windows still exposes an older PATH,
sign out or reboot once.

## Install Rift

Run `Rift-win-Setup.exe`. It installs per user without administrator access.

The public beta is not code-signed, so SmartScreen may show **Windows protected
your PC**. Choose **More info → Run anyway**. If the browser warns about the
download, choose **Keep** only when the installer came from Rift's official
GitHub release.

## Connect a provider

Open **Settings → Providers**:

- **ChatGPT:** use the subscription sign-in/check flow. Rift uses the models and
  limits reported by the signed-in Codex App Server.
- **Claude:** complete the Claude CLI login or save an optional Anthropic key.
- **Optional ChatGPT API access:** save an OpenAI API key only when separately
  billed API usage is intended.

The compact model picker separates ChatGPT and Claude, shows primary models
first, and keeps compatibility or unavailable rows under **Other models**.
Existing conversations remain pinned to their original ChatGPT subscription or
API billing route.

## Send useful feedback

For a bug, include:

- What you did and what happened.
- Rift version from **Settings → System**.
- Active provider and model.
- Relevant `claude --version` or `codex --version` output.
- A screenshot for visual problems.

To copy diagnostics:

1. Open **Settings → System**.
2. Choose **Open diagnostics console**.
3. Use **Copy** or export the filtered events.

Diagnostics scrub the Windows username and credential-shaped values. Never
paste API keys, login files, or secrets into a bug report.

## Quick troubleshooting

| Symptom | Action |
|---|---|
| CLI not found | Restart Rift; sign out/reboot if PATH is still stale |
| ChatGPT unavailable | Run `codex login status`, then **Check again** in Providers |
| Claude unavailable | Run `claude` and complete login, then re-check Providers |
| SmartScreen blocks setup | **More info → Run anyway** for the official installer |
| App will not open | Repair/install Microsoft WebView2 Runtime |
| Message opens a command window | Confirm Rift is updated to `v0.151.1` or newer |

# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.97.0 — Split panes hardened + AI Health dashboard + /mcp dialog

- **Split panes hardened** — two panes can no longer end up showing the same chat: that duplicate crashed the entire chat surface AND saved itself to disk, so it came back on every restart (blank chat no reboot could fix). Every pane path now guards it, and old broken saves self-heal on load. The composer bar also sheds decoration progressively in narrow panes (effort text → labels → mic/attach) instead of painting controls over each other.
- **AI Health tells the truth about "right now"** — the "API is slow" strip now judges only the last 24 hours of replies, so one slow afternoon no longer alarms for weeks (a lifetime number shows a quiet "lifetime" tag instead). New **MCP servers card** shows per-server status from the latest turn, with honest copy for the always-signed-out claude.ai connectors. Workspace "spent" relabeled "est. spend".
- **AI Health redesigned into a real dashboard** — two-column card grid on wide windows; the health verdict always shows all three labeled dimension pills (Latency · Cache · Plan) instead of anonymous dots; a **time-range picker** (24h / 7d / 30d / All) re-scopes every speed and spend number; **spend per day** is a proper bar chart; **spend by model** shows where the dollars actually go (≠ message share); **per-model speed rows** surface the breakdown the advisor always had ("Opus · medium — typical 2.5s · slow 10.4s · mostly thinking"); and plan-limit bars gain a **pace forecast** ("on pace for ~50% by reset", warning when you'd hit the cap early).
- **"Worked for Ns" is now real wall-clock** — the turn header and Done footer previously summed only thinking + tool seconds, so a 30-second text reply read "Worked for 0s". Now they report spawn-to-result time from the CLI.
- **Quieter startup** — the "MCP server not connected" toast no longer fires for user-level claude.ai connectors (permanently unauthenticated inside Rift's headless CLI — that's their normal state). Only a dead `rift` workspace server still warns.
- **New `/mcp` command** — opens a centered dialog showing every MCP server in your Claude setup, not just this chat's: it runs the CLI's own `claude mcp list` (user scope + project `.mcp.json`, live health check) and overlays the current chat's per-turn statuses, so it answers before the first message ever sends — for any user's config. Each server gets a status dot, its URL/command, and an honest state (Connected · Needs sign-in · Needs approval · Failed) with a "this chat" badge on live session statuses, a contextual fix-it hint in the footer, and a Re-check button. Esc or click-outside closes; no turn, no cost, no notification minted.
- **No more stub reveals** — terminal output never hides a short tail behind a click: a remainder of ≤8 lines just renders (killed "Show 4 more lines"), a no-op Collapse no longer appears on short output, and the collapsed peek shows a lone extra line instead of "+1 more line".

## v0.96.0 — Settings overhaul + chat display polish

Settings restructured into 5 searchable tabs with live previews and one-click Looks; About tab + update dialog rebuilt (non-blocking downloads, "Updated to vX" welcome-back); your Claude Code skills surfaced in the `/` menu; chat tool display sharpened (per-domain favicons, real icons for every tool, unified rhythm); density/Lexend/texture fixes; dead code swept and compiler-enforced.

## v0.95.1 — Pre-open-source audit

13-reviewer adversarial sweep ahead of going public — security surfaces (path containment, XSS sanitization, capability grants, TLS, token auth) all held. Fixed: helper-process handle leaks on kill timeouts · process kills moved off the async runtime · package metadata → MIT · dev-machine paths and infra names scrubbed from scripts, fixtures, CI.


## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`) can't be recovered from Azure's servers — real fix is the on-device **Whisper** engine (built, not yet shipped).
- **Dismissed ask-cards** render "(no answer recorded)" instead of a "Dismissed" state — cosmetic, fix planned.

## Earlier

History via `git log -- docs/CHANGELOG.md`.

# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## Unreleased

- **Quieter startup** — the "MCP server not connected" toast no longer fires for user-level claude.ai connectors (permanently unauthenticated inside Rift's headless CLI — that's their normal state). Only a dead `rift` workspace server still warns.
- **New `/mcp` command** — instant per-session MCP server status (name + connection state) straight from the CLI's init report; no turn, no cost. In the `/` menu under Info.
- **No more stub reveals** — terminal output never hides a short tail behind a click: a remainder of ≤8 lines just renders (killed "Show 4 more lines"), a no-op Collapse no longer appears on short output, and the collapsed peek shows a lone extra line instead of "+1 more line".

## v0.96.0 — Settings overhaul + chat display polish

- **Chat tool display sharpened** — web-search sources get per-domain favicons; test/lint icons take the pass/fail color; subagent cards get an accent-wash head; read/grep rows a subtle tint; a running shell prompt glows and active rows scan-sweep. Vertical rhythm unified — every block sits on one consistent, density-aware gap. Every Claude Code tool shows a real icon (eight ex-generic-wrench tools got their own), and any future CLI tool still renders as a clean named chip rather than vanishing. **Fixed:** a narration sentence split across a tool call ("…in p" · tool · "arallel.") no longer renders a word sliced in half.
- **Your Claude Code skills in the `/` menu** — user + project skills/commands scanned from `.claude` dirs with descriptions and argument hints; fuzzy filter with highlight, source badges, Tab-inserts-args vs Enter-runs. Menu chrome redesigned (glass panel, sticky groups).
- **Settings restructured into 5 tabs** with a search box (press `/` or Ctrl+F) that jumps to the matching card; redundant cards merged. **Show-don't-tell everywhere:** live code + density preview, accent sample strip, mini activity-stream preview, classic-bubbles preview when stream view is off, real-pipeline reply preview, a live context gauge under the plan picker, collapsed option grids with animated expanders, and slider drag-bubbles.
- **One-click Looks** — five curated accent + texture + vividness combos atop Appearance; every dial stays tunable after.
- **Per-tab "Reset to defaults"** on Appearance, Chat, Claude, and Speech — with a confirm step; API key, spending cap, and Whisper models are never touched.
- **Claude tab reworked:** CLI version chip, billing route strip, contextual Pro note, warn-only tab dot.
- **About tab redone:** version and stack collapse to a chip strip with an inline "Check for updates"; a "Source code & license" row links the GitHub repo; installed local tools shrink to hover-labelled chips; new "Report a bug" copies your diagnostic and opens a GitHub issue.
- **Update dialog rebuilt** — the version jump (v0.95 → v0.96) is the hero, with download size, "checked N min ago", and one-tap release notes. Human copy everywhere: a calm "dev build" panel, an offline message with the raw error in a collapsible, and "your chats and settings are kept" on a broken install with one-click "Get Setup.exe". Downloads are non-blocking — hide the dialog and the top bar tracks progress.
- **"Updated to vX" welcome-back** — after an update installs and Rift relaunches, a one-time toast confirms the new version with a "What's new" shortcut; the up-to-date panel notes what you moved up from.
- **Fixed:** interface density (compact/regular/comfy) never applied — a theme selector outranked it; dyslexia Lexend font not applied app-wide; sidebar scope-toggle thumb dislocating; texture picker tiles too faint to tell apart; unstyled "Install" button in Local tools; a banner-started download briefly hid its own progress bar.
- **Leaner, tighter internals** — dead code swept from every layer and now compiler-enforced; dialog permissions narrowed to exactly what's used; dependency audit clean.

## v0.95.1 — Pre-open-source audit

13-reviewer adversarial sweep ahead of going public — security surfaces (path containment, XSS sanitization, capability grants, TLS, token auth) all held. Fixed: helper-process handle leaks on kill timeouts · process kills moved off the async runtime · package metadata → MIT · dev-machine paths and infra names scrubbed from scripts, fixtures, CI.


## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`) can't be recovered from Azure's servers — real fix is the on-device **Whisper** engine (built, not yet shipped).
- **Dismissed ask-cards** render "(no answer recorded)" instead of a "Dismissed" state — cosmetic, fix planned.

## Earlier

History via `git log -- docs/CHANGELOG.md`.

# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.117.0 — GitHub, woven in: the branch chip comes alive

- **The branch chip now carries a CI status dot.** If the open folder's `origin` is a GitHub repo and the `gh` CLI is signed in, the chip (welcome card, composer, status bar) shows a live dot: gray idle, amber breathing while a run is in flight, green on success, red on failure. Non-GitHub folders look exactly like before.
- **Click the chip for a GitHub popover** — ahead/behind sync with origin, the latest workflow run (with a link out), and the branch's open PR with review state. Two one-click actions hand the work to Claude: **"Ask Claude to fix the failing run"** (pulls the failed job logs and gets to work) and **"Draft a pull request"** (reviews the branch diff, then asks before creating).
- **Claude gets matching GitHub tools**: `gh_checks`, `gh_run_view` (with failed-log tails), `gh_pr_list` / `gh_pr_view` / `gh_pr_diff`, and `gh_pr_create` — the one write tool, gated to Standard trust just like `git_push`.
- **Tokenless by design.** Everything rides your own `gh` login — Rift never sees or stores a GitHub credential — and every call is pinned to the workspace's `origin` repo through a whitelist parser, so neither the model nor the UI can point the tools somewhere else. No `gh` installed / not signed in? The popover says so and links the fix; nothing else changes.
- Status refreshes lazily: on chip appearance, on window focus (60s minimum gap), and immediately after a push or PR creation.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.

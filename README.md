# Rift

A local-workspace coding assistant for Windows. Rift wraps the Claude CLI in a native Tauri shell: pick a workspace folder, chat with Claude, and let it read, search, edit, and run git against that folder — all on your machine, no remote connections.

Tauri 2 (Rust backend) + SvelteKit 2 / Svelte 5 (runes) + Tailwind 4. The backend spawns the Claude CLI as a subprocess and hosts a local stdio MCP server exposing workspace-scoped `read_file` / `list_dir` / `grep` plus local-git tools. Velopack installer (per-user, installs as `rift-tauri.exe`, no admin) for first install; thereafter self-updates via Velopack (background download + apply-on-restart, no mandatory signing key).

## Status

Pre-1.0 (`-alpha`). Pure-assistant — the former SFTP/sync/server/RCON half was fully removed (2026-06-03). See [`docs/HANDOFF.md`](docs/HANDOFF.md) for the current in-flight state.

## Quick links

- **Download (end users):** latest `Rift-win-Setup.exe` on the [releases page](https://github.com/Blazzer10200/rift/releases). Per-user install, no admin — see [`docs/DEVELOPING.md` §1](docs/DEVELOPING.md#1-end-user-install-onboarding).
- **How it fits together:** [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- **Install, build, contribute:** [`docs/DEVELOPING.md`](docs/DEVELOPING.md)
- **Release history:** [`docs/CHANGELOG.md`](docs/CHANGELOG.md)
- **Issue tracker:** [`docs/ISSUES.md`](docs/ISSUES.md)
- **Accepted security advisories:** [`docs/SECURITY.md`](docs/SECURITY.md)

## What it does

- **Workspace chat** — point Rift at a folder; Claude works scoped to it via a local MCP server (`read_file` / `list_dir` / `grep`).
- **Local git** — `git_status` / `diff` / `log` / `pull` / `commit` / `push` exposed as assistant tools.
- **Per-tab sessions** — multiple concurrent chats/panes, each with its own model, permission mode, and thinking-effort.
- **Permission modes** — ask-before-edits, edit-automatically, plan, auto, or bypass — switchable per turn.
- **Voice dictation** — push-to-talk speech-to-text (Web Speech or local Whisper) plus a prompt-enhance wand.
- **Browser dock** — Claude can open docs or a local dev server in an in-app browser pane beside the chat.
- **Self-update** — Velopack checks on launch + every 6h, downloads in the background, and applies on restart (one click to consent, then unattended).

## Platforms

Windows 11 (primary). macOS / Linux are buildable from source but not packaged or tested.

## License

**Proprietary — © 2026 Blazzer10200. All rights reserved.** This source is private and not licensed for use, modification, or redistribution. The distributed binaries (via [`rift`](https://github.com/Blazzer10200/rift)) are provided for use as-is; the source carries no open-source grant.

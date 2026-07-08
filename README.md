<p align="center">
  <img src="design-system/assets/rift-logo.png" width="96" alt="Rift logo" />
</p>

<h1 align="center">Rift</h1>

<p align="center">
  Claude Code as a native Windows desktop app.<br/>
  Point it at a folder and chat — Claude reads, edits, searches, and runs git in place.<br/>
  <strong>No server, no telemetry — everything runs on your machine.</strong>
</p>

<p align="center">
  <a href="https://github.com/Blazzer10200/rift/releases"><img src="https://img.shields.io/github/v/release/Blazzer10200/rift?label=download&color=10b981" alt="Latest release" /></a>
  <a href="https://claude.com/claude-code"><img src="https://img.shields.io/badge/powered%20by-Claude%20Code-d97757" alt="Powered by Claude Code" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT license" /></a>
  <img src="https://img.shields.io/badge/platform-Windows%2011-0078d4" alt="Windows 11" />
  <img src="https://img.shields.io/badge/Tauri%202-Rust%20%2B%20Svelte%205-f74c00" alt="Tauri 2" />
</p>

<p align="center">
  <!-- GitHub READMEs do NOT play <video> tags whose src is an in-repo path
       (verified 2026-07-08: only github.com/user-attachments/assets/... URLs
       render as players) — the GIF is the reliable always-animating hero.
       Full-quality upgrade path once the final video exists: drag rift-demo.mp4
       into any GitHub issue/PR comment box, copy the generated
       user-attachments URL, and paste it on its own line below this block. -->
  <img src="docs/media/rift-demo.gif" width="820" alt="Rift demo: ask a question about your codebase and watch Claude read files and stream a structured answer, live." />
</p>

<p align="center"><sub>Real capture — a live turn on Rift's own source code. Ask, watch it read the files, and stream a structured answer.</sub></p>

## Install

1. Download the latest `Rift-win-Setup.exe` from the **[releases page](https://github.com/Blazzer10200/rift/releases)** and run it.
2. Per-user install, no admin needed. Rift self-updates from then on (background download, apply on restart, one-click consent).
3. On first launch, connect your Claude account (the CLI's own sign-in, or an API key stored in the Windows Credential Manager — never on disk).

**Requirements:** Windows 11 x64 · the [Claude Code CLI](https://claude.com/claude-code) (Rift checks for it on first launch and guides setup) · a [Claude](https://claude.com) subscription or API key. macOS / Linux build from source but aren't packaged or tested yet.

## What it does

- **Workspace chat** — pick a folder; Claude works scoped to it through a local MCP (Model Context Protocol) server (`read_file` / `list_dir` / `grep`). Nothing outside the workspace is reachable.
- **Local git built in** — `status` / `diff` / `log` / `commit` / `push` / `pull` exposed as assistant tools.
- **Live streaming UI** — watch tool calls form in real time: shell commands render as terminal IN/OUT blocks, edits as diffs, plans as task cards, sub-agents as live inline cards with a floating overview bar.
- **Per-tab sessions** — multiple concurrent chats, each with its own model, permission mode, and thinking-effort dial. Mid-chat model switching included.
- **Permission modes** — ask-before-edits, auto-edit, plan-first, or bypass — switchable per turn.
- **Voice dictation** — push-to-talk speech-to-text plus a prompt-enhance wand.
- **Browser dock** — the assistant can open docs or your local dev server in an in-app pane beside the chat.
- **Local LLM (experimental)** — point Rift at an Ollama / LiteLLM endpoint instead of the cloud.
- **Self-update** — Velopack checks on launch and every 6 hours; updates apply on restart.

## How it works

Tauri 2 (Rust) shell around a SvelteKit 2 / Svelte 5 frontend. The backend spawns the Claude Code CLI as a per-turn subprocess (a warm child stays ready so turns start fast) and hosts a local stdio MCP server that exposes the workspace-scoped file, search, and git tools plus a small UI bridge (`ask_user` / `open_browser` / `notify`) over a loopback socket. There is no remote component: your code, your prompts, and your keys never leave the machine except for the model API call itself. Auth belongs to the CLI — Rift spawns the official `claude` binary, which handles its own sign-in; Rift never reads, stores, or proxies your Claude credentials.

Full picture in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## FAQ

**Do I need Claude Code installed?**
Yes — Rift drives the official CLI rather than reimplementing it. First launch checks for it and guides setup; sign-in and credentials stay inside the CLI.

**Does my Claude Pro / Max subscription work?**
Yes. Sign in to the CLI as usual — Rift turns are ordinary Claude Code usage on your plan. Your own API key works too.

**Where does my code go?**
Nowhere except the model call the official CLI makes. No server, no telemetry, no analytics — Rift is a local shell around a local process. Details in [`docs/SECURITY.md`](docs/SECURITY.md).

**Windows only?**
Packaged and tested for Windows 11 today. macOS / Linux build from source; packaging them is on the roadmap.

## Building from source

```
npm install
npm run tauri dev     # dev shell with hot reload
npm run tauri build   # release bundle
```

Prereqs, the release pipeline, and the contributor map live in [`docs/DEVELOPING.md`](docs/DEVELOPING.md).

## Project docs

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — how the pieces fit together
- [`docs/DEVELOPING.md`](docs/DEVELOPING.md) — install, build, contribute
- [`docs/CHANGELOG.md`](docs/CHANGELOG.md) — release history
- [`docs/SECURITY.md`](docs/SECURITY.md) — security model + accepted advisories
- [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) — how to file issues and open PRs

## Status

Pre-1.0, moving fast — releases ship continuously via tag-driven CI to the [`rift`](https://github.com/Blazzer10200/rift) feed repo. Rift is built with Rift and is its maintainer's daily driver, but expect fast iteration and the occasional rough edge. See the [changelog](docs/CHANGELOG.md) for what's landing.

## License

[MIT](LICENSE) © 2026 Braison Swilling (Blazzer10200)

<sub>Rift is an independent open-source project — not affiliated with, sponsored, or endorsed by Anthropic. Claude and Claude Code are trademarks of Anthropic, PBC.</sub>

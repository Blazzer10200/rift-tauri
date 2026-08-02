<p align="center">
  <img src="design-system/assets/rift-logo.png" width="96" alt="Rift logo" />
</p>

<h1 align="center">Rift</h1>

<p align="center">
  Claude Code and ChatGPT models in one native Windows workspace.<br/>
  Point Rift at a folder and chat — your chosen model can read, edit, search, and run git in place.<br/>
  <strong>No server, no telemetry — everything runs on your machine.</strong>
</p>

<p align="center">
  <a href="https://github.com/Blazzer10200/rift-tauri/releases"><img src="https://img.shields.io/github/v/release/Blazzer10200/rift-tauri?label=download&color=10b981" alt="Latest release" /></a>
  <img src="https://img.shields.io/badge/providers-Claude%20%2B%20OpenAI-8b5cf6" alt="Claude and OpenAI providers" />
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT license" /></a>
  <img src="https://img.shields.io/badge/platform-Windows%2011-0078d4" alt="Windows 11" />
  <img src="https://img.shields.io/badge/Tauri%202-Rust%20%2B%20Svelte%205-f74c00" alt="Tauri 2" />
</p>

<p align="center">
  <!-- GitHub READMEs do NOT play <video> tags whose src is an in-repo path
       (verified 2026-07-08: only github.com/user-attachments/assets/... URLs
       render as players) — the GIF is the reliable always-animating hero.
       Both tour files are hosted as v0.143.0 release assets (moved out of the
       tree 2026-07-23: 18MB of binaries don't belong in every clone).
       Full-quality upgrade path: drag rift-tour.mp4 (1080p60) into any GitHub
       issue/PR comment box, copy the generated user-attachments URL, and
       paste it on its own line below this block. -->
  <img src="https://github.com/Blazzer10200/rift-tauri/releases/download/v0.143.0/rift-tour.gif" width="820" alt="Rift product tour: open a workspace, chat with Claude about the code, split panes, check AI Health, and re-theme the app live from Settings." />
</p>

<p align="center"><sub>The 45-second tour — workspace to chat to split panes to Settings, with the whole app re-tinting live. Full-quality video: <a href="https://github.com/Blazzer10200/rift-tauri/releases/download/v0.143.0/rift-tour.mp4">rift-tour.mp4</a>.</sub></p>

## Install

1. Download the latest `Rift-win-Setup.exe` from the **[releases page](https://github.com/Blazzer10200/rift-tauri/releases)** and run it.
2. Per-user install, no admin needed. Rift self-updates from then on (background download, apply on restart, one-click consent).
3. On first launch, connect Claude and/or OpenAI. Keys entered in Rift are stored in Windows Credential Manager—never in app files or the WebView. The Providers page can also inspect and sign in to a local Codex CLI without importing its credentials.

**Requirements:** Windows 11 x64 and at least one provider: the [Claude Code CLI](https://claude.com/claude-code) with a Claude login/API key, or ChatGPT through the standalone Codex CLI and optional [OpenAI API key](https://platform.openai.com/api-keys). ChatGPT subscriptions and API usage are billed separately. macOS/Linux build from source but are not packaged or tested yet.

## What it does

- **Workspace chat** — pick a folder and choose Claude or GPT per conversation. File/search/git tools remain scoped to that folder.
- **Local git built in** — `status` / `diff` / `log` / `commit` / `push` / `pull` exposed as assistant tools.
- **Live streaming UI** — watch tool calls form in real time: shell commands render as terminal IN/OUT blocks, edits as diffs, plans as task cards, sub-agents as live inline cards with a floating overview bar.
- **Per-tab sessions** — multiple concurrent chats, each with its own model, permission mode, and thinking-effort dial. Mid-chat model switching included.
- **Permission modes** — ask-before-edits, auto-edit, plan-first, or bypass — switchable per turn.
- **Voice dictation** — push-to-talk speech-to-text plus a prompt-enhance wand.
- **Browser dock** — the assistant can open docs or your local dev server in an in-app pane beside the chat.
- **Native OpenAI support** — Responses API streaming, reasoning effort, image input, account-visible model discovery, permission-gated function calls, cancellation, and locally persisted conversation history.
- **Provider-aware workspace** — at-a-glance Claude, OpenAI, and Codex connection status; Workspace news names its exact official source, and optional AI summaries stay gated behind their required connection.
- **Compatible endpoints (experimental)** — connect supported Anthropic-compatible or local endpoints from Settings.
- **Self-update** — Velopack checks on launch and every 6 hours; updates apply on restart.

## How it works

Tauri 2 (Rust) shell around a SvelteKit 2 / Svelte 5 frontend. Claude turns run through the official Claude Code CLI with a warm child process. OpenAI turns use the native Responses API with `store: false`; Rift sends the locally saved canonical Responses items on each turn. Both routes share workspace-scoped file, search, git, permission, and UI tools. The Codex connection uses only the official standalone CLI’s public status/login commands; its App Server turn route is not exposed until authenticated contract coverage exists. There is no Rift server or telemetry service: data leaves the machine only for the provider call or a tool action you authorize.

Full picture in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## FAQ

**Do I need Claude Code installed?**
Only for Claude models. ChatGPT API models use Rift's native route and require an API key instead.

**Does my Claude Pro / Max subscription work?**
Yes. Sign in to the CLI as usual — Rift turns are ordinary Claude Code usage on your plan. Your own API key works too.

**Does my ChatGPT Plus / Pro subscription work?**
ChatGPT and the OpenAI API are billed separately. Rift's GPT integration needs a key from the OpenAI API platform and uses the models available to that API account. Settings can sign a standalone Codex CLI into ChatGPT without copying its credentials; Codex App Server turns remain a tracked, authenticated-test-gated route.

**Where does my code go?**
Only to the provider selected for that turn and to any external tool action you approve. Rift has no intermediary server, telemetry, or analytics. Details in [`docs/SECURITY.md`](docs/SECURITY.md).

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

Pre-1.0, moving fast — releases ship continuously via tag-driven CI straight to this repo's [releases page](https://github.com/Blazzer10200/rift-tauri/releases). Rift is built with Rift and is its maintainer's daily driver, but expect fast iteration and the occasional rough edge. See the [changelog](docs/CHANGELOG.md) for what's landing.

## License

[MIT](LICENSE) © 2026 Braison Swilling (Blazzer10200)

Rift is an independent open-source project—not affiliated with, sponsored by, or endorsed by Anthropic or OpenAI. Claude and Claude Code are trademarks of Anthropic, PBC. ChatGPT, GPT, and OpenAI are trademarks of OpenAI. Names are used only to identify compatible services. Licenses for bundled third-party software are collected in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

# Developing and running Rift

This is the practical guide for installation, local development, provider
boundaries, and releases. System topology lives in
[`ARCHITECTURE.md`](ARCHITECTURE.md).

## End-user setup

1. Run the latest `Rift-win-Setup.exe`. Rift installs per user and does not
   require administrator access.
2. If SmartScreen appears, choose **More info → Run anyway**. Public builds are
   not code-signed.
3. Open **Settings → Providers** and connect at least one route:
   - **ChatGPT subscription:** install the standalone Codex CLI with
     `npm install -g @openai/codex`, then use Rift's ChatGPT sign-in/check flow.
   - **Claude:** install Claude Code with
     `npm install -g @anthropic-ai/claude-code`, then complete its login.
   - **Optional API access:** save an Anthropic or OpenAI API key in the matching
     provider card. OpenAI API usage is separate from ChatGPT subscription
     billing.
4. Open a local folder and start a conversation. File, search, Git, and GitHub
   tools remain scoped to that workspace and the selected permission mode.

If a newly installed CLI is not detected, restart Rift first. Windows GUI apps
can inherit an older PATH; signing out or rebooting refreshes it when needed.

Rift checks for app updates on launch and periodically afterward. A user accepts
an update once; Velopack downloads it in the background and applies it on the
next restart.

## Build from source

### Prerequisites

- Windows 11 for the supported packaged target.
- Node.js 20+ and npm.
- Rust via rustup. `rust-toolchain.toml` pins the compiler and Clippy component
  used by CI.
- Git Bash for the CDP shell helper.

```powershell
git clone https://github.com/Blazzer10200/rift-tauri.git
Set-Location rift-tauri
npm install
```

### Run development

```powershell
npm run cdp:dev
```

This is the supported launcher for a debuggable app: it keeps the development
WebView profile separate from installed Rift and waits for CDP. `npm run tauri
dev` remains available when CDP is unnecessary.

Do not run Cargo checks while Tauri development is active; both write the same
incremental build state. The dev console is the Rust feedback loop until the app
is stopped.

### Project map

| Location | Purpose |
|---|---|
| `src/` | SvelteKit/Svelte 5 frontend |
| `src/lib/state/assistant/` | send, streaming, persistence, tabs, provider display |
| `src/lib/components/assistant/` | chat, composer, stream, and persisted block UI |
| `src-tauri/src/assistant/` | Claude, Codex App Server, OpenAI API, tools, conversations |
| `src-tauri/src/stt/` | microphone, VAD, Parakeet, and optional Whisper |
| `src-tauri/capabilities/` | Tauri WebView permission grants |
| `design-system/` | static visual reference and token mirror |
| `scripts/cdp/` | local WebView inspection bridge |
| `docs/` | architecture, security, current changelog, local handoff/issues |

For task-specific entrypoints, use the fast-navigation table in
[`ARCHITECTURE.md`](ARCHITECTURE.md).

### Verification

```powershell
npm run doctor
npm run check
npm test
cargo test --manifest-path src-tauri/Cargo.toml
```

`npm run verify` runs the complete frontend and Rust gate and refuses to collide
with a running Tauri dev process. `npm run verify:frontend` and
`npm run verify:rust` are the focused variants.

Do not hand-edit version files independently. `pwsh scripts/bump.ps1 X.Y.Z`
keeps `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and
`src-tauri/Cargo.lock` aligned.

## Provider routes

See [`CHATGPT.md`](CHATGPT.md) for the current model, exact effort, Fast-mode,
and billing-route contracts.

### ChatGPT subscription

Rift discovers a runnable standalone `codex`, launches the CLI's official login
flow, and reads account/model/skill/usage information through the local App
Server. Turns use `initialize`, thread start/resume, and turn start. Rift maps
streamed text, reasoning, tools, approvals, questions, usage, and cancellation
into its shared UI while persisting only the Codex thread ID. Rift never reads
or copies Codex credential files. Model capabilities come from live
`model/list`; never replace them with a static guessed effort or Fast table.

### OpenAI API

The optional API route uses `/v1/responses` with streaming and `store: false`.
Its key lives in Windows Credential Manager. Rift owns the canonical Responses
history locally, including opaque items required for continuation. Environment
`OPENAI_API_KEY` values are not silently adopted.

API model capabilities are reviewed metadata because `/v1/models` reports
access but not context windows, effort ranges, or Priority-processing support.
Unknown account-visible models keep unverified capabilities disabled until the
registry is updated from official model docs.

Each GPT conversation pins one of these two routes on its first turn. If that
route later becomes unavailable, Rift fails visibly instead of switching billing
paths.

### Claude

Claude turns use the official CLI with Rift's workspace-scoped stdio MCP server.
The prompt travels over stdin, and the CLI owns continuation through session IDs.
An optional Anthropic key saved in Rift is isolated from the OpenAI key slot.

## Releases

Releases are tag-driven CI. Do not run `scripts/release.ps1` locally for a normal
ship.

1. Stop the repo-scoped development stack and run `npm run verify`.
2. Run `pwsh scripts/bump.ps1 X.Y.Z`, update `docs/CHANGELOG.md`, review, and
   create a signed commit.
3. Create a signed annotated `vX.Y.Z` tag.
4. Push the tag first, then `main`.
5. Monitor with `pwsh scripts/ship-watch.ps1 vX.Y.Z -TimeoutSeconds 1800`.
6. Independently verify the public R2 feed/packages and the GitHub release
   assets. A source push or a single green endpoint is not a completed release.

The self-hosted Windows runner tests, builds, and packages once. Publication is
feed-first: R2 is the installed-client source; GitHub is the human download
page.

The `vpk` CLI version must equal the pinned `velopack` crate version. When
dependencies change, regenerate `THIRD-PARTY-NOTICES.md` with
`python scripts/gen-third-party-notices.py`.

For packaged smoke tests, never launch `rift-tauri.exe` from a shared raw Cargo
release directory. Use a clean allowlisted staging directory containing only the
app and explicitly required runtime sidecars; Windows can otherwise load an
unrelated adjacent DLL.

## Local paths and environment

End users need no environment variables. Rift stores app configuration under
`~/.rift/`, speech models under `~/.rift/models/`, and provider API keys in the
OS keychain.

Development-only variables include:

| Variable | Purpose |
|---|---|
| `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` | Enables local WebView CDP |
| `WEBVIEW2_USER_DATA_FOLDER` | Separates development and installed profiles |
| `RIFT_CDP_MAX_EDGE`, `RIFT_CDP_MAX_TOKENS` | Adjust screenshot bounds |
| `RIFT_CDP_SS_FACTOR` | Adjust screenshot supersampling |
| `RIFT_MCP_SERVER` | Internal child-process marker; never set manually |

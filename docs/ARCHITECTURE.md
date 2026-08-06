# Rift — Architecture

> Durable system map. Running/building lives in [`DEVELOPING.md`](DEVELOPING.md);
> security boundaries live in [`SECURITY.md`](SECURITY.md); ChatGPT model,
> effort, and Fast contracts live in [`CHATGPT.md`](CHATGPT.md).

## System shape

Rift is a local Tauri 2 desktop app with one Svelte 5 interface and three
explicit assistant routes:

1. Claude through the official Claude Code CLI.
2. ChatGPT subscriptions through the signed-in standalone Codex CLI's local
   App Server.
3. Optional, separately billed OpenAI API access through Rift's native
   Responses API adapter.

All routes share workspace containment, permissions, tool rendering, streaming
events, and local conversation metadata. Authentication, model catalogs,
usage, continuation state, and billing stay route-specific.

| Layer | Technology | Location |
|---|---|---|
| Desktop shell | Tauri 2 / Rust | `src-tauri/` |
| Frontend | SvelteKit SPA / Svelte 5 runes | `src/` |
| Assistant runtime | Claude CLI, Codex App Server, Responses API | `src-tauri/src/assistant/` |
| Distribution | Velopack over Cloudflare R2 | `src-tauri/src/update_service.rs`, `scripts/release.ps1` |

## Fast navigation

| Change | Start here |
|---|---|
| Provider account, auth, models, or health | `src/lib/state/assistant.svelte.ts`, `src/lib/state/assistant/providerDisplay.ts`, `src-tauri/src/assistant/{auth_update,codex,codex_app_server,openai}.rs` |
| Send, continuation, streaming, or cancellation | `src/lib/state/assistant/{send,streaming,persistence}.ts`, `src-tauri/src/assistant/{turn,codex_app_server,openai}.rs` |
| Shared workspace tools and permissions | `src-tauri/src/assistant/{mcp_server,mcp_bridge,permission,git_local}.rs` |
| Model and reasoning controls | `src/lib/components/assistant/Composer.svelte`, `src/lib/components/assistant/composer/{SettingsMenu.svelte,modelMatrix.ts}` |
| Settings and onboarding | `src/lib/components/settings/SettingsPage.svelte`, `src/lib/components/onboarding/` |
| Voice input and cleanup | `src/lib/state/stt.svelte.ts`, `src-tauri/src/stt/`, `src-tauri/src/assistant/transcript_cleanup.rs` |
| Live UI inspection | `.agents/skills/rift-ui/SKILL.md`, `scripts/cdp/c.sh`, `npm run cdp:doctor` |
| Local health and gates | `npm run doctor`, `npm run check`, `npm test`, `npm run verify` |

Generated or dependency trees are not navigation targets: `node_modules/`,
`.svelte-kit/`, `build/`, `src-tauri/target/`, and `src-tauri/gen/`.

## Turn lifecycle

```text
Composer -> assistant.send()
  |-- Claude -> assistant_send -> turn.rs -> warm claude process + stdio MCP
  |-- ChatGPT subscription -> assistant_codex_send -> codex_app_server.rs
  |     -> initialize -> thread start/resume -> turn start
  `-- OpenAI API -> assistant_openai_send -> openai.rs -> /v1/responses

All routes -> assistant://stream | done | error | permission-request | ask-user
  -> streaming.ts routes by window, session, and turn epoch
  -> MessageBubble / ToolChip / EditDiff / StreamTurn
```

`codex_app_server.rs` is the protocol boundary for subscription turns. It
normalizes current App Server items and deltas into shared envelopes: live
command output becomes `tool_progress`, patch updates become one file tool per
path, and completed items replace partial state. Frontend renderers therefore
remain provider-neutral and saved conversations keep the same block model.

History ownership is deliberate:

- Claude owns its CLI session; Rift persists the session ID.
- Codex owns the ChatGPT thread; Rift persists only the Codex thread ID.
- OpenAI API requests use `store: false`; Rift persists canonical Responses
  items locally, including opaque reasoning/tool/compaction items required for
  continuation.
- A GPT conversation pins `codex` or `openai` on its first turn. Missing access
  fails visibly and never switches billing routes.

Prompts never enter a process argument: Claude receives stream JSON on stdin,
Codex receives JSON-RPC over stdin, and the API route uses an HTTPS body.

## Backend map

`src-tauri/src/lib.rs` registers the Tauri commands. `src-tauri/src/main.rs` is
the thin binary entrypoint. Table paths are relative to `src-tauri/src/`.

| Area | Responsibility |
|---|---|
| `assistant/turn.rs` | Claude live turns, process/session registry, stream and permission events |
| `assistant/warm_pool.rs` | Persistent per-session Claude child and idle/death recovery |
| `assistant/codex.rs` | Standalone Codex discovery, status, and interactive login |
| `assistant/codex_app_server.rs` | ChatGPT account overview, model/skill/usage discovery, thread turns, approvals, tools, and cancellation |
| `assistant/openai.rs` | Native Responses API transport, SSE, local continuation, images, tools, and usage |
| `assistant/mcp_server.rs` | Shared workspace file/search/git/GitHub tool catalog for Claude |
| `assistant/mcp_bridge.rs`, `bridge.rs` | Token-gated loopback round trips for `ask_user`, `open_browser`, and `notify` |
| `assistant/git_local.rs`, `gh_remote.rs` | Hardened local Git and origin-pinned GitHub operations |
| `assistant/convo_store.rs` | Conversation persistence, route metadata, and migration |
| `assistant/transcript_cleanup.rs`, `oneshot.rs` | Provider-aware voice cleanup and other bounded one-off assistant calls |
| `stt/` | Web Speech plus default Parakeet and optional Whisper engines |
| `diagnostics/` | Structured event bus, logs, performance records, and panic forwarding |
| `browser/` | In-app browser child WebView |
| `state/paths.rs`, `secrets.rs` | Canonical `~/.rift/` paths and OS-keychain secrets |
| `update_service.rs` | R2 feed checks, download/apply, relaunch, and child-process reaping |

## Frontend map

SvelteKit supplies the SPA build; runtime screens live under `src/lib`. Paths
below are relative to that directory.

- `state/assistant.svelte.ts` owns the public assistant store. Its modules under
  `state/assistant/` own sending, streaming, tabs, persistence, attachments,
  workspace state, health alerts, and shared types.
- `components/assistant/` owns chat rendering. `Composer.svelte` keeps keyboard
  orchestration; `composer/` contains the model/settings/queue/attachment
  surfaces. `stream/`, `bubble/`, and `toolchip/` render live and persisted
  blocks.
- `components/settings/SettingsPage.svelte` is the single provider setup and
  product settings surface.
- `components/workspace/`, `ai-health/`, `diagnostics/`, `webview/`,
  `onboarding/`, and `shell/` own the remaining screens and chrome.
- `components/workspaces/index.ts` is the lazy screen registry. The legacy
  `projects` ID remains only as a persisted-state migration alias.

## Load-bearing invariants

- Provider-neutral stored effort tiers preserve legacy meanings. Claude mapping
  changes together in `helpers.ts`, `config.rs`, and `turn.rs`; Codex and API
  mappings change together in `modelMatrix.ts`, `codex_app_server.rs`, and
  `openai.rs`. App Server capability sets remain live catalog data.
- Fast availability is route-specific and revalidated in the backend. Codex
  uses live service tiers; API uses reviewed model metadata; result chips require
  provider confirmation rather than the frontend request bit.
- App Server protocol changes stay normalized in `codex_app_server.rs`. Never
  key failure state on presence of nullable fields, collapse a multi-path
  `fileChange` into one opaque card, or drop live output/patch/progress events.
- Slash-command discovery stays route-aware: stable built-in lanes precede
  provider-native commands and skills, while insertion preserves `/` versus
  `$` syntax and never sends one provider's catalog through another route.
- Persisted conversation fields and provider-route migration remain backward
  compatible.
- Tauri capabilities scope local grants by `webviews`, not whole `windows`; the
  browser child can display arbitrary remote pages.
- Velopack apply must reap every Rift-spawned child before exit because a child
  can lock the installed `current/` directory and make the swap silently fail.
- Never kill `rift-tauri.exe` by image name. Installed and development builds
  share it; cleanup must verify executable paths and target PIDs.
- Release versions remain aligned in `package.json`,
  `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and
  `src-tauri/Cargo.lock`. The `vpk` CLI and `velopack` crate stay on the same
  version.

## Release topology

A pushed `v*` tag starts `.github/workflows/release.yml`. The self-hosted Windows
runner verifies, builds, and packages once, then publishes feed-first: R2 is the
client update source; GitHub is the human download page. A release is complete
only after both public locations are checked independently.

Removed SFTP/sync/server/RCON, cost-cockpit, compaction, swarm, and old SDK
abstractions stay removed. Git history is the archive; do not recreate them as
parallel systems.

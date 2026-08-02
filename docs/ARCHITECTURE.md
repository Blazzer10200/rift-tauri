# Rift — Architecture

> How the whole system fits together. For *running/building* see [`DEVELOPING.md`](DEVELOPING.md); for the *security model* see [`SECURITY.md`](SECURITY.md). This doc is the durable system map — update it when the topology changes, not every session.

## 1. One sentence

Rift is a Tauri 2 desktop app with two assistant engines—Claude through the official **Claude Code CLI** and OpenAI through the native **Responses API**—sharing one workspace-scoped tool, permission, streaming, and conversation UI.

## 2. The stack

| Layer | Tech | Where |
|---|---|---|
| Shell | Tauri 2 (Rust) | `src-tauri/` |
| Frontend | SvelteKit 2 SPA · Svelte 5 runes · Tailwind 4 | `src/` |
| Assistant engines | Claude CLI + stdio MCP; OpenAI Responses API + native function loop | `src-tauri/src/assistant/` |
| Distribution | NSIS first-install → Velopack self-update | `update_service.rs` · `scripts/release.ps1` |

### Fast navigation

| If you are changing... | Start here |
|---|---|
| Provider status, auth, or model readiness | `src/lib/state/assistant.svelte.ts`, `src/lib/state/assistant/providerDisplay.ts`, `src-tauri/src/assistant/{auth_update,codex,openai}.rs` |
| Sending, streaming, tools, or continuation state | `src/lib/state/assistant/{send,streaming}.ts`, `src-tauri/src/assistant/{turn,openai,mcp_server}.rs` |
| Composer model/effort controls | `src/lib/components/assistant/Composer.svelte`, `src/lib/components/assistant/composer/{SettingsMenu,modelMatrix}.ts` |
| Settings and connection UI | `src/lib/components/settings/SettingsPage.svelte`, `src/lib/components/onboarding/` |
| Live UI debugging | `.agents/skills/rift-ui/SKILL.md`, `scripts/cdp/c.sh`, then `npm run cdp:doctor` |
| Local correctness gates | `npm run doctor`, `npm run verify:frontend`, `npm run verify` |

The frontend is a single-window SPA (no SSR at runtime—SvelteKit is the build tool). The workspace registry (`src/lib/components/workspaces/index.ts`) has six IDs: **Workspace** (`home`, ⌨1), **Chat** (`chat`, ⌨2), legacy **`projects`** (folded to `home`), **Settings** (⌨4), **Diagnostics** (⌨5), and **AI Health** (⌨6). Each workspace is a dynamic import: only the active screen loads at startup, while every opened screen stays mounted afterward to preserve scroll, focus, and in-flight state. A fresh install defaults to `chat` (`workspace.svelte.ts`).

## 3. Request lifecycle — a turn, end to end

```
Composer (Svelte) → assistant.send(prompt, tabId?)
  ├─ Claude model → invoke("assistant_send")
  │    └─ turn.rs → warm `claude` subprocess + local stdio MCP server
  │         └─ CLI owns session history through --session-id / --resume
  └─ GPT model → invoke("assistant_openai_send")
       └─ openai.rs → POST /v1/responses (stream=true, store=false)
            ├─ frontend supplies locally persisted opaque Responses items
            └─ native function-call loop uses the shared tool definitions/executor

Both routes → assistant://stream · assistant://done · assistant://error
  └─ streaming.ts filters by session id and drives MessageBubble / ToolChip / EditDiff
```

Key properties:
- **Events are scoped to the originating window** (`app.emit_to(window_label, …)`, `turn.rs`) so a second window never sees another window's turn events. Within a window, frames carry the `cliSessionId`; `streaming.ts` applies them to the matching tab. This is what makes multi-tab / multi-pane / multi-window work — the backend partitions per-window, the store routes per-session.
- **History ownership is provider-specific.** Claude history belongs to the CLI (`--session-id` / `--resume`). OpenAI responses use `store: false`; Rift persists the canonical Responses input/output items locally, including opaque encrypted reasoning, tool, and compaction items. It never reconstructs that continuation state from rendered chat bubbles.
- **The prompt is never an argv.** It goes in on stdin as a stream-json user envelope — no argument-injection surface.

## 4. Backend (`src-tauri/src/`)

`lib.rs` is the Tauri entry: it registers ~95 `#[tauri::command]`s (most live per-domain in `commands/*.rs`; `stt::*` and `usage::limits` register directly from their own modules) and runs `VelopackApp::build().run()` early for install/update hooks. `main.rs` is the thin binary.

### `assistant/` — the engine
| File | Role |
|---|---|
| `turn.rs` | Claude live-turn nervous system: session registry, CLI spawn, stream/permission/error event emit, stop, per-turn env snapshot. |
| `openai.rs` | Native OpenAI Responses API: key/model checks, SSE decoding, usage events, cancellation, reasoning configuration, image inputs, and the bounded function-call loop. |
| `codex.rs` | Local Codex CLI discovery and official ChatGPT browser-login bridge. It intentionally does not read Codex credential files; an App Server turn adapter remains tracked separately. |
| `mcp_server.rs` | stdio JSON-RPC MCP server: `read_file` / `list_dir` / `grep` + `git_*` + `gh_*` + the bridge-gated UI tools (dispatched from here, implemented in `mcp_bridge.rs`). Workspace-scoped, trust-gated. |
| `mcp_bridge.rs` | The bridge-gated UI MCP tools — `ask_user` / `open_browser` / `notify` + the loopback `bridge_call` round-trip. Split out of `mcp_server.rs` (v0.60.0). |
| `git_local.rs` | Hardened `run_git` (no shell, args pre-split, env stripped, non-interactive) + path/message validators. Backs the `git_*` MCP tools in `mcp_server.rs` (there is no `commands/git.rs`). |
| `gh_remote.rs` | GitHub via the user's own `gh` CLI (tokenless; repo pinned to `origin`): `gh_checks` / `gh_run_view` / `gh_pr_list` / `gh_pr_view` / `gh_pr_diff` MCP tools + trust-gated `gh_pr_create`, and the `gh_branch_status` command behind the UI branch chip + popover. |
| `bridge.rs` | Loopback TCP UI bridge (127.0.0.1, ephemeral port, 192-bit token) so MCP tools can round-trip `ask_user`/`open_browser`/`notify` through the running webview. |
| `convo_store.rs` | On-disk conversation persistence + export. |
| `oneshot.rs` | One-off CLI calls (prompt-enhance, title, usage-analyze) outside the live turn loop. |
| `local_llm.rs` | Ollama context-probe/optimize commands + the shared `probe_messages`/`probe_models` HTTP probes `providers.rs` tests through. Split out of `oneshot.rs` (v0.60.0). |
| `providers.rs` | Multi-model provider registry (Kimi / DeepSeek / GLM / OpenRouter / local): named endpoint profiles; activating one copies base_url/model/key/effort into the `local_llm_*` wire fields + `LOCAL_LLM_API_KEY`, so `turn.rs` needs zero provider awareness. Effort-capable profiles get the composer effort ladder + a direct (no-shim) route when thinking is on. Per-provider keys in the OS keychain. |
| `warm_pool.rs` | Persistent per-session CLI child (warm process) so subsequent turns skip cold-start; idle-evicts, transparently respawns on dead pipe. |
| `projects.rs` | Named folder aliases with include/exclude file scoping (`assistant_list/save/delete_project`); validates globs via the shared `glob_to_regex`. |
| `permission.rs` · `ask_user.rs` · `config.rs` · `workspace.rs` · `auth_update.rs` · `cli_install.rs` · `cli_caps.rs` · `env_checks.rs` · `nothink.rs` | permission plumbing · UI-ask registry · per-turn config (effort/model clamps) · workspace roots · auth probe · CLI install detection · CLI capability probe · environment preflight · thinking-flag handling. |

### Other backend domains
- `commands/` — frontend-facing IPC: `assistant.rs`, `browser.rs`, `update.rs`. (Git working-tree state is served by the `git_*` MCP tools in `assistant/git_local.rs`, not a `commands/git.rs`.)
- `state/` — app-paths module (`paths.rs`): canonical `~/.rift/` locations (config, models).
- `diagnostics/` — `DiagBus` + log forwarder + panic hook; pumps to the frontend over `diag://event`.
- `usage/limits.rs` — OAuth `/usage` rate-limit fetch (the only usage module; read-only on the CLI token).
- `stt/` — speech-to-text (Web Speech bridge + local Whisper), events on `stt://*`.
- `browser/` — in-app browser dock control.
- `secrets.rs` — OS keychain wrapper (`keyring`) with separate Anthropic and OpenAI slots. `update_service.rs` — Velopack `UpdateManager` over an `HttpSource` (Cloudflare R2 feed).

## 5. Frontend (`src/`)

SvelteKit routes mount under `src/routes`; the bulk is `src/lib`.

### State (`src/lib/state/`)
Svelte-5 runes-class singletons (the `export const store = new Store()` pattern). The big one is the **assistant store**, split into `src/lib/state/assistant/`:
| Module | Role |
|---|---|
| `send.ts` | Turn orchestrator — entry point from the composer. |
| `streaming.ts` | Stream pump: listens to `assistant://*`, filters by session id, applies frames to the active tab. |
| `tabs.ts` · `types.ts` | Multi-tab / multi-pane state (`PaneState`, `MAX_PANES=4`), persisted to `localStorage`. |
| `persistence.ts` · `telemetry.ts` · `attachments.ts` · `workspace.ts` · `healthAlerts.ts` · `helpers.ts` | disk save · usage rollups · file attachments · per-tab workspace root · health banners · effort/model mapping. |

Other stores: `environment.svelte.ts` (host-tool presence — git/node/npm/cargo/code, probed once via `environment_check` and cached to hide dead affordances), `projects.svelte.ts` (named-folder registry), `usage.svelte.ts` (rate-limit gauges), `cliUpdate.svelte.ts` + `updates.svelte.ts` (CLI + app update notices), `stt.svelte.ts`, `browserDock.svelte.ts`, `localLlm.svelte.ts`, `workspace.svelte.ts`, `ui-prefs.svelte.ts`, `toast.svelte.ts`.

### Components (`src/lib/components/`)
- `assistant/` — the Chat surface: `MessageBubble`, `ToolChip`, `EditDiff`, `Markdown`, `Composer` (split into `composer/*`), `AssistantPane`, `AssistantPage`, `AssistantWelcome` (warm/cold welcome), `PermissionBar`. Sub-agent dispatches render **inline** as cards (`stream/StreamAgent` live · `toolchip/AgentCard` persisted) — no floating dock.
- `workspace/` — `WorkspacePage.svelte`, the merged Workspace (home) surface. Provider setup lives only in Settings → Providers; `NewsFeed.svelte` labels Claude's independently verified source coverage rather than implying every connected provider supplies a feed. `globPreview.ts` + `welcomeShared.ts` back its glob validation and shared greeting.
- `shell/` — `Titlebar` (custom drag region — needs `core:window:allow-start-dragging`), `WorkspaceShell`, `Sidebar`, `Topbar`, `StatusBar`, `ConversationList`, `ProjectRail`, `ContextMenuHost`, `RiftLogo`. (`tabsbar/` holds only pure helpers now — `ChatTabsBar` was folded in.)
- `home/` (`statsHelpers.ts` — the pure usage-stat aggregation WorkspacePage + AiHealth both read) · `settings/` · `local-llm/` · `ai-health/` · `webview/` · `onboarding/` — the other workspaces, onboarding, and browser pane.

### Cross-cutting invariants
- **Effort mapping is lockstep** across three places: `state/assistant/helpers.ts::effortToFlag` ↔ `turn.rs` match ↔ `composer/modelMatrix.ts`. Change all three together.
- **Accent is one hue** — `--accent-h` (emerald 163) drives the whole OKLCH ramp in `app.css`.
- **Design tokens** live in `app.css` (`--radius-*`, `--fs-*`, `--dur-*`/`--ease-*` motion vocabulary). Prefer tokens over literals.

## 6. The UI bridge (`bridge.rs`)

The MCP server runs as a child process and can't touch the webview directly. For the three UI tools (`ask_user`, `open_browser`, `notify`) it opens a loopback TCP connection back to the main process: bound to `127.0.0.1:0` (ephemeral), gated by a 192-bit CSPRNG token minted per launch. The main process then `emit`s to the webview (`assistant://ask-user` / `assistant://open-browser` / `assistant://notify`) and parks the tool call until the user answers (10-min timeout). See [`SECURITY.md`](SECURITY.md) for the threat model.

## 7. Self-update (Velopack)

`update_service.rs` wraps `velopack::UpdateManager` over a Velopack `HttpSource` pointed at the Cloudflare R2 feed (`UPDATE_FEED_URL`, `update_service.rs:35`). Flow: check on launch + every 6h → background download with progress (`update-progress`/`update-downloaded`) → on consent, `wait_exit_then_apply_updates(silent, restart)`. **Critical:** before exit, `apply()` reaps the per-turn `rift-tauri.exe` MCP children (they lock `current/`, and `app.exit(0)` skips `Drop` so `kill_on_drop` never fires). The CLI child's cwd defaulting to `temp_dir()` (not the install dir) is the load-bearing prevention added in v0.12.3.

## 8. Build & release

Tag-driven CI: push a `v*` tag → `.github/workflows/release.yml` builds + packs (Velopack `vpk`) + publishes **feed-first**: Cloudflare R2 (the live update feed clients read — `update_service.rs::UPDATE_FEED_URL`) before the GitHub release (human download page; retried 3×). The client-critical path never touches GitHub — a GitHub outage can delay the release page, never user updates (2026-07-16 lesson). **Version lockstep across THREE files** — `package.json` · `src-tauri/Cargo.toml` · `src-tauri/tauri.conf.json` (+ `Cargo.lock`); `release.ps1` preflight bails on mismatch. The `vpk` CLI version MUST equal the `velopack` crate version (both pinned `=1.2.0`).

## 9. Hot-file split invariants (don't regress a load-bearing seam)

Four hot files were split into module dirs; step detail in `git log`. Forward-looking invariants a future refactor must preserve:

- **`assistant/mod.rs`** — every `#[tauri::command]` is registered by path in `lib.rs`; extracted modules re-export commands (`pub use cli_install::*;`) so the registry never churns. `kill_all_session_children()` is load-bearing for the Velopack apply — keep it on whatever module owns `SESSION_PIDS`. Process-state statics (`SESSION_PIDS`/`SESSION_STOPPED`/`CLAUDE_EXE`/`MCP_CFG_SEQ`/`CONFIG_WRITE_LOCK`) move with their accessor cluster, never duplicated. `McpConfigGuard` (Drop) stays paired with `write_mcp_config` (the Drop deletes the temp config — splitting invites a leak).
- **`assistant.svelte.ts`** — per-tab fields live on `TabState` with a getter on `AssistantStore`, never back on the store (compaction/queue/draft/attachments/messages/streaming/agentSpawns/askUser bindings are all per-tab). `import { assistant } from "$lib/state/assistant.svelte"` MUST keep working — extracted concerns re-export at the same shape. Persisted JSON contract (`compactionHistory[]` camelCase, `openTabs`/`panes`/`currentConvoId` shapes) is locked; shared defs live in `types.ts`. `TabState` ctor signature locked (`ensureTab` passes `(cliSessionId)`).
- **`Composer.svelte`** — `onKey` is the one keyboard handler (slash menu, mention popover, queue recall, Enter to fire-or-queue); stays in the parent, children get open/close/index via props. `fire()`/`onBtnClick`/send-button `mode` stay in the parent. (Enter while streaming queues for the next turn — no mid-turn steer; removed v0.75.0.) `.composer-wrap` sets `--model-color`; children consume the variable, never `:global()`.
- **`shell/tabsbar/`** (was `ChatTabsBar.svelte`) — `portal` action is canonical at `$lib/actions/portal.ts` (`portalFocus` is the focus-first variant); don't re-fork. Drag-reorder moves as ONE unit — the six handlers + the `window` `dragend` listener (WebView2 missed-dragend workaround) + its `onDestroy` teardown. `:global` selectors move with their element's owner.

## 10. What was removed (so you don't go looking)

The pure-assistant conversion (2026-06-03) + minimal-core strip (2026-06-12) deleted the SFTP / sync / server / RCON / tunnel / transport stack, Swarm + cost-cockpit + compaction subsystems, the old multi-SDK abstraction, and the SQLite usage DB. Do not look for those systems. The current two-engine design is intentionally narrow: Claude CLI plus one native OpenAI Responses implementation, both behind the same Rift event and tool contracts. Compatible endpoint profiles remain a Claude-CLI route, not a third assistant engine. History is in `git log`.

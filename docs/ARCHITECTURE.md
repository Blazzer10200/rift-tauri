# Rift — Architecture

> How the whole system fits together. For *running/building* see [`DEVELOPING.md`](DEVELOPING.md); for the *security model* see [`SECURITY.md`](SECURITY.md); for *current in-flight state* see [`HANDOFF.md`](HANDOFF.md). This doc is the durable system map — update it when the topology changes, not every session.

## 1. One sentence

Rift is a Tauri 2 desktop app that wraps the **Claude CLI** as a per-turn subprocess and feeds it a **local stdio MCP server** scoped to a chosen workspace folder, so Claude can read / search / edit / run-git against that folder — entirely on-device, no remote connections.

## 2. The stack

| Layer | Tech | Where |
|---|---|---|
| Shell | Tauri 2 (Rust) | `src-tauri/` |
| Frontend | SvelteKit 2 SPA · Svelte 5 runes · Tailwind 4 | `src/` |
| Assistant engine | Claude CLI subprocess + stdio MCP server | `src-tauri/src/assistant/` |
| Distribution | NSIS first-install → Velopack self-update | `update_service.rs` · `scripts/release.ps1` |

The frontend is a single-window SPA (no SSR at runtime — SvelteKit is the build tool). The workspace registry (`src/lib/components/workspaces/index.ts`) has six IDs, keyboard-switchable: **Workspace** (`home`, ⌨1) · **Chat** (`chat`, ⌨2) · a legacy **`projects`** alias (⌨3, folded to `home` on load) · **Settings** (⌨4) · **Local LLM** (⌨5) · **AI Health** (⌨6). The `home` id mounts `WorkspacePage.svelte` — the merged Home+Projects surface (greeting, current-folder context, resume-a-chat cards, project manager, "adopt an existing folder"). `init()` migrates a stored `projects` active-id to `home` (one-time, for users who had Projects active); a fresh install with no stored value defaults to `chat` (`workspace.svelte.ts`). The empty-Chat warm/cold welcome is `AssistantWelcome.svelte` (under `assistant/`). `goHome()` is still "Home is a verb" — it floats to an empty Chat tab and is what opening a project / resuming a chat calls after picking context.

## 3. Request lifecycle — a turn, end to end

```
Composer (Svelte)
  └─ assistant.send(prompt, tabId?)            src/lib/state/assistant/send.ts
       └─ invoke("assistant_send", …)          → Tauri IPC
            └─ commands/assistant.rs → assistant::turn::assistant_send
                 └─ spawn `claude` subprocess   turn.rs
                      • --mcp-config <rift mcp>  (Rift's own stdio MCP server)
                      • --allowed-tools mcp__rift__*
                      • --session-id (1st) / --resume (subsequent)
                      • --model / --permission-mode / effort flag
                      • prompt delivered on STDIN as a stream-json envelope
                      • cwd = workspace root (else temp_dir — never the install dir)
                 ⇅ stdio JSON-RPC
            ┌─ Claude calls tools → Rift's MCP server (mcp_server.rs)
            │    read_file / list_dir / grep / git_* / ask_user / open_browser / notify
            └─ Claude streams assistant/tool/result frames back
                 └─ turn.rs re-emits as app-wide Tauri events (carry cliSessionId):
                      assistant://stream · assistant://done · assistant://error
                      assistant://permission-request
       ◄─ streaming.ts listens, FILTERS by session id, drives the store
  MessageBubble / ToolChip / EditDiff render the live transcript
```

Key properties:
- **Events are scoped to the originating window** (`app.emit_to(window_label, …)`, `turn.rs`) so a second window never sees another window's turn events. Within a window, frames carry the `cliSessionId`; `streaming.ts` applies them to the matching tab. This is what makes multi-tab / multi-pane / multi-window work — the backend partitions per-window, the store routes per-session.
- **The CLI owns conversation state.** Rift passes `--session-id` on turn 1 and `--resume` after; it never reconstructs history server-side. Session loss auto-recovers as a fresh start (`assistant://session-lost`).
- **The prompt is never an argv.** It goes in on stdin as a stream-json user envelope — no argument-injection surface.

## 4. Backend (`src-tauri/src/`)

`lib.rs` is the Tauri entry: it registers ~79 `#[tauri::command]`s (most live per-domain in `commands/*.rs`; `stt::*` and `usage::limits` register directly from their own modules) and runs `VelopackApp::build().run()` early for install/update hooks. `main.rs` is the thin binary.

### `assistant/` — the engine
| File | Role |
|---|---|
| `turn.rs` | Live-turn nervous system: session registry, CLI spawn, stream/permission/error event emit, steer/stop, per-turn env snapshot. The hot file. |
| `mcp_server.rs` | stdio JSON-RPC MCP server: `read_file` / `list_dir` / `grep` + `git_*` + bridge-gated `ask_user` / `open_browser` / `notify`. Workspace-scoped, trust-gated. |
| `git_local.rs` | Hardened `run_git` (no shell, args pre-split, env stripped, non-interactive) + path/message validators. Backs both the MCP git tools and `commands/git.rs`. |
| `bridge.rs` | Loopback TCP UI bridge (127.0.0.1, ephemeral port, 192-bit token) so MCP tools can round-trip `ask_user`/`open_browser`/`notify` through the running webview. |
| `convo_store.rs` | On-disk conversation persistence + export. |
| `oneshot.rs` | One-off CLI calls (prompt-enhance, title) outside the live turn loop. |
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
- `secrets.rs` — OS keychain wrapper (`keyring`) for the API key. `update_service.rs` — Velopack `UpdateManager` over an `HttpSource` (Cloudflare R2 feed).

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

Other stores: `environment.svelte.ts` (the Environment floating widget — pill/expanded panel; `open` auto-managed, `expanded` persisted), `projects.svelte.ts` (named-folder registry), `usage.svelte.ts` (rate-limit gauges), `cliUpdate.svelte.ts` + `updates.svelte.ts` (CLI + app update notices), `stt.svelte.ts`, `browserDock.svelte.ts` / `activityDock.svelte.ts`, `localLlm.svelte.ts`, `workspace.svelte.ts`, `ui-prefs.svelte.ts`, `toast.svelte.ts`.

### Components (`src/lib/components/`)
- `assistant/` — the Chat surface: `MessageBubble`, `ToolChip`, `EditDiff`, `Markdown`, `Composer` (split into `composer/*`), `AssistantPane`, `AssistantPage`, `AssistantWelcome` (warm/cold welcome), `PermissionBar`, `SubAgentDock`.
- `workspace/` — `WorkspacePage.svelte`, the merged Workspace (home) surface; `globPreview.ts` + `welcomeShared.ts` back its glob validation and shared greeting.
- `shell/` — `Titlebar` (custom drag region — needs `core:window:allow-start-dragging`), `WorkspaceShell`, `Sidebar`, `Topbar`, `StatusBar`, `ConversationList`, `ProjectSwitcher`, `ContextMenuHost`, `RiftLogo`. (`tabsbar/` holds only pure helpers now — `ChatTabsBar` was folded in.)
- `home/` (the `StatsPanel` widget WorkspacePage embeds) · `settings/` · `local-llm/` · `ai-health/` · `webview/` · `onboarding/` — the other workspaces, onboarding, and browser pane.

### Cross-cutting invariants
- **Effort mapping is lockstep** across three places: `state/assistant/helpers.ts::effortToFlag` ↔ `turn.rs` match ↔ `composer/modelMatrix.ts`. Change all three together.
- **Accent is one hue** — `--accent-h` (emerald 163) drives the whole OKLCH ramp in `app.css`.
- **Design tokens** live in `app.css` (`--radius-*`, `--fs-*`, `--dur-*`/`--ease-*` motion vocabulary). Prefer tokens over literals.

## 6. The UI bridge (`bridge.rs`)

The MCP server runs as a child process and can't touch the webview directly. For the three UI tools (`ask_user`, `open_browser`, `notify`) it opens a loopback TCP connection back to the main process: bound to `127.0.0.1:0` (ephemeral), gated by a 192-bit CSPRNG token minted per launch. The main process then `emit`s to the webview (`assistant://ask-user` / `assistant://open-browser` / `assistant://notify`) and parks the tool call until the user answers (10-min timeout). See [`SECURITY.md`](SECURITY.md) for the threat model.

## 7. Self-update (Velopack)

`update_service.rs` wraps `velopack::UpdateManager` over a Velopack `HttpSource` pointed at the Cloudflare R2 feed (`UPDATE_FEED_URL`, `update_service.rs:34`). Flow: check on launch + every 6h → background download with progress (`update-progress`/`update-downloaded`) → on consent, `wait_exit_then_apply_updates(silent, restart)`. **Critical:** before exit, `apply()` reaps the per-turn `rift-tauri.exe` MCP children (they lock `current/`, and `app.exit(0)` skips `Drop` so `kill_on_drop` never fires). The CLI child's cwd defaulting to `temp_dir()` (not the install dir) is the load-bearing prevention added in v0.12.3. Full lineage + rationale: `git log -- docs/design/velopack-auto-update.md` (arc doc retired after ship).

## 8. Build & release

Tag-driven CI: push a `v*` tag → `.github/workflows/release.yml` builds + packs (Velopack `vpk`) + publishes to the public `rift` repo (renamed from `rift-releases`, v0.16.2). **Version lockstep across THREE files** — `package.json` · `src-tauri/Cargo.toml` · `src-tauri/tauri.conf.json` (+ `Cargo.lock`); `release.ps1` preflight bails on mismatch. The `vpk` CLI version MUST equal the `velopack` crate version (both pinned `=1.2.0`).

## 9. What was removed (so you don't go looking)

The pure-assistant conversion (2026-06-03) + minimal-core strip (2026-06-12) deleted the entire SFTP / sync / server / RCON / tunnel / transport stack, the Swarm + cost-cockpit + compaction subsystems, custom providers, and the SQLite usage DB. Rift today is first-party Anthropic only, five active workspaces (Workspace · Chat · Settings · Local LLM · AI Health), no remote anything. History is in `git log`.

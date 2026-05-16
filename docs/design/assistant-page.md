# Assistant Page — Design Brief

> Status: planning only (session 60, 2026-05-14). No code yet. Next session executes from this doc.

## Goal

JetBrains-style AI partner embedded as a dedicated page in Rift. Claude account login (OAuth via Claude Agent SDK), side-panel chat, inline file-edit diffs, full visibility into what Claude is doing in real time. Themed to match the rest of Rift. Strict isolation from existing features — Assistant page is additive, never modifies Sync / Browse / Terminal / Activity behavior.

## Non-goals (v1)

Ghost-text inline completions as-you-type, voice input, image paste-and-analyze, multi-agent conversations, marketplace/extensions, debugger UI, git panel.

## Architecture (revised α1, 2026-05-14)

**Originally planned (Agent SDK in renderer) — REJECTED on implementation.** `@anthropic-ai/claude-agent-sdk` is a Node-only package that wraps the `claude` CLI by spawning it as a subprocess. It can't run inside Tauri's webview (no `child_process`/`fs`/Node APIs).

**Actual α1 architecture:** Rust spawns the user's installed `claude` CLI in headless streaming-JSON mode (`claude -p <prompt> --output-format stream-json --verbose --include-partial-messages --model sonnet --tools "" --strict-mcp-config --no-session-persistence --permission-mode dontAsk`). Rift parses NDJSON line-by-line and re-emits each line on the Tauri event channel `assistant://stream`, with terminal events `assistant://done` and `assistant://error`. Conversation state, message rendering, cost display all live Svelte-side; the SDK loop and API auth live inside the CLI binary the user already has installed.

Flow (α1): Svelte composer → `invoke("assistant_send")` → Rust spawns `claude -p …` → CLI streams NDJSON to stdout → Rust forwards lines as Tauri events → Svelte parses + renders.

**CSP update no longer required** — the API call originates from the CLI subprocess, not the webview.

For α2+ tool routing (Rift MCP server): the MCP server still lives Rust-side (`src-tauri/src/assistant/mcp_server.rs`), spawned independently and registered with the CLI via `--mcp-config`. CLI's built-in tools stay disabled. This preserves the original brief's safety-rail invariant.

**Auth model unchanged** — piggyback on the user's existing `claude login` session by default; API-key fallback spawns with `--bare` + `ANTHROPIC_API_KEY` env so the CLI ignores OAuth.

## Page placement

New top-level tab "Assistant" after Terminal, before Activity. Icon: `Sparkles` from lucide. Lazy-mounted under v0.2.55's keep-alive pattern (mounts on first visit, stays alive, conversation state survives tab switches via singleton store).

## State model

New singleton store `src/lib/state/assistant.svelte.ts` mirroring `sync-page.svelte.ts` shape. Fields: `conversations[]`, `activeConversationId`, `messages[]`, `pendingEdits[]`, `streaming`, `thinkingTokens`, `outputTokens`, `cost`, `currentActivity` (live status string), `status` (idle / thinking / tooling / streaming / waiting-accept / error), `signedIn`, `model`, `effortLevel`, `verboseLog[]`.

Persist conversations to `~/.rift/assistant/<conversationId>.json`. OAuth token in OS keychain only (new dep: `tauri-plugin-stronghold` or `keyring-rs` crate). Never in localStorage, never in `~/.rift/*.json`.

## Auth — piggyback model (research-verified 2026-05-14)

**Primary path: piggyback on user's existing Claude Code CLI session.** Anthropic explicitly prohibits third-party distributable products from offering claude.ai OAuth login (policy actively enforced April 2026+). Rift therefore does NOT implement any OAuth flow itself. Instead, Rift's Assistant page consumes the Agent SDK which bundles/wraps the Claude Code CLI binary — and the CLI handles its own auth via `claude login` / `claude setup-token` outside Rift's boundary. If the user is already logged into Claude Code on their machine, the SDK inherits that session automatically and routes calls through the user's Pro/Max subscription. Zero auth code, zero token storage, zero policy gray area on Rift's side.

**Fallback path: user-provided API key.** Settings panel accepts an `sk-ant-api03-` token from console.anthropic.com, stored in OS keychain via `keyring-rs` crate. Pay-per-token billing. Used when Claude Code CLI isn't installed or user prefers separate billing.

**Auth-state detection at page load.** Probe `claude --version` and check for an authenticated session (CLI exposes a status command). Three states surface in the Assistant page header pill:
- Green check, "Using Claude Code session" — CLI detected, logged in, SDK will inherit.
- Yellow warn, "API key configured" — no CLI session but API key saved.
- Red, "Not configured — install Claude Code CLI or add API key" — empty-state with install link to https://claude.com/download and inline API-key input.

**Non-goal: no claude.ai OAuth button in Rift.** Future sessions must not reintroduce this. Auth happens outside Rift's process boundary by design — that's what makes the piggyback model legal and zero-maintenance.

**Watch-out: June 15 2026 billing change.** Subscription-routed Agent SDK usage will draw from a separate "Agent SDK credit" pool distinct from interactive Claude Code limits. Surface this in the settings panel as an info note so users aren't surprised when their credit appears to deplete independently of their normal CLI usage.

**Watch-out: env-var precedence.** If `ANTHROPIC_API_KEY` is set in the user's environment it shadows the CLI's OAuth token. Detection probe must check both; UI must clearly indicate which is active.

## Tool routing — locked decision (2026-05-14)

**All SDK built-in tools are disabled. Rift exposes its own MCP server which the Agent SDK consumes as the sole tool source.** Reasoning: the existing safety stack (`path_guard`, mass-delete circuit breaker, Mirror gate, conflict gate) lives in Tauri command paths, NOT in raw filesystem writes — so letting the SDK's built-in Write/Edit/Bash run direct would bypass every safety rail Rift has. Wrapping reads too (even though they're observational) is the right call for consistency, unified logging, and SFTP-path awareness (Claude can directly query remote files without a pull-first dance).

Implementation: Rift Rust side spawns an internal MCP server (stdio transport), Agent SDK is configured with `allowedTools: ["mcp__rift__*"]` plus `disallowedTools: [<every built-in>]`. MCP server lives in `src-tauri/src/assistant/mcp_server.rs` and dispatches to existing Rust commands.

Forward bonus: MCP server is reusable surface, future external clients (Claude Desktop, Cursor) could consume Rift's sync engine through the same server.

## Working-directory / project scope — locked decision (2026-05-14)

α1-α2: CWD = workspace `local_root` (broad, simple, zero switching logic needed for read-only tools). α3+: shift CWD to currently-focused-resource in LocalPane with reframe on user resource-switch, plus a `rift__list_workspace_resources` tool so Claude can pivot to other resources for cross-cutting context (qb-core deps, shared libs). User-pinnable scope deferred to v0.2.57.

## Tool bridge

SDK tools we expose via the Rift MCP server, all routed through existing Rust commands:

- `rift__read_file(path)` → `local_fs::read` or `sftp::read` based on path root, supports both local and remote paths transparently
- `rift__write_file(path, content)` → routes through `path_guard` first, then `local_fs::write` (local) or `sftp::upload` (remote). Local writes still trigger watcher → drift → auto-sync.
- `rift__edit_file(path, old_string, new_string, replace_all)` → same routing as write_file, anchor-based edit semantics matching our existing Edit tool philosophy
- `rift__list_dir(path)` → existing list commands, local + remote
- `rift__grep(pattern, path, glob)` → ripgrep shell-out or walkdir+regex fallback
- `rift__bash(cmd, cwd)` → reuse `portable-pty`, spawn hidden pane, capture output, enforce 120s timeout
- `rift__sync_status()` → drift scanner snapshot (lets Claude see pending pushes/pulls before deciding)
- `rift__list_workspace_resources()` → all connected resources + their local paths (for α3+ cross-resource pivots)
- `rift__get_server_context()` → currently-connected server + working resource + open files in LocalPane

Invariant: every tool call respects the existing safety rails. Claude can't bypass Mirror gate, can't trigger mass-delete circuit-breaker, can't write outside `path_guard`-approved paths.

## Layout

Top: conversation header (model picker, effort slider, cost display toggle, clear, settings).
Middle: scrollable messages area, virtualized for long conversations.
Bottom: composer (textarea, attach-file chip, send button, Stop button visible while streaming).
Persistent activity rail pinned just above composer: always one line describing current activity (see "Visibility" below).
Optional left strip (collapsible): pinned context files, attached @-mentions, project notes.

## Message rendering

User bubbles: simple, Rift accent color.
Assistant text: markdown rendered, code blocks via Monaco's read-only mode (so syntax highlighting matches our future editor).
Thinking blocks: collapsible "💭 Thinking..." pill, expands to show streamed reasoning. Default collapsed, toggle in settings to default-expand or hide entirely.
Tool-call cards: collapsed one-liner by default ("📖 Read server.lua · 247 lines"), expand to show full input/output. Verbose-mode toggle defaults all expanded.
Proposed edits: rendered as diff cards with Apply / Reject buttons per edit. Streaming-diff or completed-diff mode behind a settings toggle (default: completed).

## SDK event reference (verified 2026-05-14)

Five core message types from the SDK stream: `SystemMessage` (lifecycle, subtype `init` first + `compact_boundary` after compaction), `AssistantMessage` (Claude turn, contains text + tool_use content blocks), `UserMessage` (tool execution results sent back), `StreamEvent` (raw API events, only when `includePartialMessages: true`), `ResultMessage` (terminal, contains final text + token usage + cost + session_id, check `subtype` for success/limit-hit).

For live token-by-token rendering enable `includePartialMessages: true` and watch for `content_block_delta` events where `delta.type === "text_delta"`. Tool calls emit `content_block_start` with a `tool_use` content block (tool name available immediately for the activity rail) then chunked `input_json_delta` events as args stream in. Thinking blocks (when enabled — **SDK default is OFF**, opt in via thinking config) emit `thinking_delta` events identical in shape to text deltas. Each thinking block carries an opaque `signature` field arriving as `signature_delta` just before `content_block_stop`; we store and forward it but never parse or display it.

If thinking is configured but display is irrelevant set `display: "omitted"` to skip thinking_delta emission entirely and reduce TTFT. We won't use this in v1 since visibility is the point.

Parallel tool calls land as multiple `tool_use` blocks in one Assistant turn — loop over all of them and send back all results in one UserMessage with matching `tool_use_id` fields. Don't process serially or you'll deadlock.

Long requests with `max_tokens > 21333` REQUIRE streaming (SDK client-side validation). We're always streaming so this is automatic but worth noting.

## Visibility surfaces (load-bearing — user's biggest pet peeve)

Live activity rail above composer, single line always updated: "💭 Thinking (2.3s, 847 tokens)" → "📖 Reading file" → "🔍 Grepping" → "✍️ Proposing edit (+12 -3)" → "✅ Done · 4.1s · $0.018".

Live token counters in header: thinking tokens + output tokens ticking as they stream.

Live cost display (optional toggle): per-message inline, session total in header, monthly cap warning.

Tool cards expand in real time: show inputs immediately, output fills in as it lands.

Network state surfaced explicitly: "⏳ API responding slowly (3.2s)" not just a spinner.

Multi-step progress indicator for agentic loops: "Step 4 of ~7" with thin progress bar.

Always-visible Stop button while streaming, clean SDK cancellation.

Assistant tab badge in tab bar: pulsing dot colored by state (blue thinking, green editing, yellow waiting-accept, red error) so visibility persists across tabs.

Verbose log panel (collapsed by default): every SDK event with timestamp + duration + raw JSON for debug. Same shape as Sync page's diagnostic log.

## Customization surfaces (settings panel under Assistant)

- Model picker: Sonnet 4.6 default, Opus 4.7, Haiku 4.5
- Effort / thinking budget: low / medium / high / xhigh
- Thinking visibility: hidden / collapsed / expanded
- Edit preview mode: streaming-diff / completed-diff
- Auto-apply trusted folders (off by default)
- Cost visibility: off / inline / session-only / full
- Notification mode: badge / desktop / sound / silent
- Verbose log default-open
- Vim mode in composer (later)
- Per-server system prompt addendum (loaded from `~/.rift/profiles/<server>.json`)
- Project context file (`CLAUDE.md` or `RIFT.md` at workspace root auto-injects)

## Theme

Inherits Rift's existing CSS variables. Same surface colors, same accent (`--accent`), same border tokens, same JetBrains-Mono for code, Inter for prose. Assistant accent and user-bubble accent can be tweaked but default to existing tokens. Chat code-blocks use Monaco with our existing dark theme tokens.

## Rift-specific UX (things JetBrains can't do)

"Sync this resource" button on Claude's response → triggers rescan + push for the folder Claude touched.

Server-aware context injection — every prompt auto-includes connected server + working resource.

Drift-aware edits — if Claude proposes editing a file with incoming remote drift, surface a warning before Apply.

Mirror gate respected — Claude can't delete files unless Mirror is on. Same rule as user.

Drag from LocalPane into composer → file attaches as context.

## Phasing

v0.2.56-α1: Empty Assistant page, OAuth flow, hello round-trip to Sonnet. No tools. Validates SDK + auth.
v0.2.56-α2: Read-only tools (read_file, list_dir, grep). Q&A about codebase, zero write risk.
v0.2.56-α3: Write tools with Apply/Reject gate, inline diff preview. Feels real.
v0.2.56-α4: Terminal tool, sync awareness, server-context injection, system-prompt addenda.
v0.2.57: Multi-conversation sidebar, history, cost dashboard, slash-commands (/clear, /compact), @-file mentions, branching.

## Don't-touch / known traps

CSP `connect-src` must allow `https://api.anthropic.com` in `tauri.conf.json`.

OAuth token in OS keychain ONLY. Never localStorage. Never `~/.rift/*.json`.

Streaming stays frontend-direct (SDK ↔ API). Do not route through Rust IPC pump — `spawn_frontend_pump` is rate-limited to 200/s and would cap streaming.

Mass-edit safety: Claude proposing >N file edits in one turn (suggest N=10) requires a single batch-confirm gate, not N individual accepts. Don't let agent loop write 50 files unsupervised.

Sync engine is the only writer pipeline. Claude's `write_file` MUST go through `auto_sync` → drift → push. No bypass commands.

Path guard frozen — Claude can't escape it. Tool bridge layer enforces.

## Failures (must-handle error cases)

API rate-limit mid-stream: SDK emits a ResultMessage with non-success subtype. Surface in activity rail as "⏳ Rate-limited, retry in Ns" with auto-retry after backoff. Conversation state preserved, user can manually resume.

OAuth-token / API-key invalid or revoked: 401 from API. Surface as red banner "Authentication failed — re-run `claude login` or update API key in settings." Don't auto-retry, that just burns budget.

Claude Code CLI absent and no API key: empty-state on Assistant page with install link + API-key input. Page is non-functional until resolved but other Rift features unaffected.

Tool-call timeout: each tool bridge enforces its own timeout (read_file 10s, write_file 30s, run_terminal 120s). On timeout return a tool_result with `is_error: true`, the agent loop decides whether to retry or abort. Don't deadlock the agent waiting on a hung SFTP op.

Mid-stream user Stop: SDK supports clean cancellation via abort signal. In-flight tool calls that have already dispatched to Rust commands run to completion (logical cancellation, not transactional) — file writes already started will land, terminal commands already spawned keep running. Document this explicitly so users know Stop ≠ rollback. UI shows "🛑 Stopped — last edit completed before cancel" so behavior is intentional not surprising.

Agent loop infinite recursion / runaway: hard cap N=25 tool calls per turn before forcing termination with error message. Configurable in settings, default 25.

Conversation file corruption: on load, malformed JSON → archive corrupt file as `<id>.json.bad`, start fresh conversation, log incident. Never crash the page.

Mass-edit safety threshold: agent loop proposing >10 file writes in one turn raises a single batch-confirm modal ("Claude wants to edit 14 files, review?"). User accepts/rejects atomically OR opens per-file diff review. Don't let unsupervised agent loops write 50 files. Threshold configurable.

Token-budget runaway: per-conversation token cap defaulting to 200k input + 50k output before auto-`/compact`. User-configurable. Hard cap at 500k to prevent runaway cost.

## Cost reporting semantics (locked α1, 2026-05-14)

The CLI's `result` envelope always emits a `total_cost_usd` field, but its meaning depends on the auth path:

- **API-key path** (`--bare` + `ANTHROPIC_API_KEY`) — the field is the actual billed amount against the user's Anthropic API account, pay-per-token. Show it; users want to see what they're spending.
- **OAuth piggyback path** (Claude Code session, `authMethod: "claude.ai"`) — the field is an **estimated-equivalent** dollar figure for how much an API-key user would have been charged. The user's subscription pool absorbs the cost; nothing is billed per-turn. Showing the figure causes confusion ("why does it say $0.06? I'm on Max?") and was the gap-3 surface in α1 smoke test.

**Rendering rule:** show the cost row only when `auth.apiKeyConfigured === true`. Hide for OAuth — the subscription handles billing, and the equivalent-cost figure isn't meaningful to the user. The June 15 2026 separate "Agent SDK credit" pool warning (see Auth section) is the right place to surface subscription-budget concerns instead, not the per-turn cost row.

For α2+, if we add a session-total cost display, apply the same rule: only show when API-key path is active. Tooltip on hover should make the distinction clear when a user toggles between paths mid-session.

## Persistence schema

`~/.rift/assistant/<conversationId>.json` shape:
```json
{
  "id": "uuid",
  "createdAt": "ISO8601",
  "updatedAt": "ISO8601",
  "model": "claude-sonnet-4-6",
  "title": "string (auto-derived from first message)",
  "messages": [
    {
      "role": "user" | "assistant" | "tool",
      "content": [...content blocks as received from SDK...],
      "timestamp": "ISO8601",
      "tokens": { "input": 0, "output": 0, "thinking": 0 },
      "cost": 0.0
    }
  ],
  "totalCost": 0.0,
  "totalTokens": { "input": 0, "output": 0, "thinking": 0 },
  "pinnedContext": ["path/to/file.lua"],
  "systemPromptAddendum": "string or null"
}
```

Schema-versioned via a top-level `schemaVersion: 1` field — bump on any breaking change, write a migration step.

## Open decisions (defer to implementation)

Conversation list UI shape (sidebar vs dropdown vs separate sub-tab).
Exact composer behavior (Enter to send vs Shift+Enter, multiline rules).
Whether to expose raw API-key login alongside OAuth (probably no for v1).
Whether `RIFT.md` or `CLAUDE.md` is the canonical project-context filename.

## Files to be created (estimate)

`src/routes/assistant/+page.svelte` (or new tab in AppShell only, no route — match existing pattern).
`src/lib/components/assistant/AssistantPage.svelte`
`src/lib/components/assistant/MessageList.svelte`
`src/lib/components/assistant/Message.svelte`
`src/lib/components/assistant/Composer.svelte`
`src/lib/components/assistant/ActivityRail.svelte`
`src/lib/components/assistant/ToolCallCard.svelte`
`src/lib/components/assistant/EditDiffCard.svelte`
`src/lib/components/assistant/SignInPanel.svelte`
`src/lib/state/assistant.svelte.ts`
`src/lib/sdk/agent-client.ts` (SDK wiring + tool bridge)
`src-tauri/src/assistant/mod.rs` (Rust-side tool command surface, if any)
Settings additions to existing Settings page (`AssistantSettings.svelte` subsection).

## Verification gates

Per phase: `svelte-check` clean, `cargo check` clean, manual round-trip with each phase's new tool.
Pre-ship: OAuth flow tested on fresh install, conversation persistence verified across restart, Stop button cancels mid-stream cleanly, write-tool circuit-breakers tested (mass-delete attempt blocked, Mirror gate respected).

## Handoff note for next session

Read this doc first. Start with v0.2.56-α1 only — auth + empty page + hello round-trip. Do NOT scaffold all phases at once. Validate the SDK integration shape before building UI scale. First concrete task: install Claude Agent SDK TS package, wire OAuth login button, get one streamed "hello" response into a basic message list. Everything else is gated behind that working.

// M0 (per docs/design/assistant-svelte-split.md) — pure type defs lifted out of
// `src/lib/state/assistant.svelte.ts` so the (still very large) host module
// has less to lex. Zero runtime; types + the `MAX_PANES` const only.
//
// Public API frozen: all type defs that were re-exported from
// `assistant.svelte.ts` continue to be re-exported there — external imports
// like `import type { Block, ChatMessage } from "$lib/state/assistant.svelte"`
// keep working unchanged.

export type WorkspaceState = {
  current: string | null;
  recent: string[];
};

/** A user-defined project — a named alias over a workspace folder plus
 *  per-project file-pattern config that scopes the assistant's workspace tools
 *  (read_file/list_dir/grep) + the @-mention picker. Mirrors `ProjectDto` in
 *  `src-tauri/src/assistant/projects.rs`. `id` is a `crypto.randomUUID()`. */
export type Project = {
  id: string;
  name: string;
  root: string;
  /** Glob allowlist. Empty = include everything (minus exclude + SKIP_DIRS). */
  include: string[];
  /** Glob blocklist, on top of the always-on SKIP_DIRS baseline. */
  exclude: string[];
  createdAt: number;
};

/** One detected Claude Code CLI install. A machine can have several at once
 *  (npm-global + native), which silently drift to different versions — Rift
 *  enumerates all, runs on the newest, and updates every one. */
export type ClaudeInstall = {
  path: string;
  method: string; // "npm" | "native" | "unknown"
  version: string | null;
  onPath: boolean;
  active: boolean;
};

export type AuthStatus = {
  cliPresent: boolean;
  cliVersion: string | null;
  loggedIn: boolean;
  authMethod: string | null;
  apiProvider: string | null;
  email: string | null;
  subscriptionType: string | null;
  apiKeyConfigured: boolean;
  /** A system ANTHROPIC_API_KEY env var exists but Rift deliberately ignores
   *  it (stripped from every CLI spawn). Surfaced so the UI can warn. */
  envApiKeyPresent: boolean;
  /** How the resolved `claude` binary was installed — "npm" | "native" |
   *  "unknown". Drives the correct in-app update command. Null when no CLI. */
  installMethod: string | null;
  pill: "green" | "yellow" | "red";
  summary: string;
  /** Every Claude CLI install detected on this machine. The `active` one drives
   *  cliVersion/installMethod; the rest are shown + updated alongside it. */
  installs: ClaudeInstall[];
};

export type ToolBlock = {
  type: "tool";
  id: string;
  name: string;
  input: Record<string, unknown>;
  result: string | null;
  isError: boolean;
  status: "pending" | "done" | "error";
  // S124: wall-clock start (ms epoch) on tool_use, end (ms epoch) on
  // tool_result. Lets the chip render an inline duration badge when the
  // call was slow (>1s). Optional — legacy records omit both fields.
  startedAt?: number;
  durationMs?: number;
  // S127: true while the tool_use input is still streaming in via
  // input_json_delta (live-forming block) — `input` holds only the caption
  // fields extracted so far. Cleared when the complete input lands
  // (content_block_stop parse or the assistant envelope); never true on
  // persisted records (terminal sweeps clear it).
  inputPartial?: boolean;
};

export type TextBlock = {
  type: "text";
  text: string;
};

export type ThinkingBlock = {
  type: "thinking";
  // Plaintext reasoning if the API streamed it. Often empty in -p mode —
  // Anthropic encrypts thinking content and only emits the signature, in
  // which case we show duration + a "reasoning recorded" hint instead.
  text: string;
  // Encrypted signature blob received (presence flag — we don't render it).
  hasSignature: boolean;
  // Wall-clock start (ms epoch) — set on content_block_start. Used by the
  // bubble to render a live elapsed counter during active reasoning.
  startedAt: number;
  // Wall-clock duration of the reasoning step. Null while still active.
  durationMs: number | null;
  status: "active" | "done";
};

/** Compaction Phase C: synthetic block that marks the boundary where a
 *  CLI session was retired in favor of a summary. */
export type BoundaryBlock = {
  type: "boundary";
  summary: string;
  at: number;
  archivedCount: number;
  costUsd: number;
  summaryModel: string;
  // S124: true while the summarize call is in-flight; flipped to false on
  // the final 'done' event.
  streaming?: boolean;
  // Phase E1: ctx% snapshot at the moment the compact fired (pre) and the
  // estimated ctx% the new session starts at (post = summary tokens / window).
  ctxPctBefore?: number;
  ctxPctEstAfter?: number;
  // CLI-emitted compaction (Claude Code's own `compact_boundary` event). The
  // CLI keeps the summary internal, so source==="cli" boundaries carry no
  // summary/archivedCount/cost — they render a lean info pill instead of the
  // legacy expandable summary path.
  source?: "cli";
  trigger?: "auto" | "manual";
  preTokens?: number;
  postTokens?: number;
};

/** User-attached image — pasted/dropped into the composer, persisted on the
 *  outgoing user message. */
export type ImageBlock = {
  type: "image";
  mime: string;
  dataBase64: string;
  sizeBytes: number;
};

/** Inline transcript marker for a mid-chat model switch — appended by send()
 *  when a turn goes out under a different model than the chat was running on.
 *  The backend honors the switch and re-pins the session (turn.rs). */
export type ModelSwitchBlock = {
  type: "modelSwitch";
  /** ModelSel (or legacy full id) the chat ran on before the switch. */
  from: string;
  /** ModelSel the chat runs on from this turn forward. */
  to: string;
  at: number;
};

export type Block = TextBlock | ToolBlock | ThinkingBlock | BoundaryBlock | ImageBlock | ModelSwitchBlock;

/** A queued outbound message (sent while the tab was already streaming).
 *  `images`/`textFiles` snapshot the composer attachments at enqueue time so
 *  the message carries its attachments when it later drains — the composer
 *  arrays are cleared immediately after enqueue, so without the snapshot the
 *  attachments were silently dropped. The queue is in-memory, in-session state
 *  (never serialized to disk — neither persistTabs nor the conversation record
 *  includes it), so holding base64 images here for the session is safe. */
export type QueueItem = {
  id: string;
  text: string;
  images?: { id: string; mime: string; dataBase64: string; sizeBytes: number }[];
  textFiles?: { id: string; name: string; text: string; sizeBytes: number; truncated: boolean }[];
};

export type ChatMessage = {
  id: string;
  role: "user" | "assistant" | "system";
  blocks: Block[];
  costUsd?: number | null;
  model?: string | null;
  /** Creation time (ms epoch) — stamped at send/turn-start since 2026-07-02.
   *  Absent on older persisted convos; UI hides the hover timestamp then. */
  ts?: number | null;
  /** Wall-clock of the whole turn (ms), from the CLI result frame's
   *  duration_ms — spawn to result, tool waits included. Drives "Worked for
   *  Ns"; the old summed thinking+tool secs under-reported badly (a pure-text
   *  turn read "Worked for 0s"). Absent on pre-field convos → legacy sum. */
  turnDurationMs?: number | null;
  // Terminal stop reason when noteworthy: "max_tokens" (response truncated at
  // the output cap) or "refusal" (model declined). Null/absent for normal
  // completions. Drives the truncation/refusal notice in MessageBubble.
  stopReason?: "max_tokens" | "refusal" | null;
  // Permission mode the turn actually RAN with (snapshotted at send time) —
  // TurnSummary's "Applied automatically"/bypass badge reads this so a later
  // mode switch can't retroactively relabel history. Absent on pre-field convos.
  permissionMode?: PermissionMode | null;
};

export type ConversationMeta = {
  id: string;
  title: string;
  model: string;
  messageCount: number;
  createdAt: number;
  updatedAt: number;
  /** Timestamp of the last real user/assistant turn. Only advances when a
   *  message is sent or an assistant result lands — NOT on open/switch/auto-save.
   *  Used by the sidebar to sort and bucket conversations. Falls back to
   *  updatedAt on legacy records that predate this field. */
  lastActivityAt?: number;
  /** Σ of per-turn costs across the transcript. 0 for convos predating cost capture. */
  costUsd: number;
  /** One-line preview of the newest text message (ui-audit #6). */
  lastSnippet?: string;
  /** Per-project scope: the workspace folder this convo belongs to, so the
   *  sidebar can show only the open project's chats. Backfilled from the
   *  session-cwd sidecar for legacy convos; null/absent = unfiled. */
  workspaceRoot?: string | null;
};

export type ConversationRecord = {
  id: string;
  title: string;
  model: string;
  createdAt: number;
  updatedAt: number;
  messages: ChatMessage[];
  // CLI session UUID (--session-id / --resume target).
  cliSessionId?: string;
  /** Last real turn timestamp — drives stable sidebar order (see ConversationMeta).
   *  Optional: legacy records predating it fall back to updatedAt on load. */
  lastActivityAt?: number;
  /** Final turn's ctx usage — hydrates the ctx meter on restore (ISSUES #32). */
  lastTurnUsage?: { input: number; output: number; cacheRead: number; cacheCreate: number } | null;
  /** Per-project scope: workspace folder active when this convo's turns run.
   *  Stamped on save so the sidebar can filter to the open project. */
  workspaceRoot?: string | null;
};

// Minimal stream-json envelope shape we care about.
export type ContentBlock =
  | { type: "text"; text: string }
  | { type: "thinking"; thinking?: string; signature?: string }
  | { type: "tool_use"; id: string; name: string; input?: Record<string, unknown> }
  | { type: "tool_result"; tool_use_id: string; content?: unknown; is_error?: boolean };

export type StreamDelta = {
  type?: string;
  text?: string;
  thinking?: string;
  signature?: string;
  // input_json_delta: one chunk of a tool_use input's JSON as it streams.
  partial_json?: string;
  // message_delta carries the terminal stop reason for the current assistant
  // message — `max_tokens` (output truncated) / `refusal` are surfaced to the
  // user; `end_turn`/`tool_use`/`stop_sequence` are normal and ignored.
  stop_reason?: string | null;
};

export type StreamEvent = {
  type?: string;
  index?: number;
  content_block?: ContentBlock;
  delta?: StreamDelta;
};

export type StreamEnvelope =
  | { type: "system"; subtype?: string; [k: string]: unknown }
  | { type: "stream_event"; event?: StreamEvent; [k: string]: unknown }
  // `parent_tool_use_id`: set to the spawning Task/Agent tool_use id when this
  // envelope is a NESTED sub-agent frame (the CLI multiplexes sub-agent output
  // into the same stream). Null/absent for top-level turn frames. Drives the
  // sub-agent live dock — see applySubAgentFrame in streaming.ts.
  | { type: "assistant"; message: { content: ContentBlock[]; usage?: Record<string, unknown> }; parent_tool_use_id?: string | null }
  | { type: "user"; message: { content: ContentBlock[] }; parent_tool_use_id?: string | null }
  | { type: "result"; subtype?: string; result?: string; total_cost_usd?: number; [k: string]: unknown }
  // `--prompt-suggestions` (CLI 2.1.201+): one predicted next user prompt,
  // emitted after the turn's result. Shape confirmed against the 2.1.201 exe:
  // { type, suggestion, uuid, session_id }.
  | { type: "prompt_suggestion"; suggestion?: unknown; [k: string]: unknown };

/** Extended-thinking tier, mirroring the CLI's effort ladder: none→low ·
 *  smart→medium (interactive default) · deep→high · ultra = ultracode: xhigh
 *  effort + autonomous dynamic-workflow orchestration via the CLI's `ultracode`
 *  settings key (see assistant_send in turn.rs). Haiku rejects the effort flag
 *  server-side, so it gets none. The legacy "quick" tier (retired 2026-07-03,
 *  also sent medium) is coerced to "smart" at every read boundary — loadEffort
 *  (FE), normalize_effort_tier (BE), normalizeApply (AI Health). */
export type ThinkingEffort = "none" | "smart" | "deep" | "ultra";

/** Model selection — stored value IS the string handed to the CLI's `--model`,
 *  so it flows through `assistant_send` untouched. `opus` is the short alias
 *  that always resolves to the newest Opus (currently 4.8) with the 1M-ctx
 *  beta; `claude-opus-4-7` pins the prior Opus. `sonnet`/`haiku` stay aliases.
 *  Must satisfy the Rust `is_valid_model_name` validator (no brackets). */
export type ModelSel = "sonnet" | "opus" | "claude-opus-4-7" | "haiku" | "claude-fable-5";

/** Visual family for the per-model aurora hue (sonnet=blue, opus=purple,
 *  haiku=teal). Both Opus versions collapse to "opus". */
export type ModelFamily = "sonnet" | "opus" | "haiku";

/** Permission mode handed to the CLI's `--permission-mode`. Mirrors the
 *  modes Claude Code exposes. Must stay in sync with the validator in
 *  src-tauri/src/assistant/mod.rs. `bypassPermissions` = run everything
 *  (Rift's historical default); the prompting modes (`default`, `acceptEdits`,
 *  `plan`) need the approval surface from Piece 2 to be fully functional. */
export type PermissionMode = "default" | "acceptEdits" | "plan" | "auto" | "bypassPermissions";

/** Subscription plan the logged-in user is on. Drives the context-window CAP
 *  applied to the model's native window (see `planContextCap` / `ctxWindowForModelId`
 *  in helpers.ts). Anthropic exposes NO programmatic plan/entitlement signal for
 *  OAuth users (no API field, no CLI JSON), so this is a USER-SET preference, not
 *  detected. Reality (2026-06, support.claude.com): Free is hard-capped at 200K
 *  even on Opus/Sonnet; Pro gets the full 1M in Claude Code only with usage
 *  credits enabled (else quota-gated); Max/Team/Enterprise get 1M automatically.
 *  Default is `max` (1M) because that's correct for the majority of paying users;
 *  Free / uncredited-Pro users set their tier once. */
export type RiftPlan = "free" | "pro" | "max";

/** Assistant trust level gating the local git tools. Must stay in sync with
 *  `is_valid_trust_level` in src-tauri/src/assistant/config.rs.
 *  `readonly` → git status/diff/log; `standard` → adds commit/pull/push. */
export type TrustLevel = "readonly" | "standard";

/** A suggestion the CLI attaches to a `can_use_tool` ask — e.g.
 *  `{ type: "setMode", mode: "acceptEdits", destination: "session" }`, which
 *  drives the "Allow for the rest of this session" affordance. */
export type PermissionSuggestion = {
  type: string;
  mode?: string;
  destination?: string;
};

/** A pending `can_use_tool` permission ask, keyed by the tool's `tool_use_id`
 *  so the streamed tool chip can render Allow / Deny inline. */
export type PermissionPromptInfo = {
  requestId: string;
  toolName: string;
  suggestions: PermissionSuggestion[];
};

/** Telemetry record for a single Claude turn. */
export type TurnRecord = {
  // Identity
  ts: number;
  convoId: string;
  cliSessionId: string;
  isFirstTurn: boolean;
  model: ModelSel;
  effort: ThinkingEffort;
  /** Actual `--effort` flag the CLI is invoked with (mirrors mod.rs mapping). */
  effortFlag: "low" | "medium" | "high" | "xhigh" | null;
  // Input
  promptLen: number;
  promptPreview: string;
  attachmentsCount: number;
  attachmentsBytes: number;
  // Usage (filled progressively)
  envelopeUsage: { input: number; output: number; cacheRead: number; cacheCreate: number } | null;
  resultUsage: { input: number; output: number; cacheRead: number; cacheCreate: number } | null;
  modelId: string | null;
  costUsd: number | null;
  // Stream stats
  deltaCount: number;
  streamEventCount: number;
  assistantEnvCount: number;
  maxStreamGapMs: number;
  toolUses: {
    name: string;
    id: string;
    startedAt: number;
    completedAt: number | null;
    durationMs: number | null;
    isError: boolean | null;
    inputPreview: string | null;
  }[];
  thinkingCount: number;
  thinkingTotalMs: number;
  thinkingBlocks: { startedAt: number; durationMs: number; charCount: number; hasSignature: boolean }[];
  envelopeFallback: boolean;
  blankTurn: boolean;
  // Timing
  firstPaintAt: number | null;
  // thinkingTotalMs as of firstPaintAt (snapshot). deadWait (healthAlerts) must
  // subtract only PRE-paint thinking, not the full-turn cumulative — else a long
  // post-paint thinking round drives deadWait negative and masks a real slow
  // start. Null until first paint (mirrors firstPaintAt).
  thinkingMsBeforeFirstPaint: number | null;
  doneAt: number | null;
  endKind: "success" | "user-stop" | "session-lost" | "error" | null;
  errorMsg?: string;
};

/** Per-pane reference into the openTabs list. v2 split UI: `panes` is always
 *  an array of length ≥1. `null` tabId = empty pane. */
export type PaneState = { tabId: string | null };

/** Hard cap on horizontal panes. 4 × min-width 320px = 1280px. */
export const MAX_PANES = 4;

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

/** A mid-turn steer: the user's interjection injected into the RUNNING turn's
 *  stdin (assistant_steer). Rendered inline in the assistant bubble at the point
 *  it landed so the steer is *visible* in the transcript, not just a toast. */
export type SteerBlock = {
  type: "steer";
  text: string;
  at: number;
};

export type Block = TextBlock | ToolBlock | ThinkingBlock | BoundaryBlock | ImageBlock | SteerBlock;

export type ChatMessage = {
  id: string;
  role: "user" | "assistant" | "system";
  blocks: Block[];
  costUsd?: number | null;
  model?: string | null;
  // Terminal stop reason when noteworthy: "max_tokens" (response truncated at
  // the output cap) or "refusal" (model declined). Null/absent for normal
  // completions. Drives the truncation/refusal notice in MessageBubble.
  stopReason?: "max_tokens" | "refusal" | null;
};

export type ConversationMeta = {
  id: string;
  title: string;
  model: string;
  messageCount: number;
  createdAt: number;
  updatedAt: number;
  /** Σ of per-turn costs across the transcript. 0 for convos predating cost capture. */
  costUsd: number;
  /** One-line preview of the newest text message (ui-audit #6). */
  lastSnippet?: string;
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
  /** Final turn's ctx usage — hydrates the ctx meter on restore (ISSUES #32). */
  lastTurnUsage?: { input: number; output: number; cacheRead: number; cacheCreate: number } | null;
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
  | { type: "result"; subtype?: string; result?: string; total_cost_usd?: number; [k: string]: unknown };

/** Extended-thinking tier, mirroring the CLI's effort ladder: none→low ·
 *  quick→medium · smart→high (API default) · deep→xhigh (Claude Code's agentic
 *  default) · ultra = ultracode: xhigh effort + autonomous dynamic-workflow
 *  orchestration via the CLI's `ultracode` settings key (see assistant_send in
 *  turn.rs). Haiku rejects the effort flag server-side, so it gets none. */
export type ThinkingEffort = "none" | "quick" | "smart" | "deep" | "ultra";

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

/** Assistant trust level gating the local git tools (and, later, RCON). Must
 *  stay in sync with `is_valid_trust_level` in src-tauri/src/assistant/mod.rs.
 *  `readonly` → git status/diff/log; `standard` → adds commit/pull/push;
 *  `full` → reserved for RCON raw passthrough (phase 2). */
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
  doneAt: number | null;
  endKind: "success" | "user-stop" | "session-lost" | "error" | null;
  errorMsg?: string;
};

/** Per-pane reference into the openTabs list. v2 split UI: `panes` is always
 *  an array of length ≥1. `null` tabId = empty pane. */
export type PaneState = { tabId: string | null };

/** Hard cap on horizontal panes. 4 × min-width 320px = 1280px. */
export const MAX_PANES = 4;

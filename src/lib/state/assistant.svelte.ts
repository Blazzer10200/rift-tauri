// Assistant page state.
//
// Spawns the user's installed `claude` CLI through Rust commands; the CLI
// streams NDJSON which the backend forwards verbatim on `assistant://stream`.
// Wires Rift's MCP server (read_file / list_dir / grep) so assistant turns
// can interleave text, tool calls, and TodoWrite-driven task lists.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

export type WorkspaceState = {
  current: string | null;
  recent: string[];
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
  pill: "green" | "yellow" | "red";
  summary: string;
};

export type ToolBlock = {
  type: "tool";
  id: string;
  name: string;
  input: Record<string, unknown>;
  result: string | null;
  isError: boolean;
  status: "pending" | "done" | "error";
};

export type TextBlock = {
  type: "text";
  text: string;
};

export type Block = TextBlock | ToolBlock;

export type ChatMessage = {
  id: string;
  role: "user" | "assistant";
  blocks: Block[];
};

export type ConversationMeta = {
  id: string;
  title: string;
  model: string;
  messageCount: number;
  createdAt: number;
  updatedAt: number;
};

type ConversationRecord = {
  id: string;
  title: string;
  model: string;
  createdAt: number;
  updatedAt: number;
  messages: ChatMessage[];
};

// Minimal stream-json envelope shape we care about.
type ContentBlock =
  | { type: "text"; text: string }
  | { type: "tool_use"; id: string; name: string; input?: Record<string, unknown> }
  | { type: "tool_result"; tool_use_id: string; content?: unknown; is_error?: boolean };

type StreamEnvelope =
  | { type: "system"; subtype?: string; [k: string]: unknown }
  | { type: "stream_event"; event?: { type?: string; delta?: { type?: string; text?: string } }; [k: string]: unknown }
  | { type: "assistant"; message: { content: ContentBlock[] } }
  | { type: "user"; message: { content: ContentBlock[] } }
  | { type: "result"; subtype?: string; result?: string; total_cost_usd?: number; [k: string]: unknown };

const MODEL_KEY = "rift.assistant.model";

function loadModel(): "sonnet" | "opus" | "haiku" {
  try {
    const v = typeof localStorage !== "undefined" ? localStorage.getItem(MODEL_KEY) : null;
    if (v === "sonnet" || v === "opus" || v === "haiku") return v;
  } catch {
    /* SSR or storage disabled */
  }
  return "sonnet";
}

function saveModel(v: "sonnet" | "opus" | "haiku") {
  try {
    if (typeof localStorage !== "undefined") localStorage.setItem(MODEL_KEY, v);
  } catch {
    /* storage disabled */
  }
}

function flattenToolResult(content: unknown): string {
  if (typeof content === "string") return content;
  if (Array.isArray(content)) {
    return content
      .map((c) => (typeof c === "object" && c && "text" in c ? String((c as { text: unknown }).text ?? "") : ""))
      .join("");
  }
  return "";
}

class AssistantStore {
  auth = $state<AuthStatus | null>(null);
  authChecking = $state(false);
  authError = $state<string | null>(null);

  messages = $state<ChatMessage[]>([]);
  streaming = $state(false);
  lastError = $state<string | null>(null);
  // Informational system notice (slash-command output, /help text, etc.).
  // Rendered as a dismissible info banner separate from error styling.
  lastNotice = $state<string | null>(null);
  totalCostUsd = $state<number | null>(null);
  // Ring buffer of user prompts, newest last. Powers /retry and Up-arrow
  // recall in the composer. Capped at 50 entries.
  promptHistory = $state<string[]>([]);

  apiKey = $state<string | null>(null);

  // The Assistant's open project folder + recent-folder list. Decoupled from
  // Sync's server folders; populated by `assistant_get_workspace` on init and
  // updated whenever the user opens, switches, or clears a folder. Empty
  // `current` falls back to AutoSync folders on the Rust side.
  workspace = $state<WorkspaceState>({ current: null, recent: [] });

  composerDraft = $state("");
  // Outbound message queue. send() appends here if a turn is already
  // streaming; onDone() pops the next one. UI surfaces queued items as
  // pills above the composer with an X to remove.
  queue = $state<{ id: string; text: string }[]>([]);
  // User's chosen model — flipped by /model slash command. Carried through
  // to assistant_send so the CLI uses sonnet/opus/haiku per their choice.
  // Initialized from localStorage so the choice survives reloads.
  model = $state<"sonnet" | "opus" | "haiku">(loadModel());
  ui = $state({ dockOpen: false, tasksUpdatedAt: 0, historyOpen: false });

  // Conversation history.
  //   - `currentConvoId` is null before the first message is sent; first
  //     `send()` assigns a fresh UUID and persists from there.
  //   - `conversations` is the metadata cache for the drawer; refreshed
  //     after every save/delete/rename.
  //   - `createdAt` is set when the convo starts, kept stable across saves.
  currentConvoId = $state<string | null>(null);
  conversations = $state<ConversationMeta[]>([]);
  private convoCreatedAt: number | null = null;
  private convoTitle: string | null = null;
  private saveTimer: ReturnType<typeof setTimeout> | null = null;
  tasks = $state<{ id: string; content: string; status: "pending" | "in_progress" | "completed" }[]>([]);

  // Activity tracking — currently-running tool label and turn start timestamp
  // for the elapsed timer. Reset on new send(). Per-tool counts come from
  // enumerating message.blocks directly in the dock.
  activity = $state<{
    currentLabel: string | null;
    turnStartedAt: number | null;
  }>({ currentLabel: null, turnStartedAt: null });

  private unlistens: UnlistenFn[] = [];
  private streamingMsgId: string | null = null;
  private seenToolUseIds = new Set<string>();
  private dockAutoOpenedThisConvo = false;

  setModel(v: "sonnet" | "opus" | "haiku") {
    this.model = v;
    saveModel(v);
  }

  clearQueue() {
    this.queue = [];
  }

  async init() {
    if (this.unlistens.length > 0) return;
    this.unlistens.push(
      await listen<string>("assistant://stream", (e) => this.onStream(e.payload)),
      await listen<{ exit_code: number }>("assistant://done", () => this.onDone()),
      await listen<string>("assistant://error", (e) => this.onError(e.payload)),
    );
    await this.refreshAuth();
    try {
      this.apiKey = await invoke<string | null>("assistant_get_api_key");
    } catch (e) {
      console.warn("assistant_get_api_key failed", e);
    }
    await this.refreshConversations();
    await this.refreshWorkspace();
  }

  async refreshWorkspace() {
    try {
      this.workspace = await invoke<WorkspaceState>("assistant_get_workspace");
    } catch (e) {
      console.warn("assistant_get_workspace failed", e);
    }
  }

  /** Native folder picker → set as active root. Returns false if user cancelled. */
  async pickFolder(): Promise<boolean> {
    try {
      const result = await openDialog({ directory: true, multiple: false });
      const path = typeof result === "string" ? result : null;
      if (!path) return false;
      await this.setRoot(path);
      return true;
    } catch (e) {
      this.lastError = `Open folder failed: ${String(e)}`;
      return false;
    }
  }

  async setRoot(path: string) {
    try {
      this.workspace = await invoke<WorkspaceState>("assistant_set_root", { path });
      this.lastNotice = `Workspace: ${path}`;
    } catch (e) {
      this.lastError = `Set workspace failed: ${String(e)}`;
    }
  }

  async clearRoot() {
    try {
      this.workspace = await invoke<WorkspaceState>("assistant_clear_root");
    } catch (e) {
      console.warn("assistant_clear_root failed", e);
    }
  }

  async removeRecentRoot(path: string) {
    try {
      this.workspace = await invoke<WorkspaceState>("assistant_remove_recent_root", { path });
    } catch (e) {
      console.warn("assistant_remove_recent_root failed", e);
    }
  }

  async refreshConversations() {
    try {
      this.conversations = await invoke<ConversationMeta[]>("assistant_list_conversations");
    } catch (e) {
      console.warn("assistant_list_conversations failed", e);
    }
  }

  /** Derive a human-friendly title from the first user message. */
  private deriveTitle(): string {
    const first = this.messages.find((m) => m.role === "user");
    if (!first) return "New conversation";
    const text = first.blocks
      .map((b) => (b.type === "text" ? b.text : ""))
      .join("")
      .trim()
      .replace(/\s+/g, " ");
    return text.length > 60 ? text.slice(0, 60) + "…" : text || "New conversation";
  }

  /** Persist the current conversation. Debounced — callers can fire freely;
   *  only one disk write per ~700ms. Set `flush=true` to write immediately. */
  private scheduleSave(flush = false) {
    if (!this.currentConvoId || this.messages.length === 0) return;
    if (this.saveTimer) {
      clearTimeout(this.saveTimer);
      this.saveTimer = null;
    }
    const doSave = async () => {
      this.saveTimer = null;
      if (!this.currentConvoId) return;
      const record: ConversationRecord = {
        id: this.currentConvoId,
        title: this.convoTitle ?? this.deriveTitle(),
        model: this.model,
        createdAt: this.convoCreatedAt ?? Date.now(),
        updatedAt: Date.now(),
        messages: this.messages,
      };
      this.convoTitle = record.title;
      this.convoCreatedAt = record.createdAt;
      try {
        await invoke("assistant_save_conversation", { convo: record });
        await this.refreshConversations();
      } catch (e) {
        console.warn("assistant_save_conversation failed", e);
      }
    };
    if (flush) void doSave();
    else this.saveTimer = setTimeout(doSave, 700);
  }

  /** Start a fresh conversation. Flushes the current one first so nothing
   *  is lost when the user clicks `+ New`. */
  async newConversation() {
    if (this.streaming) await this.stop();
    if (this.messages.length > 0) this.scheduleSave(true);
    this.messages = [];
    this.tasks = [];
    this.queue = [];
    this.lastError = null;
    this.lastNotice = null;
    this.totalCostUsd = null;
    this.promptHistory = [];
    this.dockAutoOpenedThisConvo = false;
    this.ui.dockOpen = false;
    this.currentConvoId = null;
    this.convoCreatedAt = null;
    this.convoTitle = null;
  }

  async loadConversation(id: string) {
    if (this.streaming) await this.stop();
    if (this.messages.length > 0 && this.currentConvoId && this.currentConvoId !== id) {
      this.scheduleSave(true);
    }
    try {
      const convo = await invoke<ConversationRecord>("assistant_load_conversation", { id });
      this.messages = convo.messages ?? [];
      this.currentConvoId = convo.id;
      this.convoCreatedAt = convo.createdAt;
      this.convoTitle = convo.title;
      this.tasks = [];
      this.queue = [];
      this.lastError = null;
      this.lastNotice = null;
      this.totalCostUsd = null;
      this.promptHistory = this.messages
        .filter((m) => m.role === "user")
        .map((m) => m.blocks.map((b) => (b.type === "text" ? b.text : "")).join("").trim())
        .filter((s) => s.length > 0)
        .slice(-50);
      this.dockAutoOpenedThisConvo = false;
      this.ui.historyOpen = false;
      if (convo.model === "sonnet" || convo.model === "opus" || convo.model === "haiku") {
        this.setModel(convo.model);
      }
    } catch (e) {
      this.lastError = `Failed to load conversation: ${String(e)}`;
    }
  }

  async deleteConversation(id: string) {
    try {
      await invoke("assistant_delete_conversation", { id });
      if (this.currentConvoId === id) {
        this.currentConvoId = null;
        this.convoCreatedAt = null;
        this.convoTitle = null;
        this.messages = [];
      }
      await this.refreshConversations();
    } catch (e) {
      this.lastError = `Failed to delete conversation: ${String(e)}`;
    }
  }

  async renameConversation(id: string, title: string) {
    const trimmed = title.trim();
    if (!trimmed) return;
    try {
      const convo = await invoke<ConversationRecord>("assistant_load_conversation", { id });
      convo.title = trimmed.slice(0, 120);
      convo.updatedAt = Date.now();
      await invoke("assistant_save_conversation", { convo });
      if (this.currentConvoId === id) this.convoTitle = convo.title;
      await this.refreshConversations();
    } catch (e) {
      this.lastError = `Failed to rename conversation: ${String(e)}`;
    }
  }

  async refreshAuth() {
    this.authChecking = true;
    this.authError = null;
    try {
      this.auth = await invoke<AuthStatus>("assistant_auth_probe");
    } catch (e) {
      this.authError = String(e);
      this.auth = null;
    } finally {
      this.authChecking = false;
    }
  }

  async setApiKey(key: string | null) {
    const v = key && key.trim().length > 0 ? key.trim() : null;
    await invoke("assistant_set_api_key", { apiKey: v });
    this.apiKey = v;
    await this.refreshAuth();
  }

  /** Derive plain-text content of a message for history replay. */
  private messageText(m: ChatMessage): string {
    return m.blocks
      .map((b) => (b.type === "text" ? b.text : ""))
      .join("")
      .trim();
  }

  async send(prompt: string) {
    const trimmed = prompt.trim();
    if (!trimmed) return;
    // Try-handle as a slash command first; if it matched, we're done.
    if (trimmed.startsWith("/") && this.runSlash(trimmed)) return;
    // Already streaming → queue instead of dropping.
    if (this.streaming) {
      this.queue = [...this.queue, { id: crypto.randomUUID(), text: trimmed }];
      return;
    }
    // Snapshot prior history BEFORE we mutate this.messages — backend replays
    // these as Human:/Assistant: chain so Claude sees the conversation context.
    // Tool blocks aren't included in history replay; the CLI handles its own
    // tool turns in-session and our replay only carries final text per role.
    const history = this.messages
      .map((m) => ({ role: m.role, text: this.messageText(m) }))
      .filter((m) => m.text.length > 0);
    // Assign a conversation id on first send; persist from there.
    if (!this.currentConvoId) {
      this.currentConvoId = crypto.randomUUID();
      this.convoCreatedAt = Date.now();
      this.convoTitle = null;
    }
    this.streaming = true;
    this.lastError = null;
    this.lastNotice = null;
    this.seenToolUseIds.clear();
    this.activity = { currentLabel: null, turnStartedAt: Date.now() };
    // Track for /retry and Up-arrow recall. De-dupe consecutive identicals.
    if (this.promptHistory[this.promptHistory.length - 1] !== trimmed) {
      this.promptHistory = [...this.promptHistory, trimmed].slice(-50);
    }
    this.messages = [
      ...this.messages,
      { id: crypto.randomUUID(), role: "user", blocks: [{ type: "text", text: trimmed }] },
    ];
    const asst: ChatMessage = { id: crypto.randomUUID(), role: "assistant", blocks: [] };
    this.messages = [...this.messages, asst];
    this.streamingMsgId = asst.id;
    try {
      await invoke("assistant_send", { prompt: trimmed, history, model: this.model });
    } catch (e) {
      this.onError(String(e));
    }
  }

  private mutateStreaming(fn: (m: ChatMessage) => ChatMessage) {
    if (!this.streamingMsgId) return;
    this.messages = this.messages.map((m) => (m.id === this.streamingMsgId ? fn(m) : m));
  }

  private appendText(chunk: string) {
    if (!chunk) return;
    this.mutateStreaming((m) => {
      const blocks = m.blocks.slice();
      const last = blocks[blocks.length - 1];
      if (last && last.type === "text") {
        blocks[blocks.length - 1] = { type: "text", text: last.text + chunk };
      } else {
        blocks.push({ type: "text", text: chunk });
      }
      return { ...m, blocks };
    });
  }

  /** User-driven pin from a chat checklist into the Tasks dock.
   *  Items arrive as plain text + checked flag from rendered HTML. */
  pinTasksFromChecklist(items: Array<{ content: string; checked: boolean }>) {
    if (items.length === 0) return;
    this.tasks = items.map((t, i) => ({
      id: `pin-${Date.now()}-${i}`,
      content: t.content,
      status: t.checked ? "completed" : "pending",
    }));
    this.ui.tasksUpdatedAt = Date.now();
    this.ui.dockOpen = true;
    this.dockAutoOpenedThisConvo = true;
  }

  private applyTodoWrite(input: Record<string, unknown> | undefined) {
    const raw = (input?.todos ?? []) as Array<{ content?: string; status?: string }>;
    const next = raw
      .filter((t) => typeof t?.content === "string")
      .map((t, i) => ({
        id: `todo-${i}-${t.content!.slice(0, 24)}`,
        content: t.content!,
        status: (t.status === "in_progress" || t.status === "completed" ? t.status : "pending") as
          | "pending"
          | "in_progress"
          | "completed",
      }));
    this.tasks = next;
    this.ui.tasksUpdatedAt = Date.now();
    // First TodoWrite of a conversation auto-opens the dock once. If user
    // closes it after, we respect that — no re-open on subsequent updates.
    if (next.length > 0 && !this.dockAutoOpenedThisConvo) {
      this.ui.dockOpen = true;
      this.dockAutoOpenedThisConvo = true;
    }
  }

  private shortToolLabel(name: string, input?: Record<string, unknown>): string {
    const base = name.replace(/^mcp__rift__/, "");
    const inp = input ?? {};
    if (base === "read_file" && typeof inp.path === "string") return `read ${inp.path}`;
    if (base === "list_dir" && typeof inp.path === "string") return `list ${inp.path}`;
    if (base === "grep" && typeof inp.pattern === "string") return `grep "${inp.pattern}"`;
    return base;
  }

  private appendToolUse(block: { id: string; name: string; input?: Record<string, unknown> }) {
    if (this.seenToolUseIds.has(block.id)) return;
    this.seenToolUseIds.add(block.id);
    // TodoWrite is intercepted — routes to Tasks dock instead of an inline card.
    if (block.name === "TodoWrite") {
      this.applyTodoWrite(block.input);
      return;
    }
    // Skip CC harness internals (ToolSearch for deferred-tool discovery, etc).
    // Only surface Rift's own MCP tools inline; everything else stays in the
    // CLI's internal layer and never reaches the chat UI.
    if (!block.name.startsWith("mcp__rift__")) return;
    this.activity = { ...this.activity, currentLabel: this.shortToolLabel(block.name, block.input) };
    // First tool call of a conversation auto-opens the dock — same UX as the
    // first TodoWrite. Respects user's manual close after that.
    if (!this.dockAutoOpenedThisConvo) {
      this.ui.dockOpen = true;
      this.dockAutoOpenedThisConvo = true;
    }
    this.mutateStreaming((m) => ({
      ...m,
      blocks: [
        ...m.blocks,
        {
          type: "tool",
          id: block.id,
          name: block.name,
          input: block.input ?? {},
          result: null,
          isError: false,
          status: "pending",
        },
      ],
    }));
  }

  private fillToolResult(toolUseId: string, content: string, isError: boolean) {
    this.mutateStreaming((m) => ({
      ...m,
      blocks: m.blocks.map((b) =>
        b.type === "tool" && b.id === toolUseId
          ? { ...b, result: content, isError, status: isError ? "error" : "done" }
          : b,
      ),
    }));
  }

  private onStream(raw: string) {
    let env: StreamEnvelope;
    try {
      env = JSON.parse(raw) as StreamEnvelope;
    } catch {
      console.debug("assistant stream: non-JSON line", raw);
      return;
    }
    switch (env.type) {
      case "stream_event": {
        const d = env.event?.delta;
        if (env.event?.type === "content_block_delta" && d?.type === "text_delta" && d.text) {
          this.appendText(d.text);
        }
        break;
      }
      case "assistant": {
        // Capture every tool_use block so its card renders before the
        // tool_result arrives. Text blocks are already streamed via
        // stream_event deltas, so we don't re-emit them here.
        for (const block of env.message?.content ?? []) {
          if (block.type === "tool_use") {
            this.appendToolUse(block);
          }
        }
        break;
      }
      case "user": {
        // Tool results from the CLI side.
        for (const block of env.message?.content ?? []) {
          if (block.type === "tool_result") {
            this.fillToolResult(
              block.tool_use_id,
              flattenToolResult(block.content),
              block.is_error === true,
            );
          }
        }
        break;
      }
      case "result": {
        if (typeof env.total_cost_usd === "number") {
          // CLI emits the cost FOR THIS RUN — accumulate across turns so
          // /cost reports the full session, not just the last turn.
          this.totalCostUsd = (this.totalCostUsd ?? 0) + env.total_cost_usd;
        }
        if (env.subtype && env.subtype !== "success") {
          this.lastError = `Run ended with subtype: ${env.subtype}`;
        }
        break;
      }
      default:
        break;
    }
  }

  private onDone() {
    this.streaming = false;
    this.streamingMsgId = null;
    this.seenToolUseIds.clear();
    this.activity = { ...this.activity, currentLabel: null };
    // Persist the just-completed turn. Debounced so a queued message that
    // fires the next turn within ~700ms only writes once.
    this.scheduleSave();
    // Drain the next queued message, if any. Run on next tick so the just-
    // finished assistant turn is fully painted before we kick off another.
    if (this.queue.length > 0) {
      const [next, ...rest] = this.queue;
      this.queue = rest;
      queueMicrotask(() => void this.send(next.text));
    }
  }

  async stop() {
    if (!this.streaming) return;
    try {
      await invoke("assistant_stop");
    } catch (e) {
      console.warn("assistant_stop failed", e);
    }
    // Backend emits assistant://done on kill — onDone() clears state.
  }

  removeQueued(id: string) {
    this.queue = this.queue.filter((q) => q.id !== id);
  }

  /** Client-side slash commands. Returns true if input was consumed. */
  private runSlash(input: string): boolean {
    const [cmd, ...rest] = input.slice(1).split(/\s+/);
    const arg = rest.join(" ").trim();
    switch (cmd.toLowerCase()) {
      case "clear":
      case "new":
        void this.newConversation();
        return true;
      case "history":
        this.ui.historyOpen = !this.ui.historyOpen;
        return true;
      case "stop":
        void this.stop();
        return true;
      case "model": {
        const v = arg.toLowerCase();
        if (v === "sonnet" || v === "opus" || v === "haiku") {
          this.setModel(v);
          this.lastNotice = `Model switched to ${v}.`;
        } else {
          this.lastError = `Unknown model "${arg}". Use sonnet, opus, or haiku.`;
        }
        return true;
      }
      case "retry":
        void this.retryLast();
        return true;
      case "copy":
        void this.copyLastAssistant();
        return true;
      case "cost":
        this.lastNotice =
          this.totalCostUsd != null
            ? `Session cost: $${this.totalCostUsd.toFixed(4)} USD across ${this.messages.filter((m) => m.role === "assistant").length} turn(s).`
            : "No cost recorded yet — send a message first.";
        return true;
      case "tools":
        this.lastNotice =
          "MCP tools (workspace-scoped, read-only): " +
          "read_file(path) — UTF-8 ≤500KB; " +
          "list_dir(path) — immediate children; " +
          "grep(pattern, path?, glob?, case_insensitive?) — regex search, ≤200 hits. " +
          "Plus TodoWrite for ≥3-step plans (surfaces in Tasks dock).";
        return true;
      case "help":
        this.lastNotice =
          "Slash commands: /new · /history · /model · /retry · /copy · /stop · /tools · /cost · /help. " +
          "Aliases: /clear → /new. Up-arrow recalls previous prompts.";
        return true;
      default:
        return false;
    }
  }

  /** Re-send the most recent user prompt. Drops the prior user+assistant
   *  pair from the visible history so the retry looks like a redo, not a
   *  duplicate. Aborts an in-flight stream first. */
  async retryLast() {
    const last = this.promptHistory[this.promptHistory.length - 1];
    if (!last) {
      this.lastError = "No previous prompt to retry.";
      return;
    }
    if (this.streaming) {
      await this.stop();
    }
    // Strip the trailing assistant turn (if any) and the matching user turn
    // so the replayed history doesn't double-include the prompt.
    const msgs = this.messages.slice();
    if (msgs[msgs.length - 1]?.role === "assistant") msgs.pop();
    if (msgs[msgs.length - 1]?.role === "user") msgs.pop();
    this.messages = msgs;
    await this.send(last);
  }

  /** Copy the latest assistant message's text content to the clipboard. */
  async copyLastAssistant() {
    const last = [...this.messages].reverse().find((m) => m.role === "assistant");
    if (!last) {
      this.lastError = "No assistant response to copy.";
      return;
    }
    const text = last.blocks
      .map((b) => (b.type === "text" ? b.text : ""))
      .join("")
      .trim();
    if (!text) {
      this.lastError = "Last response had no text content.";
      return;
    }
    try {
      await navigator.clipboard.writeText(text);
      this.lastNotice = `Copied ${text.length.toLocaleString()} chars to clipboard.`;
    } catch (e) {
      this.lastError = `Clipboard write failed: ${String(e)}`;
    }
  }

  /** Up-arrow recall. Returns the n-th-most-recent prompt, or null. */
  recallPrompt(offsetFromEnd: number): string | null {
    const idx = this.promptHistory.length - 1 - offsetFromEnd;
    return idx >= 0 ? this.promptHistory[idx] : null;
  }

  dismissNotice() {
    this.lastNotice = null;
  }

  private onError(msg: string) {
    this.lastError = msg;
    this.streaming = false;
    if (this.streamingMsgId) {
      const id = this.streamingMsgId;
      this.messages = this.messages.filter(
        (m) => !(m.id === id && m.blocks.length === 0),
      );
      this.streamingMsgId = null;
    }
    this.seenToolUseIds.clear();
  }

  clear() {
    this.messages = [];
    this.lastError = null;
    this.lastNotice = null;
    this.totalCostUsd = null;
    this.tasks = [];
    this.queue = [];
    this.promptHistory = [];
    this.dockAutoOpenedThisConvo = false;
    this.ui.dockOpen = false;
  }
}

export const assistant = new AssistantStore();

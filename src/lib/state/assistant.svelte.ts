// Assistant page state.
//
// Spawns the user's installed `claude` CLI through Rust commands; the CLI
// streams NDJSON which the backend forwards verbatim on `assistant://stream`.
// Wires Rift's MCP server (read_file / list_dir / grep) so assistant turns
// can interleave text, tool calls, and TodoWrite-driven task lists.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { uiPrefs } from "./ui-prefs.svelte";

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

export type ThinkingBlock = {
  type: "thinking";
  // Plaintext reasoning if the API streamed it. Often empty in -p mode —
  // Anthropic encrypts thinking content and only emits the signature, in
  // which case we show duration + a "reasoning recorded" hint instead.
  text: string;
  // Encrypted signature blob received (presence flag — we don't render it).
  hasSignature: boolean;
  // Wall-clock duration of the reasoning step. Null while still active.
  durationMs: number | null;
  status: "active" | "done";
};

export type Block = TextBlock | ToolBlock | ThinkingBlock;

export type ChatMessage = {
  id: string;
  role: "user" | "assistant";
  blocks: Block[];
  /** Per-turn cost in USD captured from the CLI `result` envelope. Only set
   *  on assistant messages after the turn completes. */
  costUsd?: number | null;
  /** Resolved model id captured from the CLI `system:init` envelope. */
  model?: string | null;
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
  | { type: "thinking"; thinking?: string; signature?: string }
  | { type: "tool_use"; id: string; name: string; input?: Record<string, unknown> }
  | { type: "tool_result"; tool_use_id: string; content?: unknown; is_error?: boolean };

type StreamDelta = {
  type?: string;
  text?: string;
  thinking?: string;
  signature?: string;
};

type StreamEvent = {
  type?: string;
  index?: number;
  content_block?: ContentBlock;
  delta?: StreamDelta;
};

type StreamEnvelope =
  | { type: "system"; subtype?: string; [k: string]: unknown }
  | { type: "stream_event"; event?: StreamEvent; [k: string]: unknown }
  | { type: "assistant"; message: { content: ContentBlock[] } }
  | { type: "user"; message: { content: ContentBlock[] } }
  | { type: "result"; subtype?: string; result?: string; total_cost_usd?: number; [k: string]: unknown };

type RemoteLockEvt = {
  file_path: string;
  user: string;
  host: string;
  since: string;
};

type RemoteShellEvt = {
  command: string;
  remote_root: string;
  at: string;
};

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
  useFullConfig = $state<boolean>(true);
  maxBudgetUsd = $state<number | null>(null);
  allowRemoteShell = $state<boolean>(false);
  remoteShellLockedByOther = $state<{ user: string; host: string; sinceMs: number } | null>(null);
  remoteShellBannerSeen = $state<boolean>(false);
  remoteShellLastEvent = $state<{ command: string; remoteRoot: string; at: string } | null>(null);

  // The Assistant's open project folder + recent-folder list. Decoupled from
  // Sync's server folders; populated by `assistant_get_workspace` on init and
  // updated whenever the user opens, switches, or clears a folder. Empty
  // `current` falls back to AutoSync folders on the Rust side.
  workspace = $state<WorkspaceState>({ current: null, recent: [] });

  // Cached relative file paths under the workspace root, populated on first
  // `@` trigger and re-loaded whenever the workspace root changes. Drives the
  // composer's `@`-file mention picker. Walk is cheap (~ms for typical FiveM
  // resource folder) so we re-fetch on each open rather than invalidate via
  // a watcher.
  workspaceFiles = $state<string[]>([]);
  workspaceFilesLoadingFor = $state<string | null>(null);

  composerDraft = $state("");
  // Outbound message queue. send() appends here if a turn is already
  // streaming; onDone() pops the next one. UI surfaces queued items as
  // pills above the composer with an X to remove.
  queue = $state<{ id: string; text: string }[]>([]);
  // User's chosen model — flipped by /model slash command. Carried through
  // to assistant_send so the CLI uses sonnet/opus/haiku per their choice.
  // Initialized from localStorage so the choice survives reloads.
  model = $state<"sonnet" | "opus" | "haiku">(loadModel());
  // `dockOpen` + `historyOpen` are v0.2-only. Under v0.3 (uiPrefs.useV03Shell),
  // Tasks + History live in the top-level Dock and visibility is driven by
  // `uiPrefs.panels.tasks.open` / `uiPrefs.panels.history.open`. The fields
  // here are kept so the v0.2 shell still renders pixel-identical for rollback.
  ui = $state({ dockOpen: false, tasksUpdatedAt: 0, historyOpen: false });

  // Conversation history.
  //   - `currentConvoId` is null before the first message is sent; first
  //     `send()` assigns a fresh UUID and persists from there.
  //   - `conversations` is the metadata cache for the drawer; refreshed
  //     after every save/delete/rename.
  //   - `createdAt` is set when the convo starts, kept stable across saves.
  //   - `openTabs` (v0.4) is the ordered list of convo ids visible as tabs in
  //     the top tab bar. Tabs share the singleton stream pipeline (mid-stream
  //     switch = stop stream; concurrent live UI deferred to v0.4.1).
  currentConvoId = $state<string | null>(null);
  conversations = $state<ConversationMeta[]>([]);
  openTabs = $state<string[]>([]);
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
  // Per-turn delta/envelope reconciliation. `--include-partial-messages` is
  // expected to stream text via stream_event text_deltas, but CLI version
  // drift or short responses sometimes emit zero deltas and ship the full
  // text only in the final `assistant` envelope. Without a fallback the
  // bubble renders blank. Track delta count + buffer envelope text; on
  // turn end, if no deltas arrived, flush the envelope buffer into the bubble.
  private deltaCount = 0;
  private envelopeTextBuffer = "";
  private rawLineLog: string[] = [];
  // Streaming pacer — decouples visual paint from network burst cadence so
  // CLI chunks of 200+ chars trickle out instead of slamming in. Base rate
  // ~120 ch/s; any backlog auto-drains in ~400ms to stay responsive.
  private pendingText = "";
  private drainHandle: ReturnType<typeof requestAnimationFrame> | null = null;
  private lastDrainAt = 0;
  // Per-turn thinking tracking. `content_block_start`/`stop` carry an `index`
  // identifying which block is open; we map that to the thinking block we
  // pushed into the message so subsequent `thinking_delta` / `signature_delta`
  // events land in the right place. `activeThinkingIndex` is the currently-
  // open block's index, null when no thinking is in flight.
  private thinkingByIndex = new Map<number, { blockOffset: number; startedAt: number }>();
  private activeThinkingIndex: number | null = null;

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
    try {
      this.useFullConfig = await invoke<boolean>("assistant_get_use_full_config");
    } catch (e) {
      console.warn("assistant_get_use_full_config failed", e);
    }
    try {
      this.maxBudgetUsd = await invoke<number | null>("assistant_get_max_budget_usd");
    } catch (e) {
      console.warn("assistant_get_max_budget_usd failed", e);
    }
    try {
      this.allowRemoteShell = await invoke<boolean>("assistant_get_allow_remote_shell");
    } catch (e) {
      console.warn("assistant_get_allow_remote_shell failed", e);
    }
    try {
      this.remoteShellBannerSeen = localStorage.getItem("rift.assistant.remoteShellBannerSeen") === "1";
    } catch { /* localStorage unavailable in some test contexts */ }

    this.unlistens.push(
      await listen<RemoteLockEvt[]>("autosync://locks", (e) => this.onLocksUpdate(e.payload)),
      await listen<RemoteShellEvt>("assistant://remote-shell-fired", (e) => this.onRemoteShellFired(e.payload)),
    );

    await this.refreshConversations();
    await this.refreshWorkspace();
    await this.restoreTabs();
  }

  private onLocksUpdate(locks: RemoteLockEvt[]) {
    const shell = locks.find((l) => l.file_path.endsWith("/.rift-shell"));
    if (shell) {
      const sinceMs = Date.parse(shell.since);
      this.remoteShellLockedByOther = {
        user: shell.user,
        host: shell.host,
        sinceMs: Number.isFinite(sinceMs) ? sinceMs : Date.now(),
      };
    } else {
      this.remoteShellLockedByOther = null;
    }
  }

  private onRemoteShellFired(evt: RemoteShellEvt) {
    this.remoteShellLastEvent = {
      command: evt.command,
      remoteRoot: evt.remote_root,
      at: evt.at,
    };
  }

  ackRemoteShellBanner() {
    this.remoteShellBannerSeen = true;
    try {
      localStorage.setItem("rift.assistant.remoteShellBannerSeen", "1");
    } catch { /* same as above */ }
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
      this.workspaceFiles = [];
      this.lastNotice = `Workspace: ${path}`;
    } catch (e) {
      this.lastError = `Set workspace failed: ${String(e)}`;
    }
  }

  async clearRoot() {
    try {
      this.workspace = await invoke<WorkspaceState>("assistant_clear_root");
      this.workspaceFiles = [];
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

  /** Lazy-load relative file paths under the current workspace root. Caches
   *  per-root in `workspaceFiles`; concurrent calls are de-duped via the
   *  `workspaceFilesLoadingFor` guard. */
  async loadWorkspaceFiles() {
    const root = this.workspace.current;
    if (!root) { this.workspaceFiles = []; return; }
    if (this.workspaceFilesLoadingFor === root) return;
    this.workspaceFilesLoadingFor = root;
    try {
      this.workspaceFiles = await invoke<string[]>("assistant_list_workspace_files");
    } catch (e) {
      console.warn("assistant_list_workspace_files failed", e);
    } finally {
      this.workspaceFilesLoadingFor = null;
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
      if (this.openTabs.includes(id)) {
        // Reuse closeTab so neighbor-pick + active-switch logic stays in one place.
        await this.closeTab(id);
      } else if (this.currentConvoId === id) {
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

  // ── v0.4 tabs ────────────────────────────────────────────────────────
  private persistTabs() {
    try {
      localStorage.setItem(
        "rift.ui.tabs.v1",
        JSON.stringify({ openTabs: this.openTabs, activeTabId: this.currentConvoId }),
      );
    } catch { /* localStorage unavailable */ }
  }

  private async restoreTabs() {
    try {
      const raw = localStorage.getItem("rift.ui.tabs.v1");
      if (!raw) return;
      const parsed = JSON.parse(raw) as { openTabs?: unknown; activeTabId?: unknown };
      const ids = Array.isArray(parsed.openTabs)
        ? parsed.openTabs.filter((s): s is string => typeof s === "string")
        : [];
      const existing = new Set(this.conversations.map((c) => c.id));
      const valid = ids.filter((id) => existing.has(id));
      this.openTabs = valid;
      const active = typeof parsed.activeTabId === "string" ? parsed.activeTabId : null;
      if (active && valid.includes(active)) {
        await this.loadConversation(active);
      } else if (valid.length > 0) {
        await this.loadConversation(valid[0]);
      }
      this.persistTabs();
    } catch (e) {
      console.warn("restoreTabs failed", e);
    }
  }

  /** Open a saved convo as a tab. Push to openTabs if not already there;
   *  activate + load from disk. Unsaved new-tab ids (minted by newTab() but
   *  no send yet → no disk record) drop into a fresh in-memory state instead
   *  of disk-load. Singleton stream pipeline — mid-stream switch is handled
   *  by loadConversation() calling stop(). */
  async openTab(id: string) {
    if (!this.openTabs.includes(id)) {
      this.openTabs = [...this.openTabs, id];
    }
    if (this.currentConvoId === id) {
      this.persistTabs();
      return;
    }
    if (this.messages.length > 0 && this.currentConvoId) {
      this.scheduleSave(true);
    }
    const inMeta = this.conversations.some((c) => c.id === id);
    if (inMeta) {
      await this.loadConversation(id);
    } else {
      if (this.streaming) await this.stop();
      this.currentConvoId = id;
      this.convoCreatedAt = null;
      this.convoTitle = null;
      this.messages = [];
      this.tasks = [];
      this.queue = [];
      this.lastError = null;
      this.lastNotice = null;
      this.totalCostUsd = null;
      this.dockAutoOpenedThisConvo = false;
    }
    this.persistTabs();
  }

  /** Close a tab. Removes from openTabs; convo stays on disk → still in History.
   *  Active-tab close picks the right neighbor (or left if at end); last-tab
   *  close drops to empty state w/ currentConvoId=null. */
  async closeTab(id: string) {
    const idx = this.openTabs.indexOf(id);
    if (idx === -1) return;
    const wasActive = this.currentConvoId === id;
    const next = this.openTabs.slice();
    next.splice(idx, 1);
    this.openTabs = next;
    if (wasActive) {
      // Save unsaved tail of the closing tab before switching/clearing.
      if (this.messages.length > 0 && this.convoCreatedAt) {
        this.scheduleSave(true);
      }
      if (this.streaming) await this.stop();
      if (next.length === 0) {
        this.messages = [];
        this.currentConvoId = null;
        this.convoCreatedAt = null;
        this.convoTitle = null;
        this.tasks = [];
        this.queue = [];
        this.lastError = null;
        this.lastNotice = null;
        this.totalCostUsd = null;
        this.dockAutoOpenedThisConvo = false;
      } else {
        // Right-priority: the entry that shifted into idx, else last.
        const neighbor = next[idx] ?? next[next.length - 1];
        const inMeta = this.conversations.some((c) => c.id === neighbor);
        if (inMeta) {
          await this.loadConversation(neighbor);
        } else {
          this.currentConvoId = neighbor;
          this.convoCreatedAt = null;
          this.convoTitle = null;
          this.messages = [];
          this.tasks = [];
          this.queue = [];
          this.lastError = null;
          this.lastNotice = null;
          this.totalCostUsd = null;
          this.dockAutoOpenedThisConvo = false;
        }
      }
    }
    this.persistTabs();
  }

  /** Open a fresh empty tab. Mints currentConvoId up-front so the tab can
   *  render before the first send; convoCreatedAt stays null so send() still
   *  flags isFirstTurn=true and the CLI gets --session-id, not --resume. */
  async newTab() {
    if (this.streaming) await this.stop();
    if (this.messages.length > 0 && this.currentConvoId) {
      this.scheduleSave(true);
    }
    const id = crypto.randomUUID();
    this.openTabs = [...this.openTabs, id];
    this.currentConvoId = id;
    this.convoCreatedAt = null;
    this.convoTitle = null;
    this.messages = [];
    this.tasks = [];
    this.queue = [];
    this.lastError = null;
    this.lastNotice = null;
    this.totalCostUsd = null;
    this.promptHistory = [];
    this.dockAutoOpenedThisConvo = false;
    this.persistTabs();
  }

  reorderTabs(fromIdx: number, toIdx: number) {
    if (fromIdx === toIdx) return;
    if (fromIdx < 0 || fromIdx >= this.openTabs.length) return;
    const next = this.openTabs.slice();
    const [moved] = next.splice(fromIdx, 1);
    const clamped = Math.max(0, Math.min(toIdx, next.length));
    next.splice(clamped, 0, moved);
    this.openTabs = next;
    this.persistTabs();
  }

  async cycleTab(direction: 1 | -1) {
    if (this.openTabs.length === 0) return;
    const cur = this.currentConvoId ? this.openTabs.indexOf(this.currentConvoId) : -1;
    const n = this.openTabs.length;
    const nextIdx = ((cur < 0 ? 0 : cur + direction) + n) % n;
    await this.openTab(this.openTabs[nextIdx]);
  }

  async closeOtherTabs(keepId: string) {
    const others = this.openTabs.filter((id) => id !== keepId);
    if (others.length === 0) return;
    this.openTabs = [keepId];
    if (this.currentConvoId !== keepId) {
      await this.loadConversation(keepId);
    }
    this.persistTabs();
  }

  /** Wipe all open tabs and drop into the empty-tabs state. Flushes the
   *  current convo if it has messages so nothing's lost; closes streams. */
  async closeAllTabs() {
    if (this.streaming) await this.stop();
    if (this.messages.length > 0 && this.convoCreatedAt) {
      this.scheduleSave(true);
    }
    this.openTabs = [];
    this.currentConvoId = null;
    this.convoCreatedAt = null;
    this.convoTitle = null;
    this.messages = [];
    this.tasks = [];
    this.queue = [];
    this.lastError = null;
    this.lastNotice = null;
    this.totalCostUsd = null;
    this.dockAutoOpenedThisConvo = false;
    this.persistTabs();
  }

  async closeTabsToRight(anchorId: string) {
    const idx = this.openTabs.indexOf(anchorId);
    if (idx === -1 || idx === this.openTabs.length - 1) return;
    const kept = this.openTabs.slice(0, idx + 1);
    const removedActive = this.currentConvoId && !kept.includes(this.currentConvoId);
    this.openTabs = kept;
    if (removedActive) {
      await this.loadConversation(anchorId);
    }
    this.persistTabs();
  }

  // ── /v0.4 tabs ───────────────────────────────────────────────────────

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

  async setUseFullConfig(value: boolean) {
    await invoke("assistant_set_use_full_config", { value });
    this.useFullConfig = value;
  }

  async setMaxBudgetUsd(value: number | null) {
    const v = value !== null && Number.isFinite(value) && value > 0 ? value : null;
    await invoke("assistant_set_max_budget_usd", { value: v });
    this.maxBudgetUsd = v;
  }

  async setAllowRemoteShell(value: boolean) {
    await invoke("assistant_set_allow_remote_shell", { value });
    this.allowRemoteShell = value;
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
    // Phase 2 (S72): the CLI owns conversation state now. First turn mints a
    // UUID and passes `--session-id`; subsequent turns pass `--resume` against
    // the same UUID. The session is persisted under `~/.claude/projects/<cwd-hash>/`.
    // v0.4: newTab() mints currentConvoId up-front so the tab can render
    // before send() — gate isFirstTurn on convoCreatedAt instead so the very
    // first send still passes --session-id, not --resume.
    const isFirstTurn = !this.convoCreatedAt;
    if (!this.currentConvoId) {
      this.currentConvoId = crypto.randomUUID();
    }
    if (!this.convoCreatedAt) {
      this.convoCreatedAt = Date.now();
      this.convoTitle = null;
    }
    // v0.4: under v0.3 shell, ensure the active convo has a tab. Catches the
    // raw newConversation→send path (slash /new under v0.2 etc.) so tabs are
    // never out of sync with the streaming convo.
    if (uiPrefs.useV03Shell && this.currentConvoId && !this.openTabs.includes(this.currentConvoId)) {
      this.openTabs = [...this.openTabs, this.currentConvoId];
      this.persistTabs();
    }
    this.streaming = true;
    this.lastError = null;
    this.lastNotice = null;
    this.seenToolUseIds.clear();
    this.deltaCount = 0;
    this.envelopeTextBuffer = "";
    this.rawLineLog = [];
    if (this.drainHandle !== null) {
      cancelAnimationFrame(this.drainHandle);
      this.drainHandle = null;
    }
    this.pendingText = "";
    this.thinkingByIndex.clear();
    this.activeThinkingIndex = null;
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
      await invoke("assistant_send", {
        prompt: trimmed,
        sessionId: this.currentConvoId,
        isFirstTurn,
        model: this.model,
      });
    } catch (e) {
      this.onError(String(e));
    }
  }

  private mutateStreaming(fn: (m: ChatMessage) => ChatMessage) {
    if (!this.streamingMsgId) return;
    this.messages = this.messages.map((m) => (m.id === this.streamingMsgId ? fn(m) : m));
  }

  private beginThinking(index: number) {
    if (this.thinkingByIndex.has(index)) return;
    this.activeThinkingIndex = index;
    const startedAt = Date.now();
    // Push a fresh thinking block into the current assistant message and
    // remember its offset so subsequent deltas find it after other blocks
    // are appended.
    this.mutateStreaming((m) => {
      const blocks = m.blocks.slice();
      this.thinkingByIndex.set(index, { blockOffset: blocks.length, startedAt });
      blocks.push({
        type: "thinking",
        text: "",
        hasSignature: false,
        durationMs: null,
        status: "active",
      });
      return { ...m, blocks };
    });
    this.activity = { ...this.activity, currentLabel: "Thinking…" };
  }

  private mutateThinking(index: number, fn: (b: ThinkingBlock) => ThinkingBlock) {
    const entry = this.thinkingByIndex.get(index);
    if (!entry) return;
    this.mutateStreaming((m) => {
      const blocks = m.blocks.slice();
      const target = blocks[entry.blockOffset];
      if (target && target.type === "thinking") {
        blocks[entry.blockOffset] = fn(target);
      }
      return { ...m, blocks };
    });
  }

  private appendThinkingText(index: number, chunk: string) {
    if (!chunk) return;
    this.mutateThinking(index, (b) => ({ ...b, text: b.text + chunk }));
  }

  private markThinkingSignature(index: number) {
    this.mutateThinking(index, (b) => (b.hasSignature ? b : { ...b, hasSignature: true }));
  }

  private endThinking(index: number) {
    const entry = this.thinkingByIndex.get(index);
    if (!entry) return;
    const durationMs = Date.now() - entry.startedAt;
    this.mutateThinking(index, (b) => ({ ...b, status: "done", durationMs }));
    if (this.activeThinkingIndex === index) {
      this.activeThinkingIndex = null;
      // Don't clobber a tool label that may have come in between.
      if (this.activity.currentLabel === "Thinking…") {
        this.activity = { ...this.activity, currentLabel: null };
      }
    }
  }

  private ensureThinkingFromEnvelope(block: { thinking?: string; signature?: string }) {
    // If a thinking block exists for the current message and is still empty,
    // hydrate it from the envelope. Otherwise append a finalized one.
    if (!this.streamingMsgId) return;
    const msg = this.messages.find((m) => m.id === this.streamingMsgId);
    if (!msg) return;
    const existing = msg.blocks.find((b) => b.type === "thinking") as ThinkingBlock | undefined;
    const envText = typeof block.thinking === "string" ? block.thinking : "";
    const envSig = !!block.signature && block.signature.length > 0;
    if (existing) {
      if (envText.length > existing.text.length || (envSig && !existing.hasSignature)) {
        this.mutateStreaming((m) => ({
          ...m,
          blocks: m.blocks.map((b) =>
            b.type === "thinking" && b === existing
              ? { ...b, text: envText.length > b.text.length ? envText : b.text, hasSignature: b.hasSignature || envSig }
              : b,
          ),
        }));
      }
      return;
    }
    this.mutateStreaming((m) => ({
      ...m,
      blocks: [
        ...m.blocks,
        {
          type: "thinking",
          text: envText,
          hasSignature: envSig,
          durationMs: null,
          status: "done",
        },
      ],
    }));
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

  private enqueueText(chunk: string) {
    if (!chunk) return;
    this.pendingText += chunk;
    if (this.drainHandle === null) {
      this.lastDrainAt = performance.now();
      this.drainHandle = requestAnimationFrame(this.drainTick);
    }
  }

  private drainTick = () => {
    if (this.pendingText.length === 0) {
      this.drainHandle = null;
      return;
    }
    const now = performance.now();
    const dt = Math.min(now - this.lastDrainAt, 100);
    this.lastDrainAt = now;
    const rate = Math.max(120, this.pendingText.length / 0.4);
    const n = Math.min(
      this.pendingText.length,
      Math.max(1, Math.round((rate * dt) / 1000)),
    );
    const chunk = this.pendingText.slice(0, n);
    this.pendingText = this.pendingText.slice(n);
    this.appendText(chunk);
    this.drainHandle = requestAnimationFrame(this.drainTick);
  };

  private flushPendingText() {
    if (this.drainHandle !== null) {
      cancelAnimationFrame(this.drainHandle);
      this.drainHandle = null;
    }
    if (this.pendingText.length > 0) {
      this.appendText(this.pendingText);
      this.pendingText = "";
    }
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
    this.ui.dockOpen = true; uiPrefs.setPanelOpen("tasks", true);
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
      this.ui.dockOpen = true; uiPrefs.setPanelOpen("tasks", true);
      this.dockAutoOpenedThisConvo = true;
    }
  }

  private shortToolLabel(name: string, input?: Record<string, unknown>): string {
    const base = name.replace(/^mcp__rift__/, "");
    const inp = input ?? {};
    // Claude Code built-ins (PascalCase) + Rift MCP variants (snake_case).
    if ((base === "Read" || base === "read_file") && typeof (inp.file_path ?? inp.path) === "string")
      return `read ${inp.file_path ?? inp.path}`;
    if (base === "Write" && typeof inp.file_path === "string") return `write ${inp.file_path}`;
    if (base === "Edit" && typeof inp.file_path === "string") return `edit ${inp.file_path}`;
    if (base === "Bash" && typeof inp.command === "string") {
      const c = inp.command as string;
      return `$ ${c.length > 70 ? c.slice(0, 69) + "…" : c}`;
    }
    if (base === "Glob" && typeof inp.pattern === "string") return `glob ${inp.pattern}`;
    if ((base === "Grep" || base === "grep") && typeof inp.pattern === "string") return `grep "${inp.pattern}"`;
    if (base === "WebFetch" && typeof inp.url === "string") return `fetch ${inp.url}`;
    if (base === "WebSearch" && typeof inp.query === "string") return `search "${inp.query}"`;
    if (base === "list_dir" && typeof inp.path === "string") return `list ${inp.path}`;
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
    // Surface all whitelisted workspace tools — Claude Code built-ins
    // (Read/Write/Edit/Bash/Glob/Grep/WebFetch/WebSearch) and Rift's MCP
    // variants. Deny-list CLI-internal helpers we never want in the chat UI.
    const DENY = new Set(["ToolSearch"]);
    if (DENY.has(block.name)) return;
    this.activity = { ...this.activity, currentLabel: this.shortToolLabel(block.name, block.input) };
    // First tool call of a conversation auto-opens the dock — same UX as the
    // first TodoWrite. Respects user's manual close after that.
    if (!this.dockAutoOpenedThisConvo) {
      this.ui.dockOpen = true; uiPrefs.setPanelOpen("tasks", true);
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
    // Ring-buffer raw lines so a blank-turn fallback or post-mortem can dump them.
    if (this.rawLineLog.length >= 200) this.rawLineLog.shift();
    this.rawLineLog.push(raw);
    let env: StreamEnvelope;
    try {
      env = JSON.parse(raw) as StreamEnvelope;
    } catch {
      // Non-JSON line on stdout — happens when the CLI silently downgrades
      // from stream-json to plain text (version/flag drift). Render it as
      // assistant text so the bubble isn't blank. Increment deltaCount to
      // suppress the envelope fallback in onDone().
      if (this.streaming && this.streamingMsgId && raw.length > 0) {
        const prefix = this.deltaCount > 0 ? "\n" : "";
        this.deltaCount++;
        this.enqueueText(prefix + raw);
      } else {
        console.debug("assistant stream: non-JSON line (idle)", raw);
      }
      return;
    }
    switch (env.type) {
      case "stream_event": {
        const ev = env.event;
        const evType = ev?.type;
        const idx = typeof ev?.index === "number" ? ev.index : null;
        if (evType === "content_block_start" && ev?.content_block?.type === "thinking" && idx !== null) {
          this.beginThinking(idx);
        } else if (evType === "content_block_delta") {
          const d = ev?.delta;
          if (d?.type === "text_delta" && d.text) {
            this.deltaCount++;
            this.enqueueText(d.text);
          } else if (d?.type === "thinking_delta" && typeof d.thinking === "string" && idx !== null) {
            this.appendThinkingText(idx, d.thinking);
          } else if (d?.type === "signature_delta" && idx !== null) {
            this.markThinkingSignature(idx);
          }
        } else if (evType === "content_block_stop" && idx !== null) {
          this.endThinking(idx);
        }
        break;
      }
      case "assistant": {
        // Capture every tool_use block so its card renders before the
        // tool_result arrives. Text blocks are normally streamed via
        // stream_event deltas, but we also buffer their final text here as
        // a fallback for when --include-partial-messages emits zero deltas
        // (CLI version drift, certain short responses). Flushed in onDone()
        // if deltaCount stayed 0.
        for (const block of env.message?.content ?? []) {
          if (block.type === "tool_use") {
            this.appendToolUse(block);
          } else if (block.type === "text" && typeof block.text === "string") {
            this.envelopeTextBuffer += block.text;
          } else if (block.type === "thinking") {
            // Final form of a thinking block. If we never saw stream events
            // for it (e.g. older CLI w/o partial-message support for
            // thinking), synthesize one now from the envelope alone.
            this.ensureThinkingFromEnvelope(block);
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
          // Pin per-turn cost to the streaming assistant message so the
          // bubble can render a tiny "$0.0042" pill next to "Claude".
          const turnCost = env.total_cost_usd;
          this.mutateStreaming((m) => ({ ...m, costUsd: turnCost }));
        }
        if (env.subtype && env.subtype !== "success") {
          this.lastError = `Run ended with subtype: ${env.subtype}`;
        }
        break;
      }
      case "system": {
        // CLI emits a `{type:"system",subtype:"init",...,model:"..."}` line
        // at the start of every spawn. Capture the resolved model id so the
        // bubble's per-turn badge shows what actually ran (e.g. "sonnet" alias
        // → "claude-sonnet-4-6").
        const sysModel = typeof env.model === "string" ? env.model : null;
        if (sysModel) {
          this.mutateStreaming((m) => ({ ...m, model: sysModel }));
        }
        break;
      }
      default:
        break;
    }
  }

  private onDone() {
    // Drain any text still sitting in the pacer buffer before deciding whether
    // we need the envelope fallback — pending counts as "deltas arrived."
    this.flushPendingText();
    // Fallback: zero text deltas this turn → CLI shipped full text only in
    // the final assistant envelope. Flush it now so the bubble isn't blank.
    if (this.deltaCount === 0 && this.envelopeTextBuffer.length > 0) {
      this.appendText(this.envelopeTextBuffer);
      console.debug(
        `[assistant] envelope-fallback flushed ${this.envelopeTextBuffer.length} chars (zero deltas this turn)`,
      );
    } else if (this.deltaCount === 0 && this.envelopeTextBuffer.length === 0) {
      // Genuinely blank turn — dump raw NDJSON to console AND surface to UI
      // so the user doesn't need DevTools to diagnose. Skip if any tool calls
      // fired (those are visible inline and a text-less assistant turn is valid).
      const msg = this.streamingMsgId
        ? this.messages.find((m) => m.id === this.streamingMsgId)
        : null;
      const hadTools = !!msg && msg.blocks.some((b) => b.type === "tool");
      if (!hadTools) {
        const lines = this.rawLineLog.slice();
        console.warn(
          "[assistant] turn ended with no text and no tools. Raw stream lines:",
          lines,
        );
        // Pretty-print the envelope types we did see — a quick fingerprint of
        // what the CLI emitted. Full lines are in console.
        const types: string[] = [];
        const nonJsonSamples: string[] = [];
        for (const ln of lines) {
          try {
            const parsed = JSON.parse(ln) as { type?: string; subtype?: string };
            types.push(parsed.subtype ? `${parsed.type}:${parsed.subtype}` : (parsed.type ?? "?"));
          } catch {
            types.push("non-json");
            if (nonJsonSamples.length < 3) {
              nonJsonSamples.push(ln.length > 240 ? ln.slice(0, 240) + "…" : ln);
            }
          }
        }
        const fingerprint = `[${types.join(", ")}]`;
        const tail =
          nonJsonSamples.length > 0
            ? ` Non-JSON output: ${nonJsonSamples.map((s) => `"${s}"`).join(" | ")}`
            : " Full NDJSON in DevTools console.";
        this.lastError = `Blank response — CLI emitted ${lines.length} line(s): ${fingerprint}.${tail}`;
      }
    }
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
        if (uiPrefs.useV03Shell) void this.newTab();
        else void this.newConversation();
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
          "Tools available this turn: " +
          "Read / Write / Edit (files); Bash (shell, in workspace cwd); " +
          "Glob (filename patterns); Grep (content search); " +
          "WebFetch / WebSearch (open web); " +
          "TodoWrite (multi-step plans → Tasks dock). " +
          "Rift MCP: read_file / list_dir / grep (workspace-scoped helpers)" +
          (this.allowRemoteShell ? "; remote_bash (russh exec on the active SSH session)." : ".");
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
    if (this.drainHandle !== null) {
      cancelAnimationFrame(this.drainHandle);
      this.drainHandle = null;
    }
    this.pendingText = "";
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

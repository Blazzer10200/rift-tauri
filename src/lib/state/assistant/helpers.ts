// M1 (per docs/design/assistant-svelte-split.md) — pure helper fns lifted out
// of `src/lib/state/assistant.svelte.ts`. Zero state, zero IPC; only
// localStorage prefs + pure transforms. Safe to import anywhere.

import type { ChatMessage, ModelFamily, ModelSel, PermissionMode, ThinkingEffort } from "./types";

const MODEL_SELS: readonly ModelSel[] = [
  "sonnet", "opus", "claude-opus-4-7", "haiku",
] as const;

const MODEL_KEY = "rift.assistant.model";
const EFFORT_KEY = "rift.assistant.thinkingEffort";
const PERMISSION_KEY = "rift.assistant.permissionMode";

const PERMISSION_MODES: readonly PermissionMode[] = [
  "default", "acceptEdits", "plan", "auto", "bypassPermissions",
] as const;

export function loadModel(): ModelSel {
  try {
    const v = typeof localStorage !== "undefined" ? localStorage.getItem(MODEL_KEY) : null;
    if (v && (MODEL_SELS as readonly string[]).includes(v)) return v as ModelSel;
  } catch {
    /* SSR or storage disabled */
  }
  return "sonnet";
}

export function saveModel(v: ModelSel) {
  try {
    if (typeof localStorage !== "undefined") localStorage.setItem(MODEL_KEY, v);
  } catch {
    /* storage disabled */
  }
}

/** Map a selected model to its visual family for the aurora hue. */
export function modelFamily(model: ModelSel): ModelFamily {
  if (model === "haiku") return "haiku";
  if (model === "opus" || model.includes("opus")) return "opus";
  return "sonnet";
}

export function loadEffort(): ThinkingEffort {
  try {
    const v = typeof localStorage !== "undefined" ? localStorage.getItem(EFFORT_KEY) : null;
    if (v === "none" || v === "quick" || v === "deep" || v === "ultra") return v;
  } catch {
    /* SSR or storage disabled */
  }
  return "quick";
}

export function saveEffort(v: ThinkingEffort) {
  try {
    if (typeof localStorage !== "undefined") localStorage.setItem(EFFORT_KEY, v);
  } catch {
    /* storage disabled */
  }
}

export function loadPermissionMode(): PermissionMode {
  try {
    const v = typeof localStorage !== "undefined" ? localStorage.getItem(PERMISSION_KEY) : null;
    if (v && (PERMISSION_MODES as readonly string[]).includes(v)) return v as PermissionMode;
  } catch {
    /* SSR or storage disabled */
  }
  // Preserve Rift's historical behavior until the user picks a mode.
  return "bypassPermissions";
}

export function savePermissionMode(v: PermissionMode) {
  try {
    if (typeof localStorage !== "undefined") localStorage.setItem(PERMISSION_KEY, v);
  } catch {
    /* storage disabled */
  }
}

export function flattenToolResult(content: unknown): string {
  if (typeof content === "string") return content;
  if (Array.isArray(content)) {
    return content
      .map((c) => (typeof c === "object" && c && "text" in c ? String((c as { text: unknown }).text ?? "") : ""))
      .join("");
  }
  return "";
}

/** First-priority field to preview for each known tool. Returns first ~120
 *  chars of that field's string value, or null. */
export function previewToolInput(_name: string, input: Record<string, unknown> | undefined): string | null {
  if (!input) return null;
  const fields = ["command", "file_path", "pattern", "path", "url", "query"] as const;
  for (const f of fields) {
    const v = input[f];
    if (typeof v === "string" && v.length > 0) {
      return v.length > 120 ? v.slice(0, 120) + "…" : v;
    }
  }
  return null;
}

/** Tool names whose presence in a tab's stream means the Session-panel right
 *  rail has content worth surfacing. */
const CONTEXT_SIGNAL_TOOLS = new Set([
  "Edit", "Write", "MultiEdit", "NotebookEdit", "WebFetch", "WebSearch",
]);

/** Early-exit scan: does this message list contain ANY Edit/Write/WebFetch/etc
 *  tool call? Cheap — bails on first match. */
export function messagesHaveContextSignals(messages: ChatMessage[]): boolean {
  for (const m of messages) {
    for (const b of m.blocks) {
      if (b.type === "tool" && CONTEXT_SIGNAL_TOOLS.has(b.name)) return true;
    }
  }
  return false;
}

/** A live, in-flight unit of work for a tab: a pending Bash shell or an
 *  agent spawn that hasn't reported a result yet. */
export type LiveActivityItem = {
  id: string;
  kind: "shell" | "agent";
  label: string;
  /** Sub-label (agent subagentType) or null for shells. */
  sub: string | null;
  startedAt: number;
};

/** First line of a shell command, trimmed + capped at 60 chars for compact
 *  display. Shared by the Activity panel rows and the composer live pills. */
export function firstLine(cmd: string): string {
  const line = (cmd.split("\n")[0] ?? "").trim();
  return line.length > 60 ? line.slice(0, 59) + "…" : line;
}

/** Live "what's running now" for a tab: pending Bash shells + in-flight agent
 *  spawns (no completedAt), sorted by start time. Single source of truth so
 *  the ActivityPanel (full rows) and the composer live pills (counts only)
 *  can never disagree. `fallbackTs` stands in for a shell block that's missing
 *  a startedAt (legacy records). */
export function liveActivity(
  messages: ChatMessage[],
  agentSpawns: { id: string; subagentType: string; description: string; startedAt: number; completedAt: number | null }[],
  fallbackTs: number,
): LiveActivityItem[] {
  const out: LiveActivityItem[] = [];
  for (const m of messages) {
    for (const b of m.blocks) {
      if (b.type !== "tool" || b.name !== "Bash" || b.status !== "pending") continue;
      const cmd = typeof b.input.command === "string" ? b.input.command : "";
      out.push({ id: b.id, kind: "shell", label: firstLine(cmd) || "shell", sub: null, startedAt: b.startedAt ?? fallbackTs });
    }
  }
  for (const a of agentSpawns) {
    if (a.completedAt != null) continue;
    out.push({ id: a.id, kind: "agent", label: a.description, sub: a.subagentType, startedAt: a.startedAt });
  }
  return out.sort((x, y) => x.startedAt - y.startedAt);
}

/** Effort → CLI flag mapping. Must mirror src-tauri/src/assistant/mod.rs.
 *  `ultra` (ultracode) maps to xhigh effort; the autonomous-workflow behavior
 *  rides the separate `ultracode` settings key, also set in mod.rs. */
export function effortToFlag(
  effort: ThinkingEffort,
  model: ModelSel,
): "low" | "medium" | "high" | "xhigh" | null {
  if (model === "haiku") return null;
  if (effort === "none") return "low";
  if (effort === "deep") return "high";
  if (effort === "ultra") return "xhigh";
  return "medium";
}

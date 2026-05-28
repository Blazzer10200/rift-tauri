// M1 (per docs/design/assistant-svelte-split.md) — pure helper fns lifted out
// of `src/lib/state/assistant.svelte.ts`. Zero state, zero IPC; only
// localStorage prefs + pure transforms. Safe to import anywhere.

import type { ChatMessage, PermissionMode, ThinkingEffort } from "./types";

const MODEL_KEY = "rift.assistant.model";
const EFFORT_KEY = "rift.assistant.thinkingEffort";
const PERMISSION_KEY = "rift.assistant.permissionMode";

const PERMISSION_MODES: readonly PermissionMode[] = [
  "default", "acceptEdits", "plan", "auto", "bypassPermissions",
] as const;

export function loadModel(): "sonnet" | "opus" | "haiku" {
  try {
    const v = typeof localStorage !== "undefined" ? localStorage.getItem(MODEL_KEY) : null;
    if (v === "sonnet" || v === "opus" || v === "haiku") return v;
  } catch {
    /* SSR or storage disabled */
  }
  return "sonnet";
}

export function saveModel(v: "sonnet" | "opus" | "haiku") {
  try {
    if (typeof localStorage !== "undefined") localStorage.setItem(MODEL_KEY, v);
  } catch {
    /* storage disabled */
  }
}

export function loadEffort(): ThinkingEffort {
  try {
    const v = typeof localStorage !== "undefined" ? localStorage.getItem(EFFORT_KEY) : null;
    if (v === "none" || v === "quick" || v === "deep") return v;
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

/** Effort → CLI flag mapping. Must mirror src-tauri/src/assistant/mod.rs. */
export function effortToFlag(
  effort: "none" | "quick" | "deep",
  model: "sonnet" | "opus" | "haiku",
): "low" | "medium" | "high" | null {
  if (model === "haiku") return null;
  if (effort === "none") return "low";
  if (effort === "deep") return "high";
  return "medium";
}

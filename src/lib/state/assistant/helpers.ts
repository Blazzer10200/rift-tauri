// M1 (per docs/design/assistant-svelte-split.md) — pure helper fns lifted out
// of `src/lib/state/assistant.svelte.ts`. Zero state, zero IPC; only
// localStorage prefs + pure transforms. Safe to import anywhere.

import type { ChatMessage, ModelFamily, ModelSel, PermissionMode, ThinkingEffort } from "./types";
import { captionForTool } from "$lib/components/assistant/toolCaption";

const MODEL_SELS: readonly ModelSel[] = [
  "sonnet", "opus", "claude-opus-4-7", "haiku",
] as const;

const MODEL_KEY = "rift.assistant.model";
const EFFORT_KEY = "rift.assistant.thinkingEffort";
const PERMISSION_KEY = "rift.assistant.permissionMode";
const DOCK_WIDTH_KEY = "rift.assistant.dockWidth";
const DOCK_COLLAPSE_KEY = "rift.assistant.dockCollapsed";

// Per-workspace override keys for model + effort. A `base::<root>` key holds a
// workspace's pinned choice; the bare global key is the baseline default for
// workspaces that have never been pinned. A per-workspace save writes ONLY the
// `base::<root>` key — it must NOT touch the global, or pinning a heavy project
// (e.g. a WPF repo with rebuild-on-chat loops) to Sonnet would drag the baseline
// to Sonnet and bleed into every unpinned project — the exact "one global
// setting dragging every project to the same cost" this feature exists to stop.
// The global baseline is updated only on a no-workspace save (ws null).
function wsKey(base: string, ws: string | null | undefined): string | null {
  return ws ? `${base}::${ws}` : null;
}

export const DOCK_MIN = 260;
export const DOCK_MAX = 520;
const DOCK_DEFAULT = 300;

const PERMISSION_MODES: readonly PermissionMode[] = [
  "default", "acceptEdits", "plan", "auto", "bypassPermissions",
] as const;

export function loadModel(ws?: string | null): ModelSel {
  try {
    if (typeof localStorage !== "undefined") {
      const k = wsKey(MODEL_KEY, ws);
      const v = (k ? localStorage.getItem(k) : null) ?? localStorage.getItem(MODEL_KEY);
      if (v && (MODEL_SELS as readonly string[]).includes(v)) return v as ModelSel;
    }
  } catch {
    /* SSR or storage disabled */
  }
  return "sonnet";
}

export function saveModel(v: ModelSel, ws?: string | null) {
  try {
    if (typeof localStorage === "undefined") return;
    const k = wsKey(MODEL_KEY, ws);
    if (k) localStorage.setItem(k, v); // per-workspace pin — never touches the global baseline
    else localStorage.setItem(MODEL_KEY, v); // no workspace → set the baseline default
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

export function loadEffort(ws?: string | null): ThinkingEffort {
  try {
    if (typeof localStorage !== "undefined") {
      const k = wsKey(EFFORT_KEY, ws);
      const v = (k ? localStorage.getItem(k) : null) ?? localStorage.getItem(EFFORT_KEY);
      if (v === "none" || v === "quick" || v === "deep" || v === "ultra") return v;
    }
  } catch {
    /* SSR or storage disabled */
  }
  return "quick";
}

export function saveEffort(v: ThinkingEffort, ws?: string | null) {
  try {
    if (typeof localStorage === "undefined") return;
    const k = wsKey(EFFORT_KEY, ws);
    if (k) localStorage.setItem(k, v); // per-workspace pin — never touches the global baseline
    else localStorage.setItem(EFFORT_KEY, v); // no workspace → set the baseline default
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

export function loadDockWidth(): number {
  try {
    const v = typeof localStorage !== "undefined" ? localStorage.getItem(DOCK_WIDTH_KEY) : null;
    const n = v ? parseInt(v, 10) : NaN;
    if (Number.isFinite(n)) return Math.min(DOCK_MAX, Math.max(DOCK_MIN, n));
  } catch {
    /* SSR or storage disabled */
  }
  return DOCK_DEFAULT;
}

export function saveDockWidth(px: number) {
  try {
    const clamped = Math.min(DOCK_MAX, Math.max(DOCK_MIN, Math.round(px)));
    if (typeof localStorage !== "undefined") localStorage.setItem(DOCK_WIDTH_KEY, String(clamped));
  } catch {
    /* storage disabled */
  }
}

/** Set of ActivityPanel section keys the user has collapsed. App-global UI pref. */
export function loadCollapsedSections(): Set<string> {
  try {
    const v = typeof localStorage !== "undefined" ? localStorage.getItem(DOCK_COLLAPSE_KEY) : null;
    if (v) {
      const arr = JSON.parse(v);
      if (Array.isArray(arr)) return new Set(arr.filter((x): x is string => typeof x === "string"));
    }
  } catch {
    /* SSR or storage disabled */
  }
  return new Set();
}

export function saveCollapsedSections(s: Set<string>) {
  try {
    if (typeof localStorage !== "undefined") localStorage.setItem(DOCK_COLLAPSE_KEY, JSON.stringify([...s]));
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
  kind: "shell" | "agent" | "tool" | "thinking";
  label: string;
  /** Sub-label: agent subagentType, the tool name for generic tools, or null. */
  sub: string | null;
  startedAt: number;
};

/** Agent-launching tool names — surfaced via agentSpawns, so the generic
 *  pending-tool branch skips them to avoid double-listing. */
const AGENT_TOOL_NAMES = new Set(["Task", "Agent"]);

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
      // Active reasoning → a single "Thinking…" row so the panel isn't blank
      // during the (often long) pre-tool think.
      if (b.type === "thinking" && b.status === "active") {
        out.push({ id: `${m.id}:think`, kind: "thinking", label: "Thinking…", sub: null, startedAt: b.startedAt });
        continue;
      }
      if (b.type !== "tool" || b.status !== "pending") continue;
      // Agent launches ride agentSpawns below — skip here to avoid double-list.
      if (AGENT_TOOL_NAMES.has(b.name)) continue;
      if (b.name === "Bash") {
        const cmd = typeof b.input.command === "string" ? b.input.command : "";
        out.push({ id: b.id, kind: "shell", label: firstLine(cmd) || "shell", sub: null, startedAt: b.startedAt ?? fallbackTs });
      } else {
        // Read/Edit/Grep/Glob/Write/WebFetch/… — the previously invisible
        // majority. Friendly caption matching the transcript rail.
        out.push({ id: b.id, kind: "tool", label: captionForTool(b.name, b.input), sub: null, startedAt: b.startedAt ?? fallbackTs });
      }
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

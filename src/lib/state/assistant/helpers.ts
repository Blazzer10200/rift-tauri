// M1 (per docs/design/assistant-svelte-split.md) — pure helper fns lifted out
// of `src/lib/state/assistant.svelte.ts`. Zero state, zero IPC; only
// localStorage prefs + pure transforms. Safe to import anywhere.

import type { ChatMessage, ModelFamily, ModelSel, PermissionMode, ThinkingEffort } from "./types";
import { captionForTool } from "$lib/components/assistant/toolCaption";

const MODEL_SELS: readonly ModelSel[] = [
  "sonnet", "opus", "claude-opus-4-7", "haiku", "claude-fable-5",
] as const;

// Claude Fable 5 — limited run, offered only through Jun 22 2026 (EOD UTC).
// After sunset the picker hides it and a stored pref self-heals to the default.
export const FABLE_SUNSET_MS = Date.UTC(2026, 5, 23);
// Manual kill-switch — Fable pulled 2026-06-14 (US-gov disablement, temporary).
// While true the picker hides Fable and any stored/selected Fable pref coerces
// to the default, exactly as a post-sunset state would. Flip back to false the
// moment it's re-enabled; the date-based sunset still applies underneath.
export const FABLE_DISABLED = true;
export function fableAvailable(): boolean {
  return !FABLE_DISABLED && Date.now() < FABLE_SUNSET_MS;
}

const MODEL_KEY = "rift.assistant.model";
const EFFORT_KEY = "rift.assistant.thinkingEffort";
const THINKING_KEY = "rift.assistant.thinkingEnabled";
const PERMISSION_KEY = "rift.assistant.permissionMode";

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

const PERMISSION_MODES: readonly PermissionMode[] = [
  "default", "acceptEdits", "plan", "auto", "bypassPermissions",
] as const;

/** Validate an untrusted string (e.g. a saved convo's model) into a ModelSel.
 *  Returns null for unknown values and for Fable past its sunset. */
export function asModelSel(v: unknown): ModelSel | null {
  if (typeof v !== "string" || !(MODEL_SELS as readonly string[]).includes(v)) return null;
  if (v === "claude-fable-5" && !fableAvailable()) return null;
  return v as ModelSel;
}

export function loadModel(ws?: string | null): ModelSel {
  try {
    if (typeof localStorage !== "undefined") {
      const k = wsKey(MODEL_KEY, ws);
      const v = (k ? localStorage.getItem(k) : null) ?? localStorage.getItem(MODEL_KEY);
      if (v && (MODEL_SELS as readonly string[]).includes(v)) {
        if (v === "claude-fable-5" && !fableAvailable()) return "opus"; // matches backend FABLE_FALLBACK_MODEL (turn.rs)
        return v as ModelSel;
      }
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
  if (model === "opus" || model.includes("opus") || model === "claude-fable-5") return "opus";
  return "sonnet";
}

export function loadEffort(ws?: string | null): ThinkingEffort {
  try {
    if (typeof localStorage !== "undefined") {
      const k = wsKey(EFFORT_KEY, ws);
      const v = (k ? localStorage.getItem(k) : null) ?? localStorage.getItem(EFFORT_KEY);
      if (v === "none" || v === "quick" || v === "smart" || v === "deep" || v === "ultra") return v;
    }
  } catch {
    /* SSR or storage disabled */
  }
  return "smart";
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

/** Extended-thinking master switch. Default on (current behavior). Per-workspace
 *  like effort — a `base::<root>` pin overrides the global baseline. */
export function loadThinkingEnabled(ws?: string | null): boolean {
  try {
    if (typeof localStorage !== "undefined") {
      const k = wsKey(THINKING_KEY, ws);
      const v = (k ? localStorage.getItem(k) : null) ?? localStorage.getItem(THINKING_KEY);
      if (v === "off") return false;
      if (v === "on") return true;
    }
  } catch {
    /* SSR or storage disabled */
  }
  return true;
}

export function saveThinkingEnabled(v: boolean, ws?: string | null) {
  try {
    if (typeof localStorage === "undefined") return;
    const k = wsKey(THINKING_KEY, ws);
    if (k) localStorage.setItem(k, v ? "on" : "off"); // per-workspace pin — never touches the global baseline
    else localStorage.setItem(THINKING_KEY, v ? "on" : "off"); // no workspace → set the baseline default
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

/** Display label for a shell command: drops leading `cd <path> &&`/`;` hops
 *  (the harness prefixes most commands with one, which made every rail row
 *  read `cd "C:/…`) and middle-truncates so the tail survives. */
export function shellLabel(cmd: string): string {
  let c = (cmd.split("\n")[0] ?? "").trim();
  for (let prev = ""; prev !== c; ) {
    prev = c;
    c = c.replace(/^cd\s+(?:"[^"]*"|'[^']*'|[^\s;&|]+)\s*(?:&&|;)\s*/, "").trim();
  }
  if (!c) c = (cmd.split("\n")[0] ?? "").trim();
  return c.length > 60 ? c.slice(0, 38) + "…" + c.slice(-21) : c;
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
      // Same for a forking skill once it's been promoted to a spawn (a matching
      // agentSpawn id exists); a non-forking skill has no spawn and still shows.
      if (AGENT_TOOL_NAMES.has(b.name) || agentSpawns.some((a) => a.id === b.id)) continue;
      if (b.name === "Bash") {
        const cmd = typeof b.input.command === "string" ? b.input.command : "";
        out.push({ id: b.id, kind: "shell", label: shellLabel(cmd) || "shell", sub: null, startedAt: b.startedAt ?? fallbackTs });
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

/** Compact token count for the live turn readout (Claude-Code style: "1.2k"). */
export function fmtTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 10_000) return `${Math.round(n / 1000)}k`;
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
  return String(n);
}

/** Effort tiers low→high — canonical order for clamping + ladder UIs. */
export const EFFORT_ORDER: readonly ThinkingEffort[] = [
  "none", "quick", "smart", "deep", "ultra",
] as const;

/** Highest effort tier each model honors server-side — the single source of
 *  truth for the capability ceiling. `MODEL_OPTIONS.maxEffort` (the picker's
 *  slider) and `clampEffort` (the value actually sent) both derive from this so
 *  they can't disagree. Opus/Fable reach `ultra` (xhigh + ultracode); Sonnet 4.6
 *  tops out at `smart` (high); Haiku rejects effort wholesale (`none`). Mirror
 *  the Sonnet ceiling in src-tauri/src/assistant/turn.rs. */
export const MODEL_MAX_EFFORT: Record<ModelSel, ThinkingEffort> = {
  opus: "ultra",
  "claude-opus-4-7": "ultra",
  "claude-fable-5": "ultra",
  sonnet: "smart",
  haiku: "none",
};

/** Clamp an effort tier to a model's ceiling. Pure. Fixes the slider hiding
 *  out-of-range stops while a stored pref still carried (e.g. an Opus workspace
 *  pinned to `ultra`, switched to Sonnet, kept sending xhigh). */
export function clampEffort(effort: ThinkingEffort, model: ModelSel): ThinkingEffort {
  const cap = MODEL_MAX_EFFORT[model] ?? "ultra";
  return EFFORT_ORDER.indexOf(effort) > EFFORT_ORDER.indexOf(cap) ? cap : effort;
}

/** Effort → CLI flag mapping. Must mirror src-tauri/src/assistant/turn.rs.
 *  Ladder: none→low · quick→medium · smart→high (API default) · deep→xhigh
 *  (Claude Code's own agentic default) · ultra→xhigh; ultra's autonomous-workflow
 *  behavior rides the separate `ultracode` settings key, set in turn.rs. The
 *  effort is clamped to the model's ceiling first, so an out-of-range tier can
 *  never emit a flag the model rejects. */
export function effortToFlag(
  effort: ThinkingEffort,
  model: ModelSel,
): "low" | "medium" | "high" | "xhigh" | null {
  if (model === "haiku") return null;
  const e = clampEffort(effort, model);
  if (e === "none") return "low";
  if (e === "quick") return "medium";
  if (e === "deep" || e === "ultra") return "xhigh";
  return "high"; // "smart" — the default tier
}

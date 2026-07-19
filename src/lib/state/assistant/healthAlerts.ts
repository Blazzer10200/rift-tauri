// Harness health alerts — turn-completion checks that surface problems (and
// background-tab completions) as toasts instead of waiting to be noticed on
// the Harness dashboard. Called from AssistantStore.handleTurnComplete via a
// thin thunk, mirroring the M8/M9 free-fn-over-store-ref pattern.

import { toast } from "../toast.svelte";
import { workspace } from "../workspace.svelte";
import type { AssistantStore, TabState } from "../assistant.svelte";
import type { TurnRecord } from "./types";

// One-shot-per-app-session latches for the health warnings — each fires as a
// hint once, not a nag (same pattern as send.ts's fableSunsetNoticed).
const warned = new Set<"deadWait" | "staleCache" | "toolErrors" | "mcpInit">();

// Context-nearly-full is latched PER CONVERSATION with hysteresis, not
// per-session — every long thread deserves its own heads-up, but only one
// until compaction actually drops usage back down (re-arm below 70%).
const CTX_WARN_PCT = 85;
const CTX_REARM_PCT = 70;
const ctxWarned = new Map<string, boolean>();

/** Pure threshold+hysteresis decision so the latch logic stays testable. */
export function ctxAlertTransition(pct: number, latched: boolean): { fire: boolean; latched: boolean } {
  if (latched) return { fire: false, latched: pct >= CTX_REARM_PCT };
  return pct >= CTX_WARN_PCT ? { fire: true, latched: true } : { fire: false, latched: false };
}

// MCP statuses that mean the rift server is genuinely dead. "pending" (still
// starting) and "disabled" (user turned it off deliberately) are healthy-ish;
// unknown future statuses are ignored rather than false-alarmed.
const BAD_MCP_STATUSES = new Set(["failed", "needs-auth", "disconnected"]);

/** The CLI's `system`/`init` frame (first frame of every turn) lists each MCP
 *  server's connection status. Only the `rift` server is toast-worthy — if it
 *  failed, the workspace/git tools are silently dead for the whole turn and
 *  the user would only discover it through failing tool calls. User-configured
 *  CLI servers (claude.ai connectors etc.) sit permanently `needs-auth` in
 *  Rift's headless subprocess — that's their normal state, not a problem, so
 *  they never toast. */
export function checkMcpInitHealth(env: { mcp_servers?: unknown }) {
  if (warned.has("mcpInit")) return;
  const servers = Array.isArray(env.mcp_servers)
    ? (env.mcp_servers as { name?: string; status?: string }[])
    : [];
  const riftDown = servers.some(
    (s) => s?.name === "rift" && typeof s.status === "string" && BAD_MCP_STATUSES.has(s.status),
  );
  if (!riftDown) return;
  warned.add("mcpInit");
  toast.push({
    severity: "danger",
    title: "Rift workspace tools failed to start",
    detail:
      "File, search, and git tools are unavailable in this chat — try a new tab; if it persists, restart Rift.",
  });
}

function tabTitle(tab: TabState): string {
  if (tab.convoTitle) return tab.convoTitle;
  const first = tab.messages.find((m) => m.role === "user");
  const text = first?.blocks
    .map((b) => (b.type === "text" ? b.text : ""))
    .join("")
    .trim()
    .replace(/\s+/g, " ");
  return text ? (text.length > 40 ? text.slice(0, 40) + "…" : text) : "Untitled chat";
}

function lastTurnFor(store: AssistantStore, convoId: string): TurnRecord | null {
  const turns = store.telemetry.turns;
  for (let i = turns.length - 1; i >= 0; i--) {
    if (turns[i].convoId === convoId) return turns[i];
  }
  return null;
}

/** An ask_user question has sat unanswered past the nudge window (the turn —
 *  and its CLI subprocess — is frozen on it). Toast unless the question is
 *  plausibly on-screen right now: app visible, chat workspace, active tab. */
export function askUserStaleNudge(store: AssistantStore, tab: TabState) {
  let convoId: string | undefined;
  for (const [id, t] of store.tabs) if (t === tab) { convoId = id; break; }
  const tabVisible = convoId ? store.panes.some(p => p.tabId === convoId) : false;
  const onScreen =
    typeof document !== "undefined" && !document.hidden &&
    workspace.activeId === "chat" && tabVisible;
  if (onScreen) return;
  toast.push({
    severity: "info",
    title: "Claude is waiting on your answer",
    detail: tabTitle(tab),
    action: convoId
      ? {
          label: "Answer",
          onClick: () => {
            workspace.setActive("chat");
            void store.openTab(convoId);
          },
        }
      : undefined,
  });
}

/** Post-turn health pass. `convoId` is the completed tab's Map key (resolved
 *  by handleTurnComplete's reverse lookup — cliSessionId can diverge from it
 *  post-compaction). */
export function checkTurnHealth(store: AssistantStore, tab: TabState, convoId: string | undefined) {
  const rec = convoId ? lastTurnFor(store, convoId) : null;

  // Context nearly full — the CLI will auto-compact mid-turn without warning
  // once it runs out of headroom. Saying so at 85% lets the user pick the
  // compaction point (a natural break) instead of having one forced on them.
  if (convoId) {
    const pct = store.ctxPctFor(tab);
    const t = ctxAlertTransition(pct, ctxWarned.get(convoId) ?? false);
    ctxWarned.set(convoId, t.latched);
    if (t.fire) {
      toast.push({
        severity: "warn",
        title: `Context is ${Math.round(pct)}% full`,
        detail: "Claude will auto-compact soon. Compacting at a natural break keeps the recap cleaner.",
        action: {
          label: "Compact",
          onClick: () => void store.send("/compact", convoId),
        },
      });
    }
  }

  // Background-tab completion — the user isn't looking at this tab, so the
  // outcome would otherwise be invisible until they switch back.
  const tabInAnyPane = convoId ? store.panes.some(p => p.tabId === convoId) : false;
  if (convoId && !tabInAnyPane) {
    const title = tabTitle(tab);
    const jump = {
      label: "View",
      onClick: () => {
        workspace.setActive("chat");
        void store.openTab(convoId);
      },
    };
    if (tab.lastError) {
      toast.push({ severity: "danger", title: "Background turn failed", detail: title, action: jump });
    } else {
      toast.push({ severity: "ok", title: "Background turn finished", detail: title, action: jump, timeoutMs: 6000 });
    }
  }

  if (!rec) return;

  // Silent pre-paint stall not attributable to thinking (spawn/prefill/queue).
  // The soft 8s notice is one-shot-per-session (a hint, not a nag); but an
  // egregious stall (>30s before first output) re-fires every time — a single
  // 138s API hang shouldn't be silenced just because a milder one warned earlier
  // in the session. Verified 2026-06-22: Rift's own TTFT is ~1s median, model
  // first-token ~4s median; a >30s deadWait is an API-side stall, worth saying.
  if (rec.firstPaintAt != null) {
    // Subtract only the thinking that happened BEFORE first paint — the snapshot,
    // not rec.thinkingTotalMs (which accumulates every thinking round for the
    // whole turn). Using the cumulative total let a long post-paint thinking
    // round drive deadWait negative and silently mask a genuinely slow start.
    const preThinking = rec.thinkingMsBeforeFirstPaint ?? rec.thinkingTotalMs;
    const deadWait = rec.firstPaintAt - rec.ts - preThinking;
    const egregious = deadWait > 30000;
    if (deadWait > 8000 && (egregious || !warned.has("deadWait"))) {
      warned.add("deadWait");
      toast.push({
        severity: "warn",
        title: "Slow turn start",
        detail: egregious
          ? `${Math.round(deadWait / 1000)}s before first output — the Anthropic API was slow (or silently retrying), not Rift.`
          : `${Math.round(deadWait / 1000)}s passed before first output (spawn/prefill/queue stall).`,
      });
    }
  }

  // Continuation turn paid full cache_create with zero cache_read — the
  // prompt cache was busted (model/effort flip, >5min idle, …).
  if (!warned.has("staleCache") && !rec.isFirstTurn && rec.resultUsage) {
    const u = rec.resultUsage;
    if (u.cacheRead === 0 && u.cacheCreate > 0) {
      warned.add("staleCache");
      toast.push({
        severity: "warn",
        title: "Prompt cache miss",
        detail: "This turn rebuilt the cache from scratch — repeated misses cost real money.",
      });
    }
  }

  if (!warned.has("toolErrors")) {
    const errs = rec.toolUses.filter((t) => t.isError === true).length;
    if (errs >= 3) {
      warned.add("toolErrors");
      toast.push({
        severity: "warn",
        title: "Tools failing repeatedly",
        detail: `${errs} tool calls errored in one turn — check the chat's tool chips for details.`,
      });
    }
  }
}

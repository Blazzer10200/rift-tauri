// /mcp status merge + display meta — pure fns, colocated vitest (mcpStatus.test.ts).
//
// Two truth layers feed the /mcp dialog: `claude mcp list` via the backend
// (the user's real harness config — user scope + project .mcp.json —
// health-checked at /mcp time, works before any turn) and the current tab's
// init frame (what THIS chat actually loaded). They can honestly disagree: a
// claude.ai connector is "connected" in a terminal but sits needs-auth inside
// Rift's headless subprocess. The session status wins per name; the harness
// list supplies the roster and statuses for everything the chat hasn't seen.

/** Row shape of the `list_mcp_servers` command (mcp_list.rs McpListRow). */
export type HarnessMcpRow = {
  name: string;
  target: string;
  transport: string | null;
  status: string;
  detail: string | null;
};

/** Per-tab init-frame row (streaming.ts → tab.mcpServers). */
export type SessionMcpRow = { name: string; status: string };

export type MergedMcpRow = {
  name: string;
  status: string;
  /** true when the status came from this chat's live init frame. */
  live: boolean;
  /** URL / spawn command from the harness list; null for session-only rows. */
  target: string | null;
  transport: string | null;
  detail: string | null;
};

export function mergeMcpRows(
  harness: HarnessMcpRow[],
  session: SessionMcpRow[] | null | undefined,
): MergedMcpRow[] {
  const rows: MergedMcpRow[] = harness.map((h) => ({
    name: h.name,
    status: h.status,
    live: false,
    target: h.target,
    transport: h.transport,
    detail: h.detail,
  }));
  const byName = new Map(rows.map((r) => [r.name, r]));
  for (const s of session ?? []) {
    const hit = byName.get(s.name);
    if (hit) {
      hit.status = s.status;
      hit.live = true;
    } else {
      // Session-only servers: rift's own workspace server (injected per-turn
      // via --mcp-config, never in the user's config files). Listed first —
      // it's the one Rift can't work without.
      rows.unshift({
        name: s.name,
        status: s.status,
        live: true,
        target: null,
        transport: null,
        detail: s.name === "rift" ? "Rift's workspace tools — files, search, git" : null,
      });
    }
  }
  return rows;
}

// Status vocabulary spans both layers: connected/needs-auth/needs-approval/
// failed/unknown from mcp_list.rs, connected/needs-auth/pending/disabled/
// failed from init frames. Unmapped future statuses render as "Configured".
const STATUS_META: Record<string, { label: string; tint: "ok" | "warn" | "danger" | "muted" }> = {
  connected: { label: "Connected", tint: "ok" },
  "needs-auth": { label: "Needs sign-in", tint: "warn" },
  "needs-approval": { label: "Needs approval", tint: "warn" },
  pending: { label: "Starting…", tint: "muted" },
  disabled: { label: "Disabled", tint: "muted" },
  failed: { label: "Failed", tint: "danger" },
};

export function statusMeta(status: string): { label: string; tint: "ok" | "warn" | "danger" | "muted" } {
  return STATUS_META[status] ?? { label: "Configured", tint: "muted" };
}

/** One contextual footer hint, worst problem first. */
export function mcpHint(rows: MergedMcpRow[]): string | null {
  if (rows.some((r) => r.status === "failed")) {
    return "A server failed its health check — check its command or URL, then re-check.";
  }
  if (rows.some((r) => r.status === "needs-approval")) {
    return "Project servers need a one-time approval — run `claude` in that folder once.";
  }
  if (rows.some((r) => r.status === "needs-auth")) {
    return "Sign-in happens in a terminal `claude` session (or claude.ai connector settings).";
  }
  return null;
}

// /mcp dialog state. Opened by the /mcp slash command (send.ts) and the
// dialog's own Re-check button; Esc / outside-click closes. Fetch lives here
// (invoke + merge) but takes the assistant-store inputs as ARGS — importing
// the assistant store from this module would close a cycle through send.ts.

import { invoke } from "@tauri-apps/api/core";
import {
  mergeMcpRows,
  type HarnessMcpRow,
  type MergedMcpRow,
  type SessionMcpRow,
} from "./assistant/mcpStatus";

class McpPanelState {
  open = $state(false);
  loading = $state(false);
  rows = $state<MergedMcpRow[] | null>(null);
  error = $state<string | null>(null);
  checkedAt = $state<number | null>(null);
  // Stale-response guard: only the newest in-flight check may land.
  #seq = 0;

  show() {
    this.open = true;
  }
  hide() {
    this.open = false;
  }

  /** Run `claude mcp list` via the backend and merge with the chat's
   *  init-frame view. Session rows still render if the CLI check fails. */
  async refresh(root: string | null, session: SessionMcpRow[] | null) {
    const seq = ++this.#seq;
    this.loading = true;
    this.error = null;
    try {
      const harness = await invoke<HarnessMcpRow[]>("list_mcp_servers", { root });
      if (seq !== this.#seq) return;
      this.rows = mergeMcpRows(harness, session);
      this.checkedAt = Date.now();
    } catch (e) {
      if (seq !== this.#seq) return;
      this.rows = session && session.length > 0 ? mergeMcpRows([], session) : null;
      this.error = String(e);
    } finally {
      if (seq === this.#seq) this.loading = false;
    }
  }
}

export const mcpPanel = new McpPanelState();

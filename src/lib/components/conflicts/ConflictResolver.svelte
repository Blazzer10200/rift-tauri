<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { connection, type ConflictRecord } from "../../state/connection.svelte";

  type Props = { conflict: ConflictRecord };
  let { conflict }: Props = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);
  let info = $state<string | null>(null);

  function fmtSize(n: number): string {
    if (n < 0) return "—";
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
    return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }
  function fmtMtime(s: string): string {
    if (!s) return "—";
    try { return new Date(s).toLocaleString(); } catch { return s; }
  }

  async function resolve(resolution: "skip" | "save_local_copy" | "force_local" | "accept_remote") {
    busy = true; error = null; info = null;
    try {
      await invoke("resolve_conflict", {
        localPath: conflict.local_path,
        resolution,
      });
      info = `Resolved (${resolution.replace("_", " ")}).`;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function editInPlace() {
    const s = connection.selected;
    if (!s) { error = "No server selected."; return; }
    busy = true; error = null; info = null;
    try {
      const localTmp = await invoke<string>("begin_edit_in_place", {
        serverKey: s.key,
        remotePath: conflict.remote_path,
      });
      await invoke("plugin:opener|open_path", { path: localTmp }).catch(() => {});
      info = `Editing ${localTmp}. Save in your editor; reupload from the watch list.`;
      await connection.refreshWatchedEdits();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="resolver">
  <header>
    <h2>{conflict.resource_name}</h2>
    <code class="path">{conflict.local_path}</code>
  </header>

  <div class="split">
    <div class="side local">
      <h3>Local</h3>
      <dl>
        <dt>Size</dt><dd>{fmtSize(conflict.local_size)}</dd>
        <dt>Modified</dt><dd>{fmtMtime(conflict.local_mtime_utc)}</dd>
        <dt>Path</dt><dd class="mono">{conflict.local_path}</dd>
      </dl>
    </div>
    <div class="side remote">
      <h3>Remote</h3>
      <dl>
        <dt>Size</dt><dd>{fmtSize(conflict.remote_size)}</dd>
        <dt>Modified</dt><dd>{fmtMtime(conflict.remote_mtime_utc)}</dd>
        <dt>Path</dt><dd class="mono">{conflict.remote_path}</dd>
      </dl>
    </div>
  </div>

  <div class="lastknown">
    <span class="label">Last known sync:</span>
    {fmtSize(conflict.last_known_size)} · {fmtMtime(conflict.last_known_mtime_utc)}
  </div>

  {#if error}<div class="err">{error}</div>{/if}
  {#if info}<div class="ok">{info}</div>{/if}

  <div class="actions">
    <button type="button" disabled={busy} onclick={() => resolve("skip")}>Skip</button>
    <button
      type="button"
      class="violet"
      disabled={busy}
      onclick={() => resolve("force_local")}
      title="Push your local file, overwriting the remote version"
    >Take local</button>
    <button
      type="button"
      class="amber"
      disabled={busy}
      onclick={() => resolve("accept_remote")}
      title="Pull the remote version, overwriting your local edit"
    >Take remote</button>
    <button
      type="button"
      disabled={busy}
      onclick={() => resolve("save_local_copy")}
      title="Save a side-copy of local then pull remote"
    >Save copy + pull</button>
    <button
      type="button"
      class="edit"
      disabled={busy}
      onclick={editInPlace}
      title="Open the remote file in your editor — Rift watches saves"
    >Edit in place</button>
  </div>
</section>

<style>
  .resolver {
    flex: 1; min-width: 0; min-height: 0;
    background: #0F0F12; color: #E8E8EE;
    padding: 16px 20px;
    overflow: auto;
    display: flex; flex-direction: column; gap: 12px;
  }
  header h2 { margin: 0 0 4px; font-size: 16px; color: #FF5C6B; }
  .path { color: #7A7A85; font-family: Consolas, monospace; font-size: 11px; }
  .split {
    display: grid; grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .side {
    background: #17171C; border: 1px solid #26262E;
    border-radius: 4px; padding: 12px;
  }
  .side.local { border-color: #8B6BE6; }
  .side.remote { border-color: #F0B95C; }
  .side h3 { margin: 0 0 8px; font-size: 13px; }
  .side.local h3 { color: #8B6BE6; }
  .side.remote h3 { color: #F0B95C; }
  dl {
    display: grid; grid-template-columns: 90px 1fr;
    gap: 4px 12px; margin: 0; font-size: 12px;
  }
  dt { color: #7A7A85; }
  dd { margin: 0; color: #E8E8EE; word-break: break-all; }
  dd.mono { font-family: Consolas, monospace; font-size: 11px; }
  .lastknown {
    background: #17171C; padding: 8px 12px; border-radius: 3px;
    font-size: 12px; color: #7A7A85;
  }
  .lastknown .label { color: #E8E8EE; font-weight: 600; margin-right: 6px; }
  .err {
    color: #FF5C6B; background: #15101E; padding: 8px 12px;
    border-radius: 3px; font-family: Consolas, monospace; font-size: 12px;
  }
  .ok {
    color: #4ADE80; background: #15101E; padding: 8px 12px;
    border-radius: 3px; font-size: 12px;
  }
  .actions {
    display: flex; gap: 8px; flex-wrap: wrap;
    padding-top: 4px;
  }
  .actions button {
    background: #17171C; color: #E8E8EE;
    border: 1px solid #26262E; border-radius: 3px;
    padding: 8px 14px; font-size: 12px; cursor: pointer;
  }
  .actions button:hover:not(:disabled) { border-color: #8B6BE6; }
  .actions button:disabled { opacity: 0.4; cursor: not-allowed; }
  .actions .violet { border-color: #8B6BE6; color: #8B6BE6; font-weight: 600; }
  .actions .amber  { border-color: #F0B95C; color: #F0B95C; font-weight: 600; }
  .actions .edit   { border-color: #4ADE80; color: #4ADE80; font-weight: 600; }
</style>

<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import {
    AppWindow, Server, Copy, Terminal, Check, AlertTriangle,
    FileX2, Files,
  } from "lucide-svelte";
  import { connection, type ConflictRecord } from "../../state/connection.svelte";

  type Props = { conflict: ConflictRecord };
  let { conflict }: Props = $props();

  type Pick = "local" | "remote" | "save_copy" | null;
  let pick = $state<Pick>(null);
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
    try { return new Date(s).toLocaleString([], { hour12: true }); } catch { return s; }
  }
  function basename(p: string): string {
    const norm = p.replaceAll("\\", "/").replace(/\/+$/, "");
    const i = norm.lastIndexOf("/");
    return i === -1 ? norm : norm.slice(i + 1);
  }

  const sizeDelta = $derived(conflict.local_size - conflict.remote_size);
  const localLarger = $derived(sizeDelta > 0);
  const remoteLarger = $derived(sizeDelta < 0);

  async function applyPick() {
    if (!pick) return;
    const map: Record<Exclude<Pick, null>, string> = {
      local: "force_local",
      remote: "accept_remote",
      save_copy: "save_local_copy",
    };
    busy = true; error = null; info = null;
    try {
      await invoke("resolve_conflict", {
        localPath: conflict.local_path,
        resolution: map[pick],
      });
      info = `Resolved (${map[pick].replaceAll("_", " ")}).`;
    } catch (e) {
      error = String(e);
    } finally { busy = false; }
  }

  async function skipFile() {
    busy = true; error = null; info = null;
    try {
      await invoke("resolve_conflict", { localPath: conflict.local_path, resolution: "skip" });
      info = "Skipped — leaves both files alone.";
    } catch (e) { error = String(e); }
    finally { busy = false; }
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
      try {
        const { openPath } = await import("@tauri-apps/plugin-opener");
        await openPath(localTmp);
      } catch (openErr) {
        // Audit M12: typed import + surface failure rather than swallow.
        error = `Couldn't launch editor: ${String(openErr)}`;
      }
      info = `Editing ${localTmp}. Save in your editor; reupload from the watch list.`;
      await connection.refreshWatchedEdits();
    } catch (e) { error = String(e); }
    finally { busy = false; }
  }

  async function copyPath() {
    try { await navigator.clipboard.writeText(conflict.local_path); info = "Path copied."; }
    catch { error = "Clipboard unavailable."; }
  }
</script>

<section class="resolver">
  <header class="head">
    <div class="head-l">
      <div class="path mono">{basename(conflict.local_path)}</div>
      <div class="help">
        Both sides modified since last sync · resource <span class="mono">{conflict.resource_name}</span>
      </div>
    </div>
    <div class="head-r">
      <button class="btn ghost sm info" type="button" onclick={editInPlace} disabled={busy}>
        <Terminal size={11}/> Editor
      </button>
      <button class="btn ghost sm" type="button" onclick={copyPath}>
        <Copy size={11}/> Path
      </button>
    </div>
  </header>

  <div class="meta">
    <button
      class="meta-side"
      data-side="local"
      data-pick={pick === "local"}
      type="button"
      onclick={() => (pick = "local")}
    >
      <div class="meta-side-l">
        <AppWindow size={13} class="side-ico"/>
        <span class="side-label">Local</span>
        {#if pick === "local"}<span class="pick-pip"><Check size={10}/> picked</span>{/if}
      </div>
      <div class="meta-side-r mono">
        <span class="sz" class:bigger={localLarger}>{fmtSize(conflict.local_size)}</span>
        <span class="dim">·</span>
        <span>{fmtMtime(conflict.local_mtime_utc)}</span>
      </div>
    </button>
    <button
      class="meta-side"
      data-side="remote"
      data-pick={pick === "remote"}
      type="button"
      onclick={() => (pick = "remote")}
    >
      <div class="meta-side-l">
        <Server size={13} class="side-ico"/>
        <span class="side-label">Remote</span>
        {#if pick === "remote"}<span class="pick-pip"><Check size={10}/> picked</span>{/if}
      </div>
      <div class="meta-side-r mono">
        <span class="sz" class:bigger={remoteLarger}>{fmtSize(conflict.remote_size)}</span>
        <span class="dim">·</span>
        <span>{fmtMtime(conflict.remote_mtime_utc)}</span>
      </div>
    </button>
  </div>

  <div class="hunks-placeholder">
    <div class="ph-head">
      <span class="ph-title">Diff peek</span>
      <span class="help">Per-hunk merge ships in a future backend update — for now, pick a side or save a copy.</span>
    </div>
    <div class="diff-summary mono">
      <div class="ds-row">
        <span class="ds-k">size delta</span>
        <span class="ds-v" data-delta={sizeDelta > 0 ? "plus" : sizeDelta < 0 ? "minus" : "zero"}>
          {sizeDelta > 0 ? `+${fmtSize(sizeDelta)} (local larger)` : sizeDelta < 0 ? `−${fmtSize(-sizeDelta)} (remote larger)` : "identical size"}
        </span>
      </div>
      <div class="ds-row">
        <span class="ds-k">last sync</span>
        <span class="ds-v">{fmtSize(conflict.last_known_size)} · {fmtMtime(conflict.last_known_mtime_utc)}</span>
      </div>
      <div class="ds-row">
        <span class="ds-k">paths</span>
        <span class="ds-v">L <span class="dim">{conflict.local_path}</span></span>
      </div>
      <div class="ds-row">
        <span class="ds-k"></span>
        <span class="ds-v">R <span class="dim">{conflict.remote_path}</span></span>
      </div>
    </div>
  </div>

  {#if info}<div class="banner ok"><Check size={11}/> {info}</div>{/if}
  {#if error}<div class="banner err"><AlertTriangle size={11}/> {error}</div>{/if}

  <div class="actions">
    <div class="actions-l">
      <button class="btn ghost sm" type="button" onclick={skipFile} disabled={busy}>
        <FileX2 size={11}/> Skip file
      </button>
      <button
        class="btn sm"
        type="button"
        onclick={() => (pick = "save_copy")}
        data-active={pick === "save_copy"}
        disabled={busy}
      >
        <Files size={11}/> Save copy + pull remote
      </button>
    </div>
    <div class="actions-r">
      <button
        class="btn primary sm"
        type="button"
        onclick={applyPick}
        disabled={busy || !pick}
      >
        <Check size={11}/>
        {#if pick === "local"}Apply: Take local
        {:else if pick === "remote"}Apply: Take remote
        {:else if pick === "save_copy"}Apply: Save copy + pull
        {:else}Pick a side
        {/if}
      </button>
    </div>
  </div>
</section>

<style>
  .resolver {
    flex: 1; min-width: 0; min-height: 0;
    background: var(--bg);
    color: var(--fg);
    padding: 14px 18px;
    overflow: auto;
    display: flex; flex-direction: column;
    gap: 12px;
  }
  .head {
    display: flex; justify-content: space-between; align-items: end;
    gap: 12px;
  }
  .path { font-size: var(--fs-md); color: var(--fg); font-weight: 600; }
  .help { color: var(--fg-muted); font-size: var(--fs-xs); margin-top: 2px; }
  .head-r { display: inline-flex; gap: 6px; }

  .meta {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .meta-side {
    display: flex; flex-direction: column; gap: 6px;
    padding: 12px 14px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    cursor: pointer;
    text-align: left;
    font: inherit;
    color: var(--fg);
    box-shadow: inset 2px 0 transparent;
    transition: border-color 100ms, background 100ms, box-shadow 100ms;
  }
  .meta-side:hover { background: var(--surface-hover); }
  .meta-side[data-side="local"][data-pick="true"] {
    border-color: color-mix(in oklch, var(--info) 45%, var(--border));
    background: var(--info-soft);
    box-shadow: inset 2px 0 var(--info);
  }
  .meta-side[data-side="remote"][data-pick="true"] {
    border-color: color-mix(in oklch, var(--warn) 45%, var(--border));
    background: var(--warn-soft);
    box-shadow: inset 2px 0 var(--warn);
  }
  .meta-side-l { display: inline-flex; align-items: center; gap: 8px; }
  .meta-side-r { font-size: var(--fs-xs); color: var(--fg-2); display: inline-flex; gap: 6px; align-items: center; flex-wrap: wrap; }
  .meta-side :global(.side-ico) {
    color: var(--fg-muted);
    transition: transform 140ms cubic-bezier(0.34, 1.56, 0.64, 1), color 100ms;
  }
  .meta-side[data-side="local"][data-pick="true"] :global(.side-ico) { color: var(--info); }
  .meta-side[data-side="remote"][data-pick="true"] :global(.side-ico) { color: var(--warn); }
  @media (prefers-reduced-motion: no-preference) {
    .meta-side:hover :global(.side-ico) { transform: scale(1.15); }
  }
  .side-label { color: var(--fg); font-size: var(--fs-sm); font-weight: 600; }
  .sz.bigger { color: var(--fg); font-weight: 600; }
  .pick-pip {
    margin-left: 6px;
    font-size: var(--fs-xs);
    display: inline-flex; align-items: center; gap: 3px;
    padding: 1px 6px;
    border-radius: 999px;
    background: color-mix(in oklch, currentColor 14%, transparent);
    color: inherit;
  }
  .meta-side[data-side="local"][data-pick="true"] .pick-pip { color: var(--info); }
  .meta-side[data-side="remote"][data-pick="true"] .pick-pip { color: var(--warn); }

  .hunks-placeholder {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
  }
  .ph-head { display: flex; align-items: center; gap: 10px; margin-bottom: 8px; }
  .ph-title { font-size: var(--fs-sm); font-weight: 600; color: var(--fg); }

  .diff-summary {
    display: flex; flex-direction: column; gap: 4px;
    font-size: var(--fs-xs);
  }
  .ds-row { display: grid; grid-template-columns: 90px 1fr; gap: 8px; align-items: start; }
  .ds-k { color: var(--fg-faint); text-transform: uppercase; letter-spacing: 0.05em; }
  .ds-v { color: var(--fg-2); word-break: break-all; }
  .ds-v[data-delta="plus"] { color: var(--info); }
  .ds-v[data-delta="minus"] { color: var(--warn); }
  .dim { color: var(--fg-faint); }

  .banner {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    font-size: var(--fs-xs);
  }
  .banner.ok { background: var(--ok-soft); color: var(--ok); border: 1px solid color-mix(in oklch, var(--ok) 30%, transparent); }
  .banner.err { background: var(--danger-soft); color: var(--danger); border: 1px solid color-mix(in oklch, var(--danger) 30%, transparent); }

  .actions {
    display: flex; align-items: center; justify-content: space-between;
    gap: 8px;
    padding-top: 6px;
    border-top: 1px solid var(--border);
  }
  .actions-l, .actions-r { display: inline-flex; gap: 6px; }

  :global(.btn[data-active="true"]) {
    border-color: var(--accent);
    color: var(--accent);
  }
</style>

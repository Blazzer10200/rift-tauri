<script lang="ts">
  import { FolderOpen, Maximize2, ServerCog } from "lucide-svelte";
  import type { PanelIcon } from "./index";
  import TwoPane from "../../browser/TwoPane.svelte";
  import { uiPrefs } from "../../../state/ui-prefs.svelte";
  import { browserTabs } from "../../../state/browser-tabs.svelte";
  import { connection } from "../../../state/connection.svelte";

  // PanelShell passes title + icon for the registry contract; TwoPane owns
  // its own toolbar / tabstrip and branches on uiPrefs.useV03Shell.
  let { }: { title: string; icon: PanelIcon } = $props();

  // Phase C: while NOT maximized, the dock-hosted Files panel is too narrow
  // (320px default dock width) to render the full LocalPane+RemotePane split.
  // Show a compact summary instead w/ a one-click "View files" affordance
  // that maximizes into the main column. When maximized, render full TwoPane.
  const isMaximized = $derived(uiPrefs.maximized === "files");
  const active = $derived(browserTabs.active);
  const tabCount = $derived(browserTabs.tabs.length);
  const serverName = $derived(connection.selected?.name ?? null);

  function basename(p: string | undefined | null): string {
    if (!p) return "";
    const n = p.replaceAll("\\", "/").replace(/\/+$/, "");
    const i = n.lastIndexOf("/");
    return i === -1 ? n : n.slice(i + 1);
  }
</script>

<div class="wrap">
  {#if isMaximized}
    <TwoPane />
  {:else}
    <div class="summary">
      <div class="row">
        <span class="lbl"><FolderOpen size={12}/> Local</span>
        <span class="path mono" title={active?.localPath ?? "—"}>
          {basename(active?.localPath) || "—"}
        </span>
      </div>
      <div class="row">
        <span class="lbl"><ServerCog size={12}/> Remote</span>
        <span class="path mono" title={active?.remotePath ?? "—"}>
          {#if serverName}{serverName} · {/if}{basename(active?.remotePath) || "/"}
        </span>
      </div>
      <div class="foot">
        <span class="meta">{tabCount} {tabCount === 1 ? "tab" : "tabs"}</span>
        <button
          type="button"
          class="view-btn"
          onclick={() => uiPrefs.maximizePanel("files")}
          title="Maximize Files into the main column"
        >
          <Maximize2 size={11}/>
          <span>Open</span>
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .wrap {
    display: flex; flex-direction: column;
    flex: 1; min-height: 0;
    height: 100%;
  }
  .summary {
    display: flex; flex-direction: column; gap: 6px;
    padding: 10px 12px 12px;
  }
  .row {
    display: flex; align-items: center; gap: 8px;
    font-size: var(--fs-sm);
    color: var(--fg-muted);
    min-width: 0;
  }
  .lbl {
    display: inline-flex; align-items: center; gap: 6px;
    color: var(--fg-muted);
    width: 60px;
    flex-shrink: 0;
    font-size: var(--fs-xs);
  }
  .path {
    flex: 1; min-width: 0;
    color: var(--fg);
    font-size: var(--fs-xs);
    background: var(--bg-elev-2);
    padding: 3px 7px;
    border-radius: var(--radius-xs);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .foot {
    display: flex; align-items: center; justify-content: space-between;
    margin-top: 4px;
  }
  .meta {
    font-size: var(--fs-xs);
    color: var(--fg-faint);
  }
  .view-btn {
    display: inline-flex; align-items: center; gap: 6px;
    height: 24px;
    padding: 0 10px;
    background: transparent;
    border: 1px solid var(--border-strong);
    color: var(--fg-2);
    border-radius: var(--radius-xs);
    cursor: pointer;
    font: inherit;
    font-size: var(--fs-xs);
    transition: background 120ms ease, border-color 120ms ease, color 120ms ease;
  }
  .view-btn:hover {
    background: color-mix(in oklch, var(--accent) 14%, transparent);
    border-color: color-mix(in oklch, var(--accent) 55%, var(--border-strong));
    color: var(--fg);
  }
</style>

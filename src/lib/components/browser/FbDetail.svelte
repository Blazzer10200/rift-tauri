<script lang="ts">
  import { FileText, Folder, Lock, Copy, AlertTriangle } from "lucide-svelte";
  import { fade } from "svelte/transition";
  import { fmtSize } from "../../utils/file-display";
  import { tooltip } from "$lib/actions/tooltip";

  export type DetailEntry = {
    side: "local" | "remote";
    name: string;
    path: string;
    is_dir: boolean;
    size: number;
    modifiedLabel: string;
    modifiedAbs: string;
    status: "synced" | "modified" | "conflict";
    locked: boolean;
    lockedBy?: string | null;
  };

  let {
    entry = null,
    onGoToConflicts,
  }: { entry?: DetailEntry | null; onGoToConflicts?: () => void } = $props();

  const ext = $derived(() => {
    if (!entry || entry.is_dir) return "";
    const i = entry.name.lastIndexOf(".");
    return i > 0 ? entry.name.slice(i + 1).toUpperCase() : "";
  });

  const statusLabel: Record<DetailEntry["status"], string> = {
    synced: "Synced",
    modified: "Modified",
    conflict: "Conflict",
  };

  async function copyPath() {
    if (!entry) return;
    try { await navigator.clipboard.writeText(entry.path); } catch (e) { console.warn("clipboard failed", e); }
  }
</script>

<aside class="fb-detail">
  {#if !entry}
    <div class="fb-empty">
      <div class="fb-empty-glyph"><FileText size={22} /></div>
      <div class="fb-empty-t">No file selected</div>
      <div class="fb-empty-s">Select a file in either pane to see its sync status, size, and last-modified time.</div>
    </div>
  {:else}
    {#key entry.path}
      <div class="fb-d-inner" in:fade={{ duration: 130 }}>
        <div class="fb-d-head">
          <div class="fb-d-crumb mono" use:tooltip={entry.path}>{entry.path}</div>
          <div class="fb-d-titlerow">
            <div class="fb-d-glyph">
              {#if entry.is_dir}<Folder size={16} />{:else}<FileText size={16} />{/if}
            </div>
            <div class="fb-d-name mono">{entry.name}</div>
            {#if ext()}<span class="fb-ext-badge">{ext()}</span>{/if}
            <div class="fb-d-spacer"></div>
            <span class="fb-status-pill sp-{entry.status}">
              <span class="dot"></span>{statusLabel[entry.status]}
            </span>
          </div>
          <div class="fb-meta">
            <div class="fb-meta-item">
              <span class="fb-meta-k">Size</span>
              <span class="fb-meta-v mono">{entry.is_dir ? "Folder" : fmtSize(entry.size, false)}</span>
            </div>
            <div class="fb-meta-item">
              <span class="fb-meta-k">Modified</span>
              <span class="fb-meta-v" use:tooltip={entry.modifiedAbs}>{entry.modifiedLabel}</span>
            </div>
            <div class="fb-meta-item">
              <span class="fb-meta-k">Location</span>
              <span class="fb-meta-v">{entry.side === "local" ? "Local" : "Remote"}</span>
            </div>
            {#if entry.locked}
              <div class="fb-meta-item">
                <span class="fb-meta-k">Locked</span>
                <span class="fb-meta-v lock"><Lock size={11} />{entry.lockedBy ?? "remote"}</span>
              </div>
            {/if}
          </div>
        </div>

        {#if entry.status === "conflict"}
          <div class="fb-conf-banner">
            <AlertTriangle size={15} class="fb-cb-ic" />
            <span class="fb-cb-text">This file has a sync <b>conflict</b>.</span>
            <button type="button" class="fb-cb-go" onclick={() => onGoToConflicts?.()}>Resolve in Sync ↗</button>
          </div>
        {/if}

        <div class="fb-d-foot">
          <button type="button" class="fb-act" onclick={copyPath}>
            <Copy size={13} /> Copy path
          </button>
        </div>
      </div>
    {/key}
  {/if}
</aside>

<style>
  .fb-detail {
    flex: none;
    width: 340px;
    min-height: 0;
    display: flex; flex-direction: column;
    background: var(--bg);
    border-left: 1px solid var(--border);
  }
  .fb-d-inner { display: flex; flex-direction: column; min-height: 0; flex: 1; }

  /* ── empty ─────────────────────────────────────────────── */
  .fb-empty {
    flex: 1;
    display: flex; flex-direction: column; align-items: center; justify-content: center;
    gap: 12px; padding: 40px 28px; text-align: center;
    color: var(--fg-muted);
  }
  .fb-empty-glyph {
    width: 52px; height: 52px; border-radius: 14px;
    display: grid; place-items: center;
    background: var(--accent-soft); color: var(--accent);
    box-shadow: 0 0 0 1px color-mix(in oklch, var(--accent) 28%, transparent);
  }
  .fb-empty-t { font-size: var(--fs-lg); font-weight: 650; color: var(--fg); }
  .fb-empty-s { font-size: var(--fs-sm); max-width: 260px; line-height: 1.5; }

  /* ── head ──────────────────────────────────────────────── */
  .fb-d-head { flex: none; padding: 16px 18px 14px; border-bottom: 1px solid var(--border); }
  .fb-d-crumb {
    font-size: 11px; color: var(--fg-subtle); margin-bottom: 10px;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap; direction: rtl; text-align: left;
  }
  .fb-d-titlerow { display: flex; align-items: center; gap: 9px; }
  .fb-d-glyph { color: var(--accent); display: inline-flex; flex: none; }
  .fb-d-name {
    font-size: 16px; font-weight: 700; letter-spacing: -0.01em;
    min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .fb-ext-badge {
    font-family: var(--font-mono); font-size: 10px; font-weight: 700; letter-spacing: 0.06em;
    padding: 2px 7px; border-radius: 5px; background: var(--bg-inset);
    border: 1px solid var(--border); color: var(--fg-muted); flex: none;
  }
  .fb-d-spacer { flex: 1; }
  .fb-status-pill {
    display: inline-flex; align-items: center; gap: 7px; flex: none;
    height: 24px; padding: 0 11px; border-radius: 999px;
    font-size: var(--fs-xs); font-weight: 650; border: 1px solid transparent;
  }
  .fb-status-pill .dot { width: 7px; height: 7px; border-radius: 999px; }
  .sp-synced { background: var(--surface); border-color: var(--border); color: var(--fg-muted); }
  .sp-synced .dot { background: var(--fg-faint); }
  .sp-modified { background: var(--accent-soft); border-color: color-mix(in oklch, var(--accent) 30%, transparent); color: var(--accent); }
  .sp-modified .dot { background: var(--accent); }
  .sp-conflict { background: var(--warn-soft); border-color: color-mix(in oklch, var(--warn) 32%, transparent); color: var(--warn); }
  .sp-conflict .dot { background: var(--warn); }

  /* ── meta strip ────────────────────────────────────────── */
  .fb-meta { display: flex; align-items: stretch; flex-wrap: wrap; gap: 0; margin-top: 16px; }
  .fb-meta-item { display: flex; flex-direction: column; gap: 3px; padding: 0 16px; }
  .fb-meta-item:first-child { padding-left: 0; }
  .fb-meta-item + .fb-meta-item { border-left: 1px solid var(--border); }
  .fb-meta-k { font-size: 10px; font-weight: 600; letter-spacing: 0.05em; text-transform: uppercase; color: var(--fg-subtle); }
  .fb-meta-v { font-size: var(--fs-sm); font-weight: 600; color: var(--fg-2); }
  .fb-meta-v.mono { font-family: var(--font-mono); }
  .fb-meta-v.lock { display: inline-flex; align-items: center; gap: 5px; color: var(--warn); }

  /* ── conflict banner ───────────────────────────────────── */
  .fb-conf-banner {
    display: flex; align-items: center; gap: 10px;
    margin: 14px 18px 0; padding: 10px 12px;
    background: var(--warn-soft);
    border: 1px solid color-mix(in oklch, var(--warn) 30%, transparent);
    border-radius: var(--r-card); color: var(--warn);
  }
  .fb-conf-banner :global(.fb-cb-ic) { flex: none; }
  .fb-cb-text { flex: 1; font-size: var(--fs-sm); font-weight: 600; }
  .fb-cb-text b { color: var(--fg); }
  .fb-cb-go {
    flex: none; display: inline-flex; align-items: center; height: 26px; padding: 0 11px;
    border-radius: var(--radius-sm); border: 1px solid color-mix(in oklch, var(--warn) 36%, transparent);
    background: color-mix(in oklch, var(--warn) 14%, transparent); color: var(--warn);
    font: inherit; font-size: var(--fs-xs); font-weight: 650; cursor: pointer;
    transition: background 120ms var(--ease-soft);
  }
  .fb-cb-go:hover { background: color-mix(in oklch, var(--warn) 22%, transparent); }

  /* ── footer ────────────────────────────────────────────── */
  .fb-d-foot {
    margin-top: auto; flex: none;
    display: flex; align-items: center; gap: 10px;
    padding: 12px 18px; border-top: 1px solid var(--border);
  }
  .fb-act {
    display: inline-flex; align-items: center; gap: 8px;
    height: 32px; padding: 0 13px; border-radius: var(--radius-sm);
    border: 1px solid var(--border); background: var(--surface); color: var(--fg-2);
    font: inherit; font-size: var(--fs-sm); font-weight: 600; cursor: pointer;
    transition: background 120ms var(--ease-soft), border-color 120ms var(--ease-soft), color 120ms var(--ease-soft);
  }
  .fb-act:hover { background: var(--surface-hover); border-color: var(--border-strong); color: var(--fg); }
</style>

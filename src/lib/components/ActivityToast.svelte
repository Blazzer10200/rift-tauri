<script lang="ts">
  import {
    RefreshCw, Download, Trash2, AlertTriangle, Check,
    GitBranch, Network, Lock, XCircle, Info,
  } from "lucide-svelte";
  import { connection, type ActivityKind } from "../state/connection.svelte";

  let visible = $state(false);
  let hideTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    if (connection.lastActivity) {
      visible = true;
      if (hideTimer) clearTimeout(hideTimer);
      hideTimer = setTimeout(() => { visible = false; }, 4500);
    }
  });

  type Variant = "ok" | "warn" | "danger" | "info" | "muted";

  function kindIcon(k: ActivityKind) {
    switch (k) {
      case "sync": return RefreshCw;
      case "pull": return Download;
      case "delete": return Trash2;
      case "conflict": return AlertTriangle;
      case "conflict_resolved": return Check;
      case "drift": return GitBranch;
      case "bridge": return Network;
      case "block": return Lock;
      case "error": return XCircle;
      case "system": return Info;
    }
  }
  function kindVariant(k: ActivityKind): Variant {
    switch (k) {
      case "sync":
      case "conflict_resolved": return "ok";
      case "pull":
      case "bridge": return "info";
      case "drift":
      case "delete":
      case "block": return "warn";
      case "conflict":
      case "error": return "danger";
      default: return "muted";
    }
  }

  function dismiss() {
    visible = false;
    if (hideTimer) { clearTimeout(hideTimer); hideTimer = null; }
  }
</script>

{#if visible && connection.lastActivity}
  {@const a = connection.lastActivity}
  {@const Icon = kindIcon(a.kind)}
  {@const v = kindVariant(a.kind)}
  <button
    class="toast"
    type="button"
    data-variant={v}
    onclick={dismiss}
    aria-label="Dismiss activity toast"
  >
    <span class="kind" data-variant={v}>
      <Icon size={12}/>
    </span>
    <span class="text">
      <span class="label">{a.action}</span>
      <span class="detail mono">{a.resource}{a.file ? ` · ${a.file}` : ""}</span>
    </span>
  </button>
{/if}

<style>
  .toast {
    position: fixed;
    bottom: 32px;
    right: 16px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    padding: 8px 12px;
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 10px; align-items: center;
    color: var(--fg);
    box-shadow: var(--shadow-lg, 0 10px 30px rgba(0,0,0,0.4));
    max-width: 360px;
    z-index: 50;
    cursor: pointer;
    text-align: left;
    font: inherit;
    animation: fade-in 160ms ease-out both;
  }
  .toast:hover { border-color: var(--accent); }
  .toast[data-variant="ok"]     { border-left: 3px solid var(--ok); }
  .toast[data-variant="warn"]   { border-left: 3px solid var(--warn); }
  .toast[data-variant="danger"] { border-left: 3px solid var(--danger); }
  .toast[data-variant="info"]   { border-left: 3px solid var(--info); }
  .toast[data-variant="muted"]  { border-left: 3px solid var(--border-strong); }

  .kind {
    width: 24px; height: 24px;
    border-radius: var(--radius-xs);
    display: inline-flex; align-items: center; justify-content: center;
    background: var(--bg-elev-3);
    color: var(--fg-muted);
    flex-shrink: 0;
  }
  .kind[data-variant="ok"]     { background: var(--ok-soft);     color: var(--ok); }
  .kind[data-variant="warn"]   { background: var(--warn-soft);   color: var(--warn); }
  .kind[data-variant="danger"] { background: var(--danger-soft); color: var(--danger); }
  .kind[data-variant="info"]   { background: var(--info-soft);   color: var(--info); }

  .text { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .label {
    color: var(--fg);
    font-size: var(--fs-sm);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .detail {
    color: var(--fg-subtle);
    font-size: var(--fs-xs);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
</style>

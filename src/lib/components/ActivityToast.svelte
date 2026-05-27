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
    --tone: var(--info);
    position: fixed;
    bottom: 32px;
    right: 16px;
    background: color-mix(in oklch, var(--bg-elev-1) 90%, transparent);
    backdrop-filter: blur(14px) saturate(140%);
    -webkit-backdrop-filter: blur(14px) saturate(140%);
    border: 1px solid color-mix(in oklch, var(--tone) 22%, var(--border-strong));
    border-left: 3px solid var(--tone);
    border-radius: 10px;
    padding: 10px 14px;
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 10px; align-items: center;
    color: var(--fg);
    box-shadow:
      0 14px 36px -8px oklch(0 0 0 / 0.55),
      0 0 0 1px color-mix(in oklch, var(--tone) 10%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 5%, transparent);
    max-width: 360px;
    z-index: 50;
    cursor: pointer;
    text-align: left;
    font: inherit;
    animation: toast-in 220ms cubic-bezier(0.22, 1, 0.36, 1) both;
    transition: border-color 140ms ease-out, transform 140ms ease-out, box-shadow 140ms ease-out;
  }
  .toast:hover {
    border-color: color-mix(in oklch, var(--tone) 42%, var(--border-strong));
    transform: translateY(-1px);
    box-shadow:
      0 18px 44px -8px oklch(0 0 0 / 0.6),
      0 0 0 1px color-mix(in oklch, var(--tone) 18%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 6%, transparent);
  }
  .toast[data-variant="ok"]     { --tone: var(--ok); }
  .toast[data-variant="warn"]   { --tone: var(--warn); }
  .toast[data-variant="danger"] { --tone: var(--danger); }
  .toast[data-variant="info"]   { --tone: var(--info); }
  .toast[data-variant="muted"]  { --tone: var(--border-strong); }
  @keyframes toast-in {
    from { opacity: 0; transform: translateY(12px) scale(0.96); }
    to   { opacity: 1; transform: translateY(0)    scale(1); }
  }
  @media (prefers-reduced-motion: reduce) {
    .toast { animation: none; }
  }

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

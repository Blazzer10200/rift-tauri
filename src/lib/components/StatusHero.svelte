<script lang="ts">
  import { AlertTriangle, Clock, XCircle, Activity } from "lucide-svelte";
  import { slide } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { connection } from "../state/connection.svelte";

  const stateLabel = $derived(connection.status?.state ?? "idle");
  const watches = $derived(connection.status?.watches ?? 0);
  const pending = $derived(connection.status?.pending ?? 0);
  const failed = $derived(connection.status?.failed ?? 0);
  const lastTs = $derived(
    connection.lastActivity
      ? new Date(connection.lastActivity.at).toLocaleTimeString([], { hour12: true })
      : null,
  );

  const isQuiet = $derived(
    connection.lockCount === 0 &&
    connection.conflictCount === 0 &&
    pending === 0 &&
    failed === 0,
  );

  type Variant = "default" | "ok" | "warn" | "danger" | "info";
  const heroVariant = $derived.by<Variant>(() => {
    if (!connection.status) return "default";
    if (connection.conflictCount > 0) return "danger";
    if (connection.status.state === "error") return "danger";
    if (failed > 0) return "warn";
    if (connection.lockCount > 0) return "warn";
    if (connection.status.state === "watching" || connection.status.state === "syncing") return "ok";
    return "info";
  });
</script>

<section class="hero" data-quiet={isQuiet} data-variant={heroVariant} aria-label="Sync status">
  {#if isQuiet}
    <div class="quiet-row" transition:slide={{ duration: 160, easing: quintOut }}>
      <span class="led-dot"></span>
      <span class="state">{stateLabel}</span>
      <span class="sep">·</span>
      <span>{watches} {watches === 1 ? "folder" : "folders"} watching</span>
      <span class="sep">·</span>
      <span class="muted">all quiet</span>
      {#if lastTs}
        <span class="sep">·</span>
        <span class="muted">last activity {lastTs}</span>
      {/if}
    </div>
  {:else}
    <div class="active-row" transition:slide={{ duration: 200, easing: quintOut }}>
      <span class="led-dot"></span>
      {#if connection.conflictCount > 0}
        <span class="chip danger"><AlertTriangle size={11}/> <strong>{connection.conflictCount}</strong> conflicts</span>
      {/if}
      {#if pending > 0}
        <span class="chip"><Clock size={11}/> <strong>{pending}</strong> queued</span>
      {/if}
      {#if failed > 0}
        <span class="chip danger"><XCircle size={11}/> <strong>{failed}</strong> errors</span>
      {/if}
      <span class="chip muted ml-auto"><Activity size={11}/> last {lastTs ?? "—"}</span>
    </div>
  {/if}
</section>

<style>
  .hero {
    padding: 0 14px;
    background: var(--bg-elev-1);
    border-bottom: 1px solid var(--border);
    transition: border-bottom-color 160ms ease;
  }
  .hero[data-variant="ok"]     { border-bottom-color: color-mix(in oklch, var(--ok)     35%, var(--border)); }
  .hero[data-variant="warn"]   { border-bottom-color: color-mix(in oklch, var(--warn)   35%, var(--border)); }
  .hero[data-variant="danger"] { border-bottom-color: color-mix(in oklch, var(--danger) 35%, var(--border)); }
  .hero[data-variant="info"]   { border-bottom-color: color-mix(in oklch, var(--info)   25%, var(--border)); }

  .quiet-row {
    display: flex; align-items: center; gap: 8px;
    height: 28px;
    font-size: var(--fs-xs);
    color: var(--fg-muted);
  }
  .quiet-row .state { color: var(--fg); text-transform: capitalize; }
  .quiet-row .sep   { color: var(--fg-faint); }
  .quiet-row .muted { color: var(--fg-muted); }

  .active-row {
    display: flex; align-items: center; gap: 8px;
    padding: 8px 0;
    flex-wrap: wrap;
  }
  .chip {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 3px 10px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 999px;
    font-size: var(--fs-xs);
    color: var(--fg-muted);
    line-height: 1.4;
  }
  .chip strong { color: var(--fg); font-weight: 600; font-variant-numeric: tabular-nums; }
  .chip.danger {
    border-color: color-mix(in oklch, var(--danger) 35%, var(--border));
    background: color-mix(in oklch, var(--danger) 8%, var(--surface));
  }
  .chip.danger strong { color: var(--danger); }
  .chip.muted { color: var(--fg-faint); }
  .ml-auto { margin-left: auto; }

  .led-dot {
    width: 8px; height: 8px; border-radius: 50%;
    background: var(--fg-muted);
    flex-shrink: 0;
    box-shadow: 0 0 0 2px color-mix(in oklch, var(--fg-muted) 18%, transparent);
  }
  .hero[data-variant="ok"]     .led-dot { background: var(--ok);     box-shadow: 0 0 0 2px color-mix(in oklch, var(--ok)     22%, transparent); }
  .hero[data-variant="warn"]   .led-dot { background: var(--warn);   box-shadow: 0 0 0 2px color-mix(in oklch, var(--warn)   22%, transparent); }
  .hero[data-variant="danger"] .led-dot { background: var(--danger); box-shadow: 0 0 0 2px color-mix(in oklch, var(--danger) 22%, transparent); }
  .hero[data-variant="info"]   .led-dot { background: var(--info);   box-shadow: 0 0 0 2px color-mix(in oklch, var(--info)   22%, transparent); }
  @media (prefers-reduced-motion: no-preference) {
    .hero[data-variant="ok"]     .led-dot,
    .hero[data-variant="warn"]   .led-dot,
    .hero[data-variant="danger"] .led-dot { animation: pulse-soft 2.4s ease-in-out infinite; }
  }
  @keyframes pulse-soft {
    0%, 100% { opacity: 1; }
    50%      { opacity: 0.55; }
  }
</style>

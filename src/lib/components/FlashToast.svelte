<script lang="ts">
  import { Check, AlertTriangle, Info, XCircle } from "lucide-svelte";

  export type FlashKind = "ok" | "err" | "info";

  type Props = {
    message: string;
    kind?: FlashKind;
    onDismiss?: () => void;
  };
  let { message, kind = "info", onDismiss }: Props = $props();

  const variant = $derived<"ok" | "danger" | "info">(
    kind === "ok" ? "ok" : kind === "err" ? "danger" : "info",
  );
  const Icon = $derived(
    kind === "ok" ? Check : kind === "err" ? XCircle : Info,
  );
</script>

<button
  class="flash"
  type="button"
  data-variant={variant}
  onclick={() => onDismiss?.()}
  aria-label="Dismiss"
>
  <span class="kind" data-variant={variant}>
    <Icon size={12}/>
  </span>
  <span class="text">{message}</span>
  {#if kind === "err"}
    <AlertTriangle size={11} class="hint"/>
  {/if}
</button>

<style>
  .flash {
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    padding: 8px 12px;
    display: inline-flex;
    align-items: center;
    gap: 10px;
    color: var(--fg);
    box-shadow: var(--shadow-lg, 0 10px 30px rgba(0,0,0,0.4));
    max-width: 460px;
    cursor: pointer;
    text-align: left;
    font: inherit;
    animation: fade-in 160ms ease-out both;
  }
  .flash:hover { border-color: var(--accent); }
  .flash[data-variant="ok"]     { border-left: 3px solid var(--ok); }
  .flash[data-variant="danger"] { border-left: 3px solid var(--danger); }
  .flash[data-variant="info"]   { border-left: 3px solid var(--info); }

  .kind {
    width: 22px; height: 22px;
    border-radius: var(--radius-xs);
    display: inline-flex; align-items: center; justify-content: center;
    background: var(--bg-elev-3);
    color: var(--fg-muted);
    flex-shrink: 0;
  }
  .kind[data-variant="ok"]     { background: var(--ok-soft);     color: var(--ok); }
  .kind[data-variant="danger"] { background: var(--danger-soft); color: var(--danger); }
  .kind[data-variant="info"]   { background: var(--info-soft);   color: var(--info); }

  .text {
    color: var(--fg);
    font-size: var(--fs-sm);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }

  :global(.hint) { color: var(--fg-faint); flex-shrink: 0; }

  @keyframes fade-in {
    from { opacity: 0; transform: translateY(6px); }
    to   { opacity: 1; transform: translateY(0); }
  }
</style>

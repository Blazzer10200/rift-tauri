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
    --tone: var(--info);
    background: color-mix(in oklch, var(--bg-elev-1) 90%, transparent);
    backdrop-filter: blur(14px) saturate(140%);
    -webkit-backdrop-filter: blur(14px) saturate(140%);
    border: 1px solid color-mix(in oklch, var(--tone) 24%, var(--border-strong));
    border-left: 3px solid var(--tone);
    border-radius: 10px;
    padding: 8px 14px;
    display: inline-flex;
    align-items: center;
    gap: 10px;
    color: var(--fg);
    box-shadow:
      0 12px 32px -8px oklch(0 0 0 / 0.55),
      0 0 0 1px color-mix(in oklch, var(--tone) 10%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 5%, transparent);
    max-width: 460px;
    cursor: pointer;
    text-align: left;
    font: inherit;
    animation: fade-in 200ms cubic-bezier(0.22, 1, 0.36, 1) both;
    transition: border-color 140ms ease-out, transform 140ms ease-out, box-shadow 140ms ease-out;
  }
  .flash:hover {
    border-color: color-mix(in oklch, var(--tone) 45%, var(--border-strong));
    transform: translateY(-1px);
    box-shadow:
      0 16px 40px -8px oklch(0 0 0 / 0.6),
      0 0 0 1px color-mix(in oklch, var(--tone) 18%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 6%, transparent);
  }
  .flash[data-variant="ok"]     { --tone: var(--ok); }
  .flash[data-variant="danger"] { --tone: var(--danger); }
  .flash[data-variant="info"]   { --tone: var(--info); }

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

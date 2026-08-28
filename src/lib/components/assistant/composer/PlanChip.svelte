<script lang="ts">
  // Plan artifact chip — the conversation's one plan, surfaced next to the
  // model pill so its lifecycle (proposed → executing → done) stays visible
  // after the stream card scrolls away. Click opens a read-only popover with
  // the full plan markdown. Fades out shortly after the plan lands done.
  import { ScrollText, Check, Loader2 } from "@lucide/svelte";
  import Markdown from "../Markdown.svelte";
  import { portal } from "$lib/actions/portal";
  import { tooltip } from "$lib/actions/tooltip";
  import type { TabState } from "$lib/state/assistant.svelte";

  let { tab }: { tab: TabState | null } = $props();
  const plan = $derived(tab?.plan ?? null);

  // Done chips linger long enough to register the finish, then get out of the
  // way — the plan itself stays in the popover-able record until replaced.
  let hideDone = $state(false);
  $effect(() => {
    if (plan?.status !== "done") {
      hideDone = false;
      return;
    }
    const t = setTimeout(() => (hideDone = true), 8000);
    return () => clearTimeout(t);
  });
  const visible = $derived(!!plan && !(plan.status === "done" && hideDone));

  const label = $derived(
    plan?.status === "proposed" ? "Plan ready"
    : plan?.status === "executing" ? "Building plan"
    : plan?.status === "approved" ? "Plan approved"
    : "Plan built",
  );
  const hint = $derived(
    plan?.status === "proposed" ? "A plan is waiting for your review — open the proposal card to approve it."
    : plan?.status === "executing" ? "The approved plan is being built right now."
    : plan?.status === "approved" ? "Approved, but the build didn't finish — send a message to continue."
    : "The plan was built to completion.",
  );

  let open = $state(false);
  let chipEl = $state<HTMLElement | null>(null);
  let panelEl = $state<HTMLElement | null>(null);
  $effect(() => {
    if (!visible) open = false;
  });

  // Anchor-relative position; flips above when the chip sits low (composer).
  let pos = $state({ left: 0, top: 0 });
  $effect(() => {
    if (!open || !chipEl) return;
    void plan;
    const r = chipEl.getBoundingClientRect();
    const w = 360;
    const h = panelEl?.offsetHeight ?? 280;
    const left = Math.min(Math.max(8, r.left), window.innerWidth - w - 8);
    const top = r.bottom + h + 12 > window.innerHeight ? r.top - h - 8 : r.bottom + 8;
    pos = { left, top };
  });

  function onDocMousedown(ev: MouseEvent) {
    if (!(ev.target instanceof Node)) return;
    if (chipEl?.contains(ev.target) || panelEl?.contains(ev.target)) return;
    open = false;
  }
  function onKey(ev: KeyboardEvent) {
    if (ev.key !== "Escape") return;
    // Escape here means "close the popover", not the app-level binding
    // (which navigates the chat away) — consume it.
    ev.stopPropagation();
    open = false;
  }
  $effect(() => {
    if (!open) return;
    window.addEventListener("mousedown", onDocMousedown);
    // Capture phase so the popover's Escape wins over app-level bindings.
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("mousedown", onDocMousedown);
      window.removeEventListener("keydown", onKey, true);
    };
  });
</script>

{#if visible && plan}
  <button
    type="button"
    class="plan-chip {plan.status}"
    class:open
    bind:this={chipEl}
    onclick={() => (open = !open)}
    aria-haspopup="dialog"
    aria-expanded={open}
    aria-label="Plan status"
    use:tooltip={`Plan · ${label.toLowerCase()}${plan.revision > 1 ? ` · revision ${plan.revision}` : ""}\nClick to read the plan`}
  >
    {#if plan.status === "executing"}
      <Loader2 size={11} class="plan-chip-spin" />
    {:else if plan.status === "done"}
      <Check size={11} />
    {:else}
      <ScrollText size={11} />
    {/if}
    <span class="plan-chip-t">{label}</span>
    {#if plan.revision > 1}<span class="plan-chip-rev">r{plan.revision}</span>{/if}
  </button>

  {#if open}
    <div
      class="rift-menu plan-pop"
      use:portal
      bind:this={panelEl}
      style="left: {pos.left}px; top: {pos.top}px;"
      role="dialog"
      aria-label="Plan details"
    >
      <header class="pp-head">
        <span class="pp-title"><ScrollText size={12} /> Plan</span>
        <span class="pp-meta">
          {#if plan.revision > 1}<span class="pp-rev">revision {plan.revision}</span><span class="pp-sep">·</span>{/if}
          <span class="pp-status {plan.status}">{label}</span>
        </span>
      </header>
      <div class="pp-hint">{hint}</div>
      <div class="pp-body">
        <Markdown text={plan.md} />
      </div>
    </div>
  {/if}
{/if}

<style>
  .plan-chip {
    display: inline-flex; align-items: center; gap: 5px;
    height: 22px; padding: 0 9px;
    border: 1px solid var(--border); border-radius: 999px;
    background: var(--bg-elev-1); color: var(--fg-2);
    font: inherit; font-size: 10.5px; font-weight: 600; letter-spacing: 0.01em;
    cursor: pointer; flex: none; white-space: nowrap;
  }
  .plan-chip:hover, .plan-chip.open { background: var(--surface-hover); color: var(--fg); border-color: var(--border-strong); }
  .plan-chip :global(svg) { flex: none; }
  .plan-chip.proposed { border-color: color-mix(in oklab, var(--accent) 38%, var(--border)); color: var(--accent); }
  .plan-chip.proposed:hover { color: var(--accent); }
  .plan-chip.executing :global(svg) { color: var(--accent); }
  .plan-chip.done { color: var(--ok); border-color: color-mix(in oklab, var(--ok) 30%, var(--border)); }
  .plan-chip.done:hover { color: var(--ok); }
  :global(.plan-chip-spin) { animation: plan-chip-rotate 0.9s linear infinite; }
  @keyframes plan-chip-rotate { to { transform: rotate(360deg); } }
  .plan-chip-t { max-width: 110px; overflow: hidden; text-overflow: ellipsis; }
  .plan-chip-rev { font-size: 9.5px; color: var(--fg-faint); font-weight: 700; }

  .plan-pop { position: fixed; width: 360px; z-index: 950; padding: 8px; }
  .pp-head {
    display: flex; align-items: center; justify-content: space-between; gap: 8px;
    padding: 2px 4px 8px; border-bottom: 1px solid var(--border); margin-bottom: 6px;
  }
  .pp-title { display: inline-flex; align-items: center; gap: 6px; font-size: var(--fs-sm); font-weight: 650; color: var(--fg); }
  .pp-title :global(svg) { color: var(--accent); }
  .pp-meta { display: inline-flex; align-items: center; gap: 6px; font-size: 10.5px; }
  .pp-rev { color: var(--fg-subtle); }
  .pp-sep { color: var(--fg-faint); }
  .pp-status { font-weight: 650; color: var(--fg-subtle); }
  .pp-status.proposed { color: var(--accent); }
  .pp-status.executing { color: var(--accent); }
  .pp-status.done { color: var(--ok); }
  .pp-hint { padding: 0 4px 6px; font-size: var(--fs-sm); color: var(--fg-muted); line-height: 1.45; }
  .pp-body { max-height: 320px; overflow-y: auto; padding: 2px 4px 4px; font-size: 12.5px; }

  @media (prefers-reduced-motion: reduce) {
    :global(.plan-chip-spin) { animation: none; }
  }
</style>

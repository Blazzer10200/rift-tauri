<script lang="ts">
  // #13 minimal tab surface — chat tabs otherwise have ZERO visible UI (the
  // old ChatTabsBar died in the redesign; Ctrl+T mints an invisible tab and
  // Ctrl+Tab is the only way to switch). A quiet count pill in the topbar +
  // a rift-menu dropdown: see the open tabs, switch, close, mint a new one.
  // Hidden below 2 tabs — a single tab is the active one, no gap to close.
  import { ChevronDown, Plus, X } from "lucide-svelte";
  import { portal } from "$lib/actions/portal";
  import { tooltip } from "$lib/actions/tooltip";
  import { assistant } from "$lib/state/assistant.svelte";
  import { workspace } from "$lib/state/workspace.svelte";

  let open = $state(false);
  let anchorEl = $state<HTMLElement | null>(null);
  let panelEl = $state<HTMLElement | null>(null);
  const tabs = $derived(assistant.tabSummaries);

  // Right-aligned under the pill; re-measures as rows come and go.
  let pos = $state({ left: 0, top: 0 });
  $effect(() => {
    if (!open || !anchorEl) return;
    void tabs.length;
    const r = anchorEl.getBoundingClientRect();
    const w = 264;
    pos = {
      left: Math.min(Math.max(8, r.right - w), window.innerWidth - w - 8),
      top: r.bottom + 6,
    };
  });

  function onDocMousedown(ev: MouseEvent) {
    if (!(ev.target instanceof Node)) return;
    if (anchorEl?.contains(ev.target) || panelEl?.contains(ev.target)) return;
    open = false;
  }
  function onKey(ev: KeyboardEvent) {
    if (ev.key === "Escape") open = false;
  }
  $effect(() => {
    if (!open) return;
    window.addEventListener("mousedown", onDocMousedown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDocMousedown);
      window.removeEventListener("keydown", onKey);
    };
  });
  // Pill disappears below 2 tabs — don't leave an orphaned floating menu.
  $effect(() => {
    if (open && tabs.length < 2) open = false;
  });

  function switchTo(id: string) {
    open = false;
    workspace.setActive("chat");
    void assistant.openTab(id).catch(console.error);
  }
  function closeOne(id: string) {
    void assistant.closeTab(id).catch(console.error);
  }
  function fresh() {
    open = false;
    workspace.setActive("chat");
    void assistant.newTab().catch(console.error);
  }
</script>

{#if tabs.length >= 2}
  <button
    class="tabs-pill"
    type="button"
    bind:this={anchorEl}
    onclick={() => (open = !open)}
    aria-haspopup="menu"
    aria-expanded={open}
    use:tooltip={"Open chat tabs — Ctrl+Tab cycles"}
  >
    <span class="tp-count">{tabs.length}</span>
    <span class="tp-label">tabs</span>
    <ChevronDown size={11} />
  </button>
{/if}

{#if open}
  <div
    class="rift-menu tabs-menu"
    use:portal
    bind:this={panelEl}
    style="left: {pos.left}px; top: {pos.top}px;"
    role="menu"
    aria-label="Open chat tabs"
  >
    {#each tabs as t (t.id)}
      <div class="tm-row" class:current={t.id === assistant.currentConvoId}>
        <button class="tm-open" type="button" role="menuitem" onclick={() => switchTo(t.id)}>
          <span class="tm-dot" class:busy={t.streaming}></span>
          <span class="tm-title">{t.title}</span>
        </button>
        <button class="tm-x" type="button" onclick={() => closeOne(t.id)} aria-label={`Close ${t.title}`}>
          <X size={11} />
        </button>
      </div>
    {/each}
    <div class="rift-menu-divider"></div>
    <button class="tm-row tm-new" type="button" role="menuitem" onclick={fresh}>
      <span class="tm-newic"><Plus size={12} /></span>
      <span class="tm-title">New tab</span>
      <span class="tm-kbd">Ctrl+T</span>
    </button>
  </div>
{/if}

<style>
  .tabs-pill {
    -webkit-app-region: no-drag;
    display: inline-flex; align-items: center; gap: 4px;
    height: 24px; padding: 0 8px; border-radius: 999px;
    font-size: var(--fs-xs); color: var(--fg-muted);
    border: 1px solid var(--border); background: transparent;
    transition: background var(--dur-fast), color var(--dur-fast);
  }
  .tabs-pill:hover { background: var(--surface-hover); color: var(--fg-2); }
  .tabs-pill[aria-expanded="true"] { background: var(--surface-hover); color: var(--fg-2); }
  .tp-count { font-weight: 600; color: var(--fg-2); }
  .tp-label { letter-spacing: 0.01em; }

  .tabs-menu { position: fixed; z-index: 1000; width: 264px; }

  .tm-row {
    position: relative; display: flex; align-items: center; gap: 2px;
    width: 100%; border-radius: var(--radius-sm);
    transition: background var(--dur-fast) ease;
  }
  .tm-row:hover { background: var(--surface-hover); }
  .tm-row.current::before {
    content: ""; position: absolute; left: 0; top: 5px; bottom: 5px; width: 2.5px;
    border-radius: 0 3px 3px 0; background: var(--accent);
  }
  .tm-open {
    flex: 1; min-width: 0; display: flex; align-items: center; gap: 8px;
    padding: 6px 2px 6px 10px; border: 0; background: transparent;
    color: var(--fg-2); cursor: pointer; font: inherit; text-align: left;
  }
  .tm-dot { width: 6px; height: 6px; border-radius: 50%; flex: none; background: var(--fg-faint); }
  .tm-dot.busy { background: var(--status-busy); animation: tm-breathe var(--pulse-live) ease-in-out infinite; }
  .tm-title {
    flex: 1; min-width: 0; font-size: var(--fs-sm);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .tm-x {
    flex: none; width: 22px; height: 22px; margin-right: 4px;
    display: grid; place-items: center; border: 0; border-radius: 5px;
    background: transparent; color: var(--fg-subtle); cursor: pointer;
    opacity: 0; transition: opacity var(--dur-fast), background var(--dur-fast);
  }
  .tm-row:hover .tm-x, .tm-x:focus-visible { opacity: 1; }
  .tm-x:hover { background: var(--surface-hover); color: var(--fg); }

  .tm-new { padding: 6px 10px; border: 0; background: transparent; color: var(--fg-2); cursor: pointer; font: inherit; text-align: left; }
  .tm-newic { width: 14px; display: inline-flex; align-items: center; justify-content: center; color: var(--fg-subtle); }
  .tm-kbd { flex: none; font-size: 10px; color: var(--fg-faint); }

  @keyframes tm-breathe { 0%, 100% { opacity: 1; } 50% { opacity: 0.45; } }
  @media (prefers-reduced-motion: reduce) { .tm-dot.busy { animation: none; } }
</style>

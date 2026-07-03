<script lang="ts">
  // Pinned plan HUD — the live `tab.tasks` aggregate rendered as a glassy bar
  // floating at the top of the conversation, so the plan never scrolls away
  // with the stream. Collapsed: icon + current task + progress + count.
  // Click expands the full checklist. All-done → brief green linger, then gone.
  import { Check, ChevronDown, ListChecks } from "lucide-svelte";
  import { tasksToPlanItems } from "./streamModel";
  import type { TabState } from "$lib/state/assistant.svelte";

  let { tab = null, streaming = false }:
    { tab?: TabState | null; streaming?: boolean } = $props();

  const items = $derived(tasksToPlanItems(tab?.tasks ?? []));
  const done = $derived(items.filter((i) => i.status === "done").length);
  const total = $derived(items.length);
  const pct = $derived(total ? Math.round((done / total) * 100) : 0);
  const allDone = $derived(total > 0 && done === total);
  const activeText = $derived(
    items.find((i) => i.status === "active")?.text
      ?? items.find((i) => i.status === "todo")?.text
      ?? "",
  );

  let open = $state(false);

  // Completion linger — only after the HUD was live-visible this session, so
  // reopening an old convo with a finished plan doesn't flash a stale 4/4.
  let sawLive = $state(false);
  let linger = $state(false);
  let lingerTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    if (total > 0 && !allDone) {
      sawLive = true;
      linger = false;
      if (lingerTimer) { clearTimeout(lingerTimer); lingerTimer = null; }
    } else if (allDone && sawLive && !lingerTimer) {
      linger = true;
      lingerTimer = setTimeout(() => { linger = false; lingerTimer = null; }, 4000);
    }
  });
  $effect(() => () => { if (lingerTimer) clearTimeout(lingerTimer); });

  const visible = $derived(total > 0 && (!allDone || linger));
  $effect(() => { if (!visible) open = false; });
</script>

{#if visible}
  <div class="phud" class:complete={allDone} class:open>
    <button
      class="phud-bar"
      type="button"
      aria-expanded={open}
      aria-label={open ? "Collapse plan" : "Expand plan"}
      onclick={() => (open = !open)}
    >
      <span class="phud-ic" aria-hidden="true">
        {#if allDone}
          <Check size={13} strokeWidth={2.5} />
        {:else if streaming}
          <span class="phud-ring"></span>
        {:else}
          <ListChecks size={13} />
        {/if}
      </span>
      <span class="phud-text">{allDone ? "Plan complete" : activeText}</span>
      <span class="phud-track"><span class="phud-fill" style="width:{pct}%"></span></span>
      <span class="phud-count">{done}/{total}</span>
      <span class="phud-chev" aria-hidden="true"><ChevronDown size={12} /></span>
    </button>
    {#if open}
      <ul class="phud-list">
        {#each items as it, i (i)}
          <li class="phud-item is-{it.status}">
            <span class="phud-mark" aria-hidden="true">
              {#if it.status === "done"}<Check size={12} strokeWidth={2.5} />
              {:else if it.status === "active"}<span class="phud-ring sm"></span>
              {:else}<span class="phud-dot"></span>{/if}
            </span>
            <span class="phud-item-text">{it.text}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<style>
  .phud {
    position: absolute;
    top: 10px;
    left: 50%;
    transform: translateX(-50%);
    width: min(560px, calc(100% - 36px));
    z-index: 4;
    border-radius: 12px;
    background: color-mix(in oklch, var(--surface) 84%, transparent);
    backdrop-filter: blur(16px) saturate(140%);
    -webkit-backdrop-filter: blur(16px) saturate(140%);
    border: 1px solid var(--border-strong);
    box-shadow: 0 12px 30px -14px oklch(0 0 0 / 0.6), 0 8px 22px -10px oklch(0 0 0 / 0.45);
    overflow: hidden;
    animation: phud-in 260ms cubic-bezier(0.22, 1, 0.36, 1) both;
    transition: border-color 200ms ease-out;
  }
  @keyframes phud-in {
    from { opacity: 0; transform: translateX(-50%) translateY(-8px); }
    to { opacity: 1; transform: translateX(-50%) translateY(0); }
  }
  .phud.complete { border-color: color-mix(in oklab, var(--ok) 45%, var(--border-strong)); }

  .phud-bar {
    display: flex; align-items: center; gap: 9px;
    width: 100%; height: 32px; padding: 0 12px 0 11px;
    background: none; border: 0; cursor: pointer;
    font-size: 12px; color: var(--fg-2); text-align: left;
    transition: background var(--dur-fast);
  }
  .phud-bar:hover { background: color-mix(in oklab, var(--fg) 4%, transparent); }
  .phud-ic {
    display: grid; place-items: center;
    width: 16px; height: 16px; flex: none;
    color: var(--accent);
  }
  .phud.complete .phud-ic { color: var(--ok); }
  .phud-ring {
    width: 12px; height: 12px; border-radius: 50%; display: block;
    border: 2px solid color-mix(in oklab, var(--accent) 30%, transparent);
    border-top-color: var(--accent);
    animation: phud-spin 0.9s linear infinite;
  }
  .phud-ring.sm { width: 11px; height: 11px; }
  @keyframes phud-spin { to { transform: rotate(360deg); } }
  .phud-text {
    flex: 1; min-width: 0;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    font-weight: 500; letter-spacing: -0.004em;
  }
  .phud.complete .phud-text { color: var(--ok); font-weight: 600; }
  .phud-track {
    flex: none; width: 72px; height: 4px; border-radius: 999px;
    background: color-mix(in oklab, var(--fg) 9%, transparent);
    overflow: hidden;
  }
  .phud-fill {
    display: block; height: 100%; border-radius: 999px;
    background: var(--accent);
    transition: width var(--dur-base) var(--ease-page), background 200ms ease-out;
  }
  .phud.complete .phud-fill { background: var(--ok); }
  .phud-count {
    flex: none;
    font-family: var(--font-mono, monospace); font-size: 10.5px;
    font-variant-numeric: tabular-nums;
    color: var(--fg-subtle);
  }
  .phud-chev { display: grid; place-items: center; flex: none; color: var(--fg-faint);
    transition: transform var(--dur-fast); }
  .phud.open .phud-chev { transform: rotate(180deg); }

  .phud-list {
    margin: 0; padding: 6px 12px 9px;
    list-style: none;
    display: flex; flex-direction: column; gap: 2px;
    border-top: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    max-height: 40vh; overflow-y: auto;
    scrollbar-width: none;
    animation: phud-open 200ms var(--ease-page) both;
  }
  .phud-list::-webkit-scrollbar { width: 0; height: 0; display: none; }
  @keyframes phud-open { from { opacity: 0; transform: translateY(-3px); } to { opacity: 1; transform: none; } }
  .phud-item {
    display: flex; align-items: center; gap: 9px;
    padding: 3px 0;
    font-size: 12px; line-height: 1.45;
    color: var(--fg-muted);
  }
  .phud-mark { width: 15px; height: 15px; display: grid; place-items: center; flex: none; }
  .phud-dot {
    width: 6px; height: 6px; border-radius: 50%; display: block;
    border: 1.5px solid currentColor;
  }
  .phud-item.is-done .phud-mark { color: var(--ok); }
  .phud-item.is-done .phud-item-text {
    color: var(--fg-faint);
    text-decoration: line-through;
    text-decoration-color: color-mix(in oklch, var(--fg-faint) 60%, transparent);
  }
  .phud-item.is-todo .phud-mark, .phud-item.is-todo .phud-item-text { color: var(--fg-faint); }
  .phud-item.is-active .phud-item-text { color: var(--fg); font-weight: 550; }
  .phud-item-text { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  @media (prefers-reduced-motion: reduce) {
    .phud, .phud-list { animation: none; }
    .phud-ring { animation: none; }
  }
</style>

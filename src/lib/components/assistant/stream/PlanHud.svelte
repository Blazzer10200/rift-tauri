<script lang="ts">
  // Pinned plan HUD — the live `tab.tasks` aggregate rendered as a glassy bar
  // floating at the top of the conversation, so the plan never scrolls away
  // with the stream. Collapsed: icon + current task + progress + count.
  // Click expands the full checklist. All-done → brief green linger, then gone.
  import { Check, ChevronDown, ListChecks } from "lucide-svelte";
  import { fade } from "svelte/transition";
  import { tasksToPlanItems } from "./streamModel";
  import type { TabState } from "$lib/state/assistant.svelte";

  // Svelte transitions don't respect prefers-reduced-motion on their own.
  const reduceMotion =
    typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;

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
  // Duplication guard (owner ask 2026-07-08, mirrors AgentHud): the bar only
  // shows while the inline plan card it mirrors is OFF-screen — it's an
  // overflow affordance, not a second copy. Declared above the deriveds that
  // read it (Svelte 5 TDZ — see the pinned $effect.pre gotcha).
  let planCardInView = $state(false);
  let sentinelEl = $state<HTMLElement | null>(null);

  // Completion linger — only after the HUD was live-visible for THIS tab, so
  // reopening an old convo with a finished plan doesn't flash a stale 4/4.
  let sawLive = $state(false);
  let linger = $state(false);
  let lingerTimer: ReturnType<typeof setTimeout> | null = null;
  // One linger per completion — latches when the timer arms, clears when the
  // plan goes live again. Declared HERE, above the effects: $effect.pre runs
  // its first pass eagerly during component init, so a later declaration is a
  // TDZ ReferenceError that kills the whole mount (caught live, cont.269).
  let lingered = false;

  // Per-tab reset — the pane reuses ONE HUD instance across tab switches (`tab`
  // swaps as a prop; deliberately NOT a {#key} remount at the mount site,
  // because tearing the subtree down while .phud-bar held focus dropped
  // keyboard focus to <body> on Ctrl+Tab). Without this, tab A's sawLive let
  // tab B's long-finished plan flash the stale green linger, and an expanded
  // checklist carried over. Declared BEFORE the linger effect — effects run in
  // declaration order, so the reset settles before linger sees the new tasks.
  let seenTab: TabState | null | undefined;
  $effect.pre(() => {
    if (tab === seenTab) return;
    seenTab = tab;
    sawLive = false;
    linger = false;
    lingered = false;
    open = false;
    if (lingerTimer) { clearTimeout(lingerTimer); lingerTimer = null; }
  });

  // `lingered`: without the latch, this effect re-runs when `streaming` flips
  // false at turn end; lingerTimer (a plain untracked var) is null again
  // post-expiry while allDone && sawLive still hold, so the green bar flashed
  // a SECOND 4s time (observed live via MutationObserver, cont.269: two
  // ADD/REMOVE cycles exactly 4.0s apart).
  // $effect.pre (both this and the reset above): linger must settle BEFORE the
  // render pass. As a plain post-render effect, the completion flush rendered
  // once with visible=false (in-progress branch dead, linger not yet set) —
  // unmounting + remounting .phud in the same flush (observed live: REMOVE+ADD
  // at the same ms), replaying the entrance animation and bouncing focus.
  $effect.pre(() => {
    // sawLive requires a LIVE stream frame, not just unfinished tasks —
    // tab.tasks persists to disk, so a reopened convo has total>0 without
    // ever having been live in this view.
    if (streaming && total > 0 && !allDone) {
      sawLive = true;
      linger = false;
      lingered = false;
      if (lingerTimer) { clearTimeout(lingerTimer); lingerTimer = null; }
    } else if (allDone && sawLive && !lingerTimer && !lingered) {
      linger = true;
      lingered = true;
      lingerTimer = setTimeout(() => { linger = false; lingerTimer = null; }, 4000);
    }
  });
  $effect(() => () => { if (lingerTimer) clearTimeout(lingerTimer); });

  // Visible only while the plan is actually LIVE (mid-stream) or in its brief
  // completion linger. tab.tasks is never reset on the error/stop path, so a
  // tasks-only gate wedged a live-looking bar at frozen progress forever after
  // a failed turn — streaming is the only honest liveness signal. The
  // transcript's inline plan card + error banner keep the record between
  // turns; the next turn's task frames bring the HUD straight back.
  const visible = $derived(((streaming && total > 0 && !allDone) || linger) && !planCardInView);
  $effect(() => { if (!visible) open = false; });

  // Watch the LIVE inline plan card (the last .splan in this pane's stream —
  // StreamTurn renders one card per plan-bearing turn; the newest mirrors the
  // same aggregate this HUD reads). ≥15% visible → the card is the display,
  // the bar yields. Re-arms when the plan grows (total) or the turn count
  // changes the card set (items identity).
  $effect(() => {
    void total;
    void items;
    const scope = sentinelEl?.closest(".csurf-col");
    const rootEl = scope?.querySelector(".stream");
    if (!scope || !rootEl || total === 0) {
      planCardInView = false;
      return;
    }
    const cards = scope.querySelectorAll(".splan");
    const target = cards[cards.length - 1];
    if (!target) {
      planCardInView = false;
      return;
    }
    // Boundary polish (mirrors AgentHud): yield immediately on card-in-view,
    // debounce the appear so threshold-hover during auto-scroll can't flicker
    // the bar. Pessimistic start kills the one-frame flash on re-arm.
    let showTimer: ReturnType<typeof setTimeout> | null = null;
    planCardInView = true;
    const io = new IntersectionObserver(
      (entries) => {
        const inView = entries.some((e) => e.isIntersecting);
        if (inView) {
          if (showTimer) { clearTimeout(showTimer); showTimer = null; }
          planCardInView = true;
        } else if (!showTimer) {
          showTimer = setTimeout(() => { planCardInView = false; showTimer = null; }, 350);
        }
      },
      { root: rootEl, threshold: 0.15 },
    );
    io.observe(target);
    return () => {
      io.disconnect();
      if (showTimer) clearTimeout(showTimer);
    };
  });

  // Focus rescue — the HUD hides on events the user didn't initiate (linger
  // timeout, stream end); if the expand button held focus, the {#if} teardown
  // would silently drop focus to <body>, restarting keyboard Tab order at the
  // top of the document. Hand focus to this pane's composer textarea instead.
  let hudEl = $state<HTMLElement | null>(null);
  function rescueFocus() {
    const el = hudEl;
    if (!el || !el.contains(document.activeElement)) return;
    el.closest(".pane-shell")?.querySelector<HTMLElement>("textarea")?.focus();
  }
  // Pre-effect covers the in-place hide ({#if visible} flip): runs BEFORE the
  // DOM update, while the node is still attached.
  $effect.pre(() => {
    if (!visible) rescueFocus();
  });
  // Cleanup covers REAL unmounts the pre-effect never sees — the parent's
  // {#if showEmpty} branch swap (switching to an empty tab), tab close, and
  // /clear all destroy this component with `visible` still true. Effect
  // teardown runs during destroy while the node is still attached; if it ever
  // ran post-detach, contains() is false and this is a no-op (no worse than
  // the unrescued path).
  $effect(() => () => rescueFocus());
</script>

<span class="phud-sentinel" bind:this={sentinelEl} aria-hidden="true"></span>
{#if visible}
  <div class="phud" class:complete={allDone} class:open bind:this={hudEl}
    out:fade={{ duration: reduceMotion ? 0 : 160 }}>
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
  .phud-sentinel { display: none; }

  /* Positioning moved to the shared .hud-stack in AssistantPane (2026-07-08,
     AgentHud arrival) — the stack owns centering/width/z so plan + agent bars
     stack with a gap instead of overlapping at the same absolute spot. */
  .phud {
    width: 100%;
    border-radius: 12px;
    background: color-mix(in oklch, var(--surface) 84%, transparent);
    backdrop-filter: blur(16px) saturate(140%);
    -webkit-backdrop-filter: blur(16px) saturate(140%);
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow-float);
    overflow: hidden;
    animation: phud-in var(--dur-base) cubic-bezier(0.22, 1, 0.36, 1) both;
    transition: border-color var(--dur-base) ease-out;
  }
  @keyframes phud-in {
    from { opacity: 0; transform: translateY(-8px); }
    to { opacity: 1; transform: none; }
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
    animation: phud-open var(--dur-base) var(--ease-page) both;
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

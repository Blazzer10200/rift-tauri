<script lang="ts">
  // C4 (per docs/design/composer-split.md) — the toolbar's middle slot, lifted
  // verbatim from Composer.svelte 2026-06-10: live-activity pills while a turn
  // runs (elapsed · tok/s · agents · shells · tools · queued) and the ↵/⇧↵
  // keyboard hint while idle-focused. Talks to the assistant store directly
  // (telemetry snapshot + Activity dock toggle — the cluster already reads it
  // pervasively, per the brief's QueueRail precedent).
  import { Bot, Terminal, Wrench, ListPlus } from "lucide-svelte";
  import { assistant } from "../../../state/assistant.svelte";
  import type { ChatMessage } from "../../../state/assistant/types";
  import { liveActivity, fmtTokens } from "../../../state/assistant/helpers";
  import { fmtClock } from "./helpers";
  import { tooltip } from "$lib/actions/tooltip";

  type AgentSpawnLike = { id: string; subagentType: string; description: string; startedAt: number; completedAt: number | null };

  let {
    tab,
    queue,
    streaming,
    composerFocused,
  }: {
    tab: { messages: ChatMessage[]; agentSpawns: AgentSpawnLike[]; activity: { turnStartedAt: number | null } } | null;
    queue: { id: string; text: string }[];
    streaming: boolean;
    composerFocused: boolean;
  } = $props();

  // ── Live activity pills ───────────────────────────────────────────────
  // Compact, additive readout of in-flight work — reuses the Activity panel's
  // `liveActivity` derivation (single source of truth) plus the telemetry
  // tok/s, so a busy turn surfaces ◍ agents · ▸ shells · elapsed + tok/s ·
  // queued without opening the panel. The idle bar renders none of this. The
  // 1s ticker only runs while streaming (drives elapsed + tok/s refresh).
  let now = $state(Date.now());
  $effect(() => {
    if (!streaming) return;
    const h = setInterval(() => { now = Date.now(); }, 1000);
    return () => clearInterval(h);
  });
  const liveItems = $derived(liveActivity(tab?.messages ?? [], tab?.agentSpawns ?? [], now));
  const agentCount = $derived(liveItems.filter((i) => i.kind === "agent").length);
  const shellCount = $derived(liveItems.filter((i) => i.kind === "shell").length);
  const toolCount = $derived(liveItems.filter((i) => i.kind === "tool").length);
  const turnStartedAt = $derived(tab?.activity.turnStartedAt ?? null);
  const turnElapsed = $derived(
    streaming && turnStartedAt != null ? fmtClock(now - turnStartedAt) : null,
  );
  // Cumulative output tokens for the in-flight turn (CC-style); touch `now`
  // so it refreshes on the 1s tick alongside elapsed.
  const liveTokens = $derived.by(() => {
    void now;
    return streaming && assistant.liveOutputTokens > 0 ? assistant.liveOutputTokens : null;
  });
  const showLivePills = $derived(streaming || agentCount > 0 || shellCount > 0 || toolCount > 0 || queue.length > 0);
  // Toggle the Activity dock.
  function openActivity() {
    assistant.ui.dockOpen = !assistant.ui.dockOpen;
  }
</script>

{#if showLivePills}
  <div class="live-pills" role="group" aria-label="Live turn activity">
    {#if turnElapsed}
      <button
        type="button"
        class="live-pill turn"
        onclick={openActivity}
        aria-label="Current turn — elapsed · output tokens. Click to open Activity."
        use:tooltip={"Current turn — elapsed · output tokens. Click to open Activity."}
      >
        <span class="lp-dot" aria-hidden="true"></span>
        <span class="mono">{turnElapsed}</span>
        {#if liveTokens != null}
          <span class="lp-sep" aria-hidden="true">·</span>
          <span class="mono">{fmtTokens(liveTokens)}<span class="lp-unit"> tokens</span></span>
        {/if}
      </button>
    {/if}
    {#if agentCount > 0}
      <button
        type="button"
        class="live-pill"
        onclick={openActivity}
        aria-label={`${agentCount} sub-agent${agentCount === 1 ? "" : "s"} running. Click to open Activity.`}
        use:tooltip={`${agentCount} sub-agent${agentCount === 1 ? "" : "s"} running. Click to open Activity.`}
      >
        <Bot size={12} />
        <span class="mono">{agentCount}</span>
      </button>
    {/if}
    {#if shellCount > 0}
      <button
        type="button"
        class="live-pill"
        onclick={openActivity}
        aria-label={`${shellCount} shell${shellCount === 1 ? "" : "s"} running. Click to open Activity.`}
        use:tooltip={`${shellCount} shell${shellCount === 1 ? "" : "s"} running. Click to open Activity.`}
      >
        <Terminal size={12} />
        <span class="mono">{shellCount}</span>
      </button>
    {/if}
    {#if toolCount > 0}
      <button
        type="button"
        class="live-pill"
        onclick={openActivity}
        aria-label={`${toolCount} tool${toolCount === 1 ? "" : "s"} running. Click to open Activity.`}
        use:tooltip={`${toolCount} tool${toolCount === 1 ? "" : "s"} running. Click to open Activity.`}
      >
        <Wrench size={12} />
        <span class="mono">{toolCount}</span>
      </button>
    {/if}
    {#if queue.length > 0}
      <button
        type="button"
        class="live-pill queued"
        onclick={openActivity}
        aria-label={`${queue.length} message${queue.length === 1 ? "" : "s"} queued to send after this turn.`}
        use:tooltip={`${queue.length} message${queue.length === 1 ? "" : "s"} queued to send after this turn.`}
      >
        <ListPlus size={12} />
        <span class="mono">{queue.length}</span>
      </button>
    {/if}
  </div>
{:else if composerFocused}
  <div class="kbd-hint" aria-hidden="true">
    <kbd>↵</kbd><span class="kh-t">send</span>
    <span class="kh-sep">·</span>
    <kbd>⇧↵</kbd><span class="kh-t">new line</span>
  </div>
{/if}

<style>
  /* One neutral capsule (same surface as the settings pill) holding quiet
     ghost stats split by hairline dividers — reads as a single intentional
     "live" readout rather than three loud floating badges. Color is held back
     to --fg-muted; the model hue is reserved for the one pulsing live dot so
     the cluster blends into the toolbar instead of competing with it. */
  .live-pills {
    display: inline-flex; align-items: center;
    height: 26px;
    padding: 0 2px;
    background: color-mix(in oklch, var(--bg-elev-2) 60%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 65%, transparent);
    border-radius: 999px;
    min-width: 0;
    animation: enter 180ms ease-out;
  }
  .live-pill {
    display: inline-flex; align-items: center; gap: 5px;
    height: 100%; padding: 0 9px;
    font: inherit; font-size: 11px; font-weight: 600; line-height: 1;
    color: var(--fg-muted);
    background: transparent;
    border: 0;
    border-radius: 999px;
    cursor: pointer;
    flex-shrink: 0;
    transition: color 140ms ease-out;
  }
  /* Hairline divider before every pill after the first. */
  .live-pill + .live-pill {
    box-shadow: inset 1px 0 0 color-mix(in oklch, var(--border) 55%, transparent);
  }
  .live-pill:hover { color: var(--fg); }
  .live-pill:active { transform: scale(0.97); }
  .live-pill:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }
  .live-pill :global(svg) { color: var(--fg-faint); transition: color 140ms ease-out; }
  .live-pill:hover :global(svg) { color: var(--fg-muted); }
  .live-pill .mono { font-variant-numeric: tabular-nums; color: var(--fg-2); }
  .live-pill .lp-sep { color: var(--fg-faint); margin: 0 1px; }
  .live-pill .lp-unit { color: var(--fg-faint); font-weight: 500; margin-left: 2px; }
  /* The one accent — a pulsing model-tinted dot marking the live turn. */
  .lp-dot {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--model-color);
    box-shadow: 0 0 6px color-mix(in oklch, var(--model-color) 65%, transparent);
    animation: lp-pulse 1.4s ease-in-out infinite;
  }
  @keyframes lp-pulse { 0%, 100% { opacity: 0.45; } 50% { opacity: 1; } }
  @media (prefers-reduced-motion: reduce) { .lp-dot { animation: none; } }
  /* Keyboard hint — occupies the toolbar's middle slot while the composer is
     focused (and no turn is live); keeps the idle bar empty + calm. */
  .kbd-hint {
    display: inline-flex; align-items: center; gap: 5px;
    font-size: 10.5px; color: var(--fg-faint);
    user-select: none; white-space: nowrap;
    animation: enter 160ms ease-out;
  }
  .kbd-hint .kh-t { letter-spacing: 0.01em; }
  .kbd-hint .kh-sep { color: var(--fg-subtle); opacity: 0.55; margin: 0 1px; }
  .kbd-hint kbd {
    display: inline-flex; align-items: center; justify-content: center;
    min-width: 16px; height: 16px; padding: 0 4px;
    font-family: var(--font-ui); font-size: 10px; font-weight: 600; line-height: 1;
    color: var(--fg-muted);
    background: color-mix(in oklch, var(--bg-elev-2) 70%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    border-radius: 4px;
  }
  @media (prefers-reduced-motion: reduce) { .kbd-hint { animation: none; } }
</style>

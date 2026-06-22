<script lang="ts">
  // C4 (per docs/design/composer-split.md) — the toolbar's middle slot, lifted
  // verbatim from Composer.svelte 2026-06-10: live-activity pills while a turn
  // runs (elapsed · tok/s · agents · shells · tools · queued) and the ↵/⇧↵
  // keyboard hint while idle-focused. Talks to the assistant store directly
  // (telemetry snapshot + Activity dock toggle — the cluster already reads it
  // pervasively, per the brief's QueueRail precedent).
  import { Bot, Terminal, Wrench, ListPlus } from "lucide-svelte";
  import type { ChatMessage } from "../../../state/assistant/types";
  import { liveActivity } from "../../../state/assistant/helpers";
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
  // The in-flight live-status label (whimsical word · elapsed · tokens) now lives
  // in StreamTurn's inline footer, under the turn's "Working…" head — matching the
  // DS reference (app/stream.jsx StreamFooter). This toolbar slot keeps only the
  // compact count pills (agents · shells · tools · queued).
  const showLivePills = $derived(agentCount > 0 || shellCount > 0 || toolCount > 0 || queue.length > 0);
</script>

{#if showLivePills}
  <div class="live-pills" role="group" aria-label="Live turn activity">
    {#if agentCount > 0}
      <span class="live-pill" use:tooltip={`${agentCount} sub-agent${agentCount === 1 ? "" : "s"} running.`}>
        <Bot size={12} />
        <span class="mono">{agentCount}</span>
      </span>
    {/if}
    {#if shellCount > 0}
      <span class="live-pill" use:tooltip={`${shellCount} shell${shellCount === 1 ? "" : "s"} running.`}>
        <Terminal size={12} />
        <span class="mono">{shellCount}</span>
      </span>
    {/if}
    {#if toolCount > 0}
      <span class="live-pill" use:tooltip={`${toolCount} tool${toolCount === 1 ? "" : "s"} running.`}>
        <Wrench size={12} />
        <span class="mono">{toolCount}</span>
      </span>
    {/if}
    {#if queue.length > 0}
      <span class="live-pill queued" use:tooltip={`${queue.length} message${queue.length === 1 ? "" : "s"} queued to send after this turn.`}>
        <ListPlus size={12} />
        <span class="mono">{queue.length}</span>
      </span>
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
    flex-shrink: 0;
    transition: color 140ms ease-out;
  }
  /* Hairline divider before every pill after the first. */
  .live-pill + .live-pill {
    box-shadow: inset 1px 0 0 color-mix(in oklch, var(--border) 55%, transparent);
  }
  .live-pill:hover { color: var(--fg); }
  .live-pill :global(svg) { color: var(--fg-faint); transition: color 140ms ease-out; }
  .live-pill:hover :global(svg) { color: var(--fg-muted); }
  .live-pill .mono { font-variant-numeric: tabular-nums; color: var(--fg-2); }
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

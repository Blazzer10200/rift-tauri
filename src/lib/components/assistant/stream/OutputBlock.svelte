<script lang="ts">
  import { untrack } from "svelte";
  import { ChevronDown, ChevronUp } from "lucide-svelte";
  import { splitOutput, nextRevealTier, type RevealTier } from "./streamModel";

  // Progressive-reveal terminal output. Long output caps at a glanceable head,
  // then steps up through "Show more" tiers (collapsed → expanded → all) instead
  // of dumping the reader into a tiny inner-scroll box. Only once EVERYTHING is
  // revealed and it's still tall does the block become a bounded scroll.
  let {
    text,
    start = "collapsed",
    live = false,
  }: { text: string; start?: RevealTier; live?: boolean } = $props();

  // Init once from the prop — the tier is user-driven (Show more / Collapse)
  // after mount, so it deliberately doesn't track later `start` changes.
  let tier = $state<RevealTier>(untrack(() => start));
  // While a command is still running, keep pinning to the tail so new output is
  // visible; the user stepping the tier up is respected once they touch it.
  let touched = $state(false);
  const view = $derived(splitOutput(text, tier));
  const next = $derived(nextRevealTier(tier, view.total));
  const shownLines = $derived(view.lines.slice(0, view.shown));
  // Reveal-all makes the block a bounded scroll only when there's genuinely a
  // lot; short "all" output just renders at its natural height.
  const scrolls = $derived(tier === "all" && view.total > 60);

  function more() { touched = true; if (next) tier = next; }
  function collapse() { touched = true; tier = "collapsed"; }

  // Live tail-follow: while streaming and untouched, jump the cap to "all" so the
  // freshest lines show as they land (bounded scroll keeps it from ballooning).
  $effect(() => {
    if (live && !touched) tier = "all";
  });
</script>

<div class="oblock">
  <pre class="oblock-out" class:scrolls>{shownLines.join("\n")}</pre>
  {#if next || tier !== "collapsed"}
    <div class="oblock-acts">
      {#if next}
        <button class="oblock-btn" type="button" onclick={more}>
          <ChevronDown size={12} strokeWidth={2.2} />
          Show {view.hidden} more line{view.hidden === 1 ? "" : "s"}
        </button>
      {/if}
      {#if tier !== "collapsed"}
        <button class="oblock-btn ghost" type="button" onclick={collapse}>
          <ChevronUp size={12} strokeWidth={2.2} />
          Collapse
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .oblock { display: flex; flex-direction: column; }
  .oblock-out {
    margin: 0; padding: 8px 11px;
    font-family: var(--font-mono); font-size: var(--fs-xs); line-height: 1.5;
    color: var(--fg-2); white-space: pre-wrap; word-break: break-word;
  }
  /* Only a fully-revealed, genuinely-long block scrolls — everything shorter
     renders at natural height so "Show more" is the primary affordance, not an
     inner scrollbar you have to fight. */
  .oblock-out.scrolls { max-height: 460px; overflow: auto; }
  .oblock-acts {
    display: flex; align-items: center; gap: 4px;
    padding: 5px 9px 6px; border-top: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
  }
  .oblock-btn {
    display: inline-flex; align-items: center; gap: 5px;
    height: 22px; padding: 0 8px; border-radius: 6px;
    border: 0; background: none; cursor: pointer;
    font: inherit; font-size: 11px; font-weight: 550;
    color: var(--fg-subtle);
    transition: background var(--dur-fast), color var(--dur-fast);
  }
  .oblock-btn:hover { background: var(--surface-hover); color: var(--fg-2); }
  .oblock-btn.ghost { color: var(--fg-faint); }
  .oblock-btn :global(svg) { flex: none; }
</style>

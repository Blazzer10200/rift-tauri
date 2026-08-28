<script lang="ts">
  import { untrack } from "svelte";
  import { ChevronDown, ChevronUp } from "@lucide/svelte";
  import {
    splitOutput, splitOutputFold, nextRevealTier, ansiLines, classifyShellLine,
    OUTPUT_CHAR_CAP,
    type RevealTier, type AnsiSeg,
  } from "./streamModel";

  // Progressive-reveal tool output — THE shared output body for both trees
  // (live stream + history ToolChip). Long output caps at a glanceable head,
  // then steps up through "Show more" tiers (collapsed → expanded → all)
  // instead of dumping the reader into a tiny inner-scroll box. Only once
  // EVERYTHING is revealed and it's still tall does the block become a bounded
  // scroll.
  //
  //  - tone "plain": one <pre>, text as-is (already ANSI-stripped upstream).
  //  - tone "shell": per-line render w/ real ANSI SGR color + a conservative
  //    semantic tone overlay (ok/err/warn keywords), terminal-style.
  //  - fold "head-tail": the visible budget splits around a "N lines hidden"
  //    divider so the END of output (summary/error) stays visible while folded.
  let {
    text,
    start = "collapsed",
    live = false,
    tone = "plain",
    fold = "head",
    cursor = false,
  }: {
    text: string;
    start?: RevealTier;
    live?: boolean;
    tone?: "plain" | "shell";
    fold?: "head" | "head-tail";
    // Terminal cursor parked on the line after the output — shown while the
    // command still runs and for a beat after output lands (print-settle).
    cursor?: boolean;
  } = $props();

  const reducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;

  // A pathological single-line blob (minified JSON, base64) has few lines but
  // can still freeze layout — hard char cap before any line math.
  const capped = $derived(text.length > OUTPUT_CHAR_CAP);
  const body = $derived((capped ? text.slice(0, OUTPUT_CHAR_CAP) : text).replace(/\s+$/, ""));

  // Init once from the prop — the tier is user-driven (Show more / Collapse)
  // after mount, so it deliberately doesn't track later `start` changes.
  let tier = $state<RevealTier>(untrack(() => start));
  let touched = $state(false);
  const view = $derived(splitOutput(body, tier));
  const foldView = $derived(fold === "head-tail" ? splitOutputFold(body, tier) : null);
  const next = $derived(nextRevealTier(tier, view.total));
  const collapsible = $derived(tier !== "collapsed" && nextRevealTier("collapsed", view.total) !== null);
  // Reveal-all makes the block a bounded scroll only when there's genuinely a
  // lot; short "all" output just renders at its natural height.
  const scrolls = $derived(tier === "all" && view.total > 60);

  // Shell tone: per-line ANSI segments (SGR state carried across lines) +
  // semantic tone per line. Indices align with view.lines.
  const segLines = $derived<AnsiSeg[][] | null>(tone === "shell" ? ansiLines(body) : null);
  const cleanLine = (i: number) =>
    segLines ? segLines[i].map((s) => s.text).join("") : view.lines[i];

  function more() { touched = true; if (next) tier = next; }
  function collapse() { touched = true; tier = "collapsed"; }

  // Live tail-follow: while streaming and untouched, jump the cap to "all" so
  // the freshest lines show as they land (bounded scroll contains it).
  $effect(() => {
    if (live && !touched) tier = "all";
  });

  // Staggered entrance — output arrives whole (results aren't token-streamed),
  // so a short per-line cascade reads as the terminal printing. Delay capped so
  // long dumps don't crawl; disabled under reduced motion.
  const lineDelay = (i: number) => (reducedMotion ? 0 : Math.min(i * 16, 400));
</script>

<div class="oblock" data-tone={tone}>
  {#if tone === "shell" && segLines}
    <div class="oblock-term" class:scrolls>
      {#if fold === "head-tail" && foldView && foldView.hidden > 0}
        {#each foldView.lines.slice(0, foldView.head) as _, i (i)}
          <div class="term-line {classifyShellLine(cleanLine(i))}" style:animation-delay="{lineDelay(i)}ms">{#each segLines[i] as seg, si (si)}<span class={seg.cls}>{seg.text}</span>{:else}{" "}{/each}</div>
        {/each}
        <button class="oblock-foldbtn" type="button" onclick={more}>
          ··· {foldView.hidden} line{foldView.hidden === 1 ? "" : "s"} hidden ···
        </button>
        {#each foldView.lines.slice(foldView.total - foldView.tail) as _, ti (ti)}
          {@const i = foldView.total - foldView.tail + ti}
          <div class="term-line {classifyShellLine(cleanLine(i))}" style:animation-delay="{lineDelay(foldView.head + ti)}ms">{#each segLines[i] as seg, si (si)}<span class={seg.cls}>{seg.text}</span>{:else}{" "}{/each}</div>
        {/each}
      {:else}
        {#each view.lines.slice(0, view.shown) as _, i (i)}
          <div class="term-line {classifyShellLine(cleanLine(i))}" style:animation-delay="{lineDelay(i)}ms">{#each segLines[i] as seg, si (si)}<span class={seg.cls}>{seg.text}</span>{:else}{" "}{/each}</div>
        {/each}
      {/if}
      {#if cursor && !reducedMotion}
        <div class="term-line term-cursorline" style:animation-delay="{lineDelay(view.shown)}ms"><span class="term-cursor" aria-hidden="true"></span></div>
      {/if}
    </div>
  {:else}
    <pre class="oblock-out" class:scrolls>{view.lines.slice(0, view.shown).join("\n")}</pre>
  {/if}
  {#if (next && !(fold === "head-tail" && foldView && foldView.hidden > 0)) || collapsible || capped}
    <div class="oblock-acts">
      {#if next && !(fold === "head-tail" && foldView && foldView.hidden > 0)}
        <button class="oblock-btn" type="button" onclick={more}>
          <ChevronDown size={12} strokeWidth={2.2} />
          Show {view.hidden} more line{view.hidden === 1 ? "" : "s"}
        </button>
      {/if}
      {#if collapsible}
        <button class="oblock-btn ghost" type="button" onclick={collapse}>
          <ChevronUp size={12} strokeWidth={2.2} />
          Collapse
        </button>
      {/if}
      {#if capped}
        <span class="oblock-capnote">output truncated</span>
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
  .oblock-out.scrolls, .oblock-term.scrolls { max-height: 460px; overflow: auto; }

  /* Terminal tone — per-line divs so ANSI segments + semantic tones can paint. */
  .oblock-term {
    padding: 8px 11px;
    font-family: var(--font-mono); font-size: var(--fs-xs); line-height: 1.55;
    color: var(--fg-2);
  }
  .term-line { white-space: pre-wrap; word-break: break-word;
    animation: termLineIn var(--dur-base) var(--ease-page) both; }
  @keyframes termLineIn { from { opacity: 0; transform: translateY(2px); } to { opacity: 1; transform: none; } }
  @media (prefers-reduced-motion: reduce) { .term-line { animation: none; } }
  /* Cursor parked on the next line after output — terminal print-settle. */
  .term-cursorline { line-height: 1.55; }
  .term-cursor { display: inline-block; width: 7px; height: 13px; border-radius: 1.5px;
    vertical-align: text-bottom;
    background: color-mix(in oklab, var(--accent) 80%, transparent);
    animation: termCurBlink 1.06s steps(1) infinite; }
  @keyframes termCurBlink { 0%, 49% { opacity: 1; } 50%, 100% { opacity: 0; } }
  /* Semantic tone (keyword-based) — the line's base color. */
  .term-line.ok   { color: var(--ok); }
  .term-line.err  { color: var(--danger); }
  .term-line.warn { color: var(--warn); }
  /* Real ANSI color — per-segment, wins over the line tone where present. */
  .term-line :global(.a-red)     { color: var(--danger); }
  .term-line :global(.a-green)   { color: var(--ok); }
  .term-line :global(.a-yellow)  { color: var(--warn); }
  .term-line :global(.a-blue)    { color: var(--info); }
  .term-line :global(.a-magenta) { color: oklch(0.75 0.14 330); }
  .term-line :global(.a-cyan)    { color: oklch(0.8 0.1 200); }
  .term-line :global(.a-white)   { color: var(--fg); }
  .term-line :global(.a-black)   { color: var(--fg-faint); }
  .term-line :global(.a-bold)    { font-weight: 650; }
  .term-line :global(.a-dim)     { opacity: 0.65; }

  /* Head+tail fold divider — the "more" affordance inside the output flow. */
  .oblock-foldbtn {
    display: block; width: 100%; margin: 2px 0; padding: 2px 0;
    border: 0; background: none; cursor: pointer;
    font-family: var(--font-mono); font-size: 10px; letter-spacing: 0.04em;
    color: var(--fg-faint); text-align: center;
    transition: color var(--dur-fast), background var(--dur-fast);
    border-radius: 5px;
  }
  .oblock-foldbtn:hover { color: var(--fg-2); background: var(--surface-hover); }

  .oblock-acts {
    display: flex; align-items: center; gap: 4px;
    padding: 5px 9px 6px; border-top: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
  }
  .oblock-btn {
    display: inline-flex; align-items: center; gap: 5px;
    height: 22px; padding: 0 8px; border-radius: 6px;
    border: 0; background: none; cursor: pointer;
    font: inherit; font-size: var(--fs-xs); font-weight: 550;
    color: var(--fg-subtle);
    transition: background var(--dur-fast), color var(--dur-fast);
  }
  .oblock-btn:hover { background: var(--surface-hover); color: var(--fg-2); }
  .oblock-btn.ghost { color: var(--fg-faint); }
  .oblock-btn :global(svg) { flex: none; }
  .oblock-capnote { margin-left: auto; font-size: 10px; font-style: italic; color: var(--fg-faint); }
</style>

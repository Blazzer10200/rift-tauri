<script lang="ts">
  import { tick, untrack } from "svelte";
  import { Check, ChevronDown, ChevronUp, Copy, Maximize2, X } from "@lucide/svelte";
  import { portal } from "$lib/actions/portal";
  import {
    splitOutput, splitOutputFold, nextRevealTier, ansiLines, classifyShellLine,
    stripAnsi, OUTPUT_CHAR_CAP,
    type RevealTier, type AnsiSeg,
  } from "./streamModel";

  // Progressive-reveal tool output — THE shared output body for both trees
  // (live stream + history ToolChip). Long output caps at a glanceable head,
  // then steps up through "Show more" tiers (collapsed → expanded → all)
  // instead of dumping the reader into a tiny inner-scroll box. Very long full
  // output opens in a dedicated inspector so the transcript keeps exactly one
  // vertical scrollbar.
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
  const opensInspector = $derived(next === "all" && view.total > 60);

  // Shell tone: per-line ANSI segments (SGR state carried across lines) +
  // semantic tone per line. Indices align with view.lines.
  const segLines = $derived<AnsiSeg[][] | null>(tone === "shell" ? ansiLines(body) : null);
  const cleanLine = (i: number) =>
    segLines ? segLines[i].map((s) => s.text).join("") : view.lines[i];

  let inspectorOpen = $state(false);
  let inspectorEl = $state<HTMLElement | null>(null);
  let inspectorTrigger = $state<HTMLButtonElement | null>(null);
  let copied = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | null = null;
  const inspectorText = $derived(tone === "shell" ? stripAnsi(body) : body);

  async function openInspector() {
    touched = true;
    inspectorOpen = true;
    await tick();
    inspectorEl?.focus();
  }
  function closeInspector() {
    inspectorOpen = false;
    void tick().then(() => inspectorTrigger?.focus());
  }
  async function copyInspector() {
    try {
      await navigator.clipboard.writeText(inspectorText);
      copied = true;
      if (copyTimer) clearTimeout(copyTimer);
      copyTimer = setTimeout(() => { copied = false; copyTimer = null; }, 1200);
    } catch (e) {
      console.warn("output copy failed", e);
    }
  }
  $effect(() => {
    if (!inspectorOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") { e.stopPropagation(); closeInspector(); }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  });
  $effect(() => () => { if (copyTimer) clearTimeout(copyTimer); });

  function more() {
    touched = true;
    if (opensInspector) { void openInspector(); return; }
    if (next) tier = next;
  }
  function collapse() { touched = true; tier = "collapsed"; }

  // Live output stays at the expanded inline tier. The CLI delivers command
  // output as a completed body, so revealing everything created a second
  // scrollbar without adding useful live feedback.
  $effect(() => {
    if (live && !touched) tier = "expanded";
  });

  // Staggered entrance — output arrives whole (results aren't token-streamed),
  // so a short per-line cascade reads as the terminal printing. Delay capped so
  // long dumps don't crawl; disabled under reduced motion.
  const lineDelay = (i: number) => (reducedMotion ? 0 : Math.min(i * 16, 400));
</script>

<div class="oblock" data-tone={tone}>
  {#if tone === "shell" && segLines}
    <div class="oblock-term">
      {#if fold === "head-tail" && foldView && foldView.hidden > 0}
        {#each foldView.lines.slice(0, foldView.head) as _, i (i)}
          <div class="term-line {classifyShellLine(cleanLine(i))}" style:animation-delay="{lineDelay(i)}ms">{#each segLines[i] as seg, si (si)}<span class={seg.cls}>{seg.text}</span>{:else}{" "}{/each}</div>
        {/each}
        <button class="oblock-foldbtn" bind:this={inspectorTrigger} type="button" onclick={more}>
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
    <pre class="oblock-out">{view.lines.slice(0, view.shown).join("\n")}</pre>
  {/if}
  {#if (next && !(fold === "head-tail" && foldView && foldView.hidden > 0)) || collapsible || capped}
    <div class="oblock-acts">
      {#if next && !(fold === "head-tail" && foldView && foldView.hidden > 0)}
        <button class="oblock-btn" class:inspect={opensInspector} bind:this={inspectorTrigger} type="button" onclick={more}>
          {#if opensInspector}<Maximize2 size={12} strokeWidth={2.2} />{:else}<ChevronDown size={12} strokeWidth={2.2} />{/if}
          {opensInspector ? `Open all ${view.total} lines` : `Show ${view.hidden} more line${view.hidden === 1 ? "" : "s"}`}
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

{#if inspectorOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="output-scrim" use:portal onclick={(e) => { if (e.target === e.currentTarget) closeInspector(); }}>
    <div class="output-inspector" bind:this={inspectorEl} role="dialog" aria-modal="true" aria-label="Full command output" tabindex="-1">
      <header class="oi-head">
        <div class="oi-title">
          <span>Output</span>
          <span class="oi-count">{view.total} line{view.total === 1 ? "" : "s"}</span>
          {#if capped}<span class="oi-capped">truncated</span>{/if}
        </div>
        <div class="oi-actions">
          <button type="button" class="oi-btn" onclick={copyInspector} aria-label="Copy full output">
            {#if copied}<Check size={13} /> Copied{:else}<Copy size={13} /> Copy{/if}
          </button>
          <button type="button" class="oi-close" onclick={closeInspector} aria-label="Close full output"><X size={15} /></button>
        </div>
      </header>
      <pre class="oi-body">{inspectorText}</pre>
    </div>
  </div>
{/if}

<style>
  .oblock { display: flex; flex-direction: column; }
  .oblock-out {
    margin: 0; padding: 8px 11px;
    font-family: var(--font-mono); font-size: var(--fs-xs); line-height: 1.5;
    color: var(--fg-2); white-space: pre-wrap; word-break: break-word;
  }
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
  .oblock-btn.inspect { color: var(--accent); }
  .oblock-btn.ghost { color: var(--fg-faint); }
  .oblock-btn :global(svg) { flex: none; }
  .oblock-capnote { margin-left: auto; font-size: 10px; font-style: italic; color: var(--fg-faint); }

  /* Full output leaves the transcript entirely. The inspector owns its own
     viewport, so the conversation never nests one scrollbar inside another. */
  .output-scrim {
    position: fixed; inset: 0; z-index: 1800;
    display: flex; justify-content: flex-end;
    background: rgb(0 0 0 / 0.48);
    animation: oiScrimIn 160ms ease-out both;
  }
  .output-inspector {
    width: min(760px, calc(100vw - 32px)); height: 100%; min-width: 0;
    display: grid; grid-template-rows: auto minmax(0, 1fr);
    background: var(--bg); border-left: 1px solid var(--border-strong);
    box-shadow: -24px 0 60px rgb(0 0 0 / 0.38);
    outline: none; animation: oiPanelIn 180ms var(--ease-page) both;
  }
  .oi-head { display: flex; align-items: center; justify-content: space-between; gap: 12px;
    min-height: 48px; padding: 0 12px 0 16px; border-bottom: 1px solid var(--border); }
  .oi-title, .oi-actions { display: inline-flex; align-items: center; gap: 8px; }
  .oi-title { min-width: 0; color: var(--fg); font-size: var(--fs-sm); font-weight: 650; }
  .oi-count { font-family: var(--font-mono); font-size: var(--fs-xs); font-weight: 500; color: var(--fg-faint); }
  .oi-capped { padding: 1px 6px; border-radius: 999px; font-size: 9.5px; color: var(--warn); background: var(--warn-soft); }
  .oi-btn, .oi-close { display: inline-flex; align-items: center; justify-content: center; gap: 6px;
    height: 28px; border: 0; border-radius: 7px; background: transparent; color: var(--fg-muted);
    font: inherit; font-size: var(--fs-xs); cursor: pointer; }
  .oi-btn { padding: 0 9px; }
  .oi-close { width: 28px; padding: 0; }
  .oi-btn:hover, .oi-close:hover { background: var(--surface-hover); color: var(--fg); }
  .oi-btn:focus-visible, .oi-close:focus-visible { outline: 0; box-shadow: 0 0 0 2px var(--ring); }
  .oi-body { min-width: 0; margin: 0; padding: 14px 16px 28px; overflow: auto;
    font-family: var(--font-mono); font-size: var(--fs-xs); line-height: 1.55;
    white-space: pre-wrap; overflow-wrap: anywhere; color: var(--fg-2); background: var(--bg-inset); }
  @keyframes oiScrimIn { from { opacity: 0; } to { opacity: 1; } }
  @keyframes oiPanelIn { from { opacity: 0; transform: translateX(18px); } to { opacity: 1; transform: none; } }
  @media (prefers-reduced-motion: reduce) {
    .output-scrim, .output-inspector { animation: none; }
  }
  @media (max-width: 620px) {
    .output-inspector { width: 100%; }
  }
</style>

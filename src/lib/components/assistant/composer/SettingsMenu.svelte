<script lang="ts">
  // C7 (per docs/design/composer-split.md) — the unified model / fast-mode /
  // effort settings popover, lifted verbatim from Composer.svelte 2026-06-10.
  // Keyboard nav stays parent-owned (onKey drives settingsIdx/activeKind and
  // ←/→ effort nudges); this child renders, handles clicks, and owns the
  // pointer-drag slider. Derives re-compute here from the shared modelMatrix
  // + assistant store — same pure helpers the parent uses, so they can't drift.
  import { Check, HelpCircle } from "lucide-svelte";
  import { tick } from "svelte";
  import { assistant } from "../../../state/assistant.svelte";
  import { uiPrefs } from "../../../state/ui-prefs.svelte";
  import { effortToFlag } from "../../../state/assistant/helpers";
  import { portal } from "$lib/actions/portal";
  import { tooltip } from "$lib/actions/tooltip";
  import { effortIdxFromX } from "./helpers";
  import {
    EFFORT_OPTIONS, MODEL_OPTIONS, modelShortcut, effortStopsFor, clampEffortIdx,
    type ModelOpt,
  } from "./modelMatrix";

  let {
    settingsIdx,
    activeKind,
    anchor,
    onPickModel,
    onRequestClose,
  }: {
    settingsIdx: number;
    activeKind: "model" | "effort" | null;
    anchor: HTMLElement | null;
    onPickModel: (m: ModelOpt) => void;
    onRequestClose: () => void;
  } = $props();

  // Portal to <body> + position against the trigger pill (same pattern as
  // PermMenu) — escapes the composer's overflow/backdrop-filter containing
  // block that was leaving this panel floating mid-app.
  let menuEl = $state<HTMLDivElement | null>(null);
  let pos = $state<{ top: number; left: number }>({ top: 0, left: 0 });
  function position() {
    if (!anchor || !menuEl) return;
    const a = anchor.getBoundingClientRect();
    const ph = menuEl.offsetHeight || 420;
    const pw = menuEl.offsetWidth || 320;
    let top = a.top - ph - 9;
    if (top < 8) top = a.bottom + 9;
    // right-align the panel to the pill's right edge
    let left = a.right - pw;
    const maxLeft = window.innerWidth - pw - 8;
    if (left > maxLeft) left = maxLeft;
    if (left < 8) left = 8;
    pos = { top, left };
  }
  function onDocMousedown(ev: MouseEvent) {
    if (anchor && ev.target instanceof Node && anchor.contains(ev.target)) return;
    if (menuEl && ev.target instanceof Node && menuEl.contains(ev.target)) return;
    onRequestClose();
  }
  $effect(() => {
    window.addEventListener("mousedown", onDocMousedown);
    return () => window.removeEventListener("mousedown", onDocMousedown);
  });
  $effect(() => {
    // re-position when the panel's height changes (effort slider show/hide, etc.)
    const _h = assistant.thinkingEnabled; const _m = assistant.effectiveModel; void _h; void _m;
    void tick().then(position);
  });
  // Mount-once resize listener — kept separate so a reposition-dep change above
  // doesn't tear down and re-register it every toggle.
  $effect(() => {
    const onResize = () => position();
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  });

  const currentModel = $derived(MODEL_OPTIONS.find((m) => m.id === assistant.effectiveModel));
  const currentEffort = $derived(EFFORT_OPTIONS.find((e) => e.id === assistant.thinkingEffort) ?? EFFORT_OPTIONS[2]);
  // The real CLI flag for the current tier — shown beside the tier name so the
  // panel never hides what actually gets sent.
  const effortFlagLabel = $derived(effortToFlag(assistant.thinkingEffort, assistant.effectiveModel));
  const effortApplies = $derived(currentModel?.effort ?? true);
  const effortStops = $derived(effortStopsFor(currentModel));
  const effortIdx = $derived(
    Math.min(
      Math.max(0, EFFORT_OPTIONS.findIndex((e) => e.id === assistant.thinkingEffort)),
      Math.max(0, effortStops.length - 1),
    ),
  );
  const stops = $derived(effortStops.length);
  const effortPct = $derived(stops > 1 ? (effortIdx / (stops - 1)) * 100 : 0);
  function setEffortByIdx(i: number) {
    const c = clampEffortIdx(effortStops, i);
    if (effortStops[c]) assistant.setThinkingEffort(effortStops[c].id);
  }
  // Plain-language summary of what the current model + effort selection actually
  // does — so users aren't guessing what a mode gets them into.
  const modelCaption = $derived.by(() => {
    const m = currentModel;
    if (!m) return "";
    if (!m.effort) return `${m.label} ${m.version} answers right away — it doesn't use extended-thinking effort modes.`;
    if (!assistant.thinkingEnabled) return `${m.label} ${m.version} replies without extended thinking — fastest, no reasoning step.`;
    return currentEffort.hint;
  });
  // Pointer-drag the slider: map clientX → nearest stop. Pointer-capture on the
  // track keeps move/up flowing even when the cursor leaves the row.
  let effortTrackEl: HTMLDivElement | null = $state(null);
  let draggingEffort = $state(false);
  function effortIdxFromClientX(clientX: number): number {
    if (!effortTrackEl) return effortIdx;
    return effortIdxFromX(clientX, effortTrackEl.getBoundingClientRect(), effortStops.length);
  }
  function startEffortDrag(e: PointerEvent) {
    e.preventDefault();
    draggingEffort = true;
    setEffortByIdx(effortIdxFromClientX(e.clientX));
    effortTrackEl?.setPointerCapture?.(e.pointerId);
  }
  function moveEffortDrag(e: PointerEvent) {
    if (!draggingEffort) return;
    setEffortByIdx(effortIdxFromClientX(e.clientX));
  }
  function endEffortDrag() { draggingEffort = false; }
</script>

<div
  class="rift-menu settings-menu"
  role="menu"
  bind:this={menuEl}
  use:portal
  style="top: {pos.top}px; left: {pos.left}px;"
>
  <div class="rift-menu-head">Model</div>
  {#each MODEL_OPTIONS as m, i (m.id)}
    {#if m.legacy && (i === 0 || !MODEL_OPTIONS[i - 1].legacy)}
      <div class="pop-label pop-label-sub">Legacy</div>
    {/if}
    {@const sel = m.id === assistant.effectiveModel}
    <button
      type="button"
      role="menuitemradio"
      aria-checked={sel}
      class="pop-item rich model-row"
      class:sel
      class:active={i === settingsIdx}
      class:limited={m.limited}
      use:tooltip={m.tagline}
      onmousedown={(e) => { e.preventDefault(); onPickModel(m); }}
    >
      <span class="pi-text">
        <span class="pi-name">
          <span class="model-name">{m.label} {m.version}</span>
          {#if m.limited}<span class="pi-tag accent">Limited</span>
          {:else if m.suffix}<span class="pi-tag">{m.suffix}</span>{/if}
        </span>
        <span class="pi-sub">{m.blurb}</span>
      </span>
      <span class="model-trail">
        {#if sel}
          <Check size={15} class="pop-ck" />
        {:else}
          <kbd class="model-num">{modelShortcut(m.id)}</kbd>
        {/if}
      </span>
    </button>
  {/each}

  {#if effortApplies}
    <div class="rift-menu-divider"></div>
    <button
      type="button"
      class="rift-menu-row toggle-row"
      onmousedown={(e) => { e.preventDefault(); assistant.toggleThinking(); }}
      role="menuitemcheckbox"
      aria-checked={assistant.thinkingEnabled}
    >
      <span class="rift-menu-row-body">
        <span class="rift-menu-row-t">Extended thinking</span>
        <span class="rift-menu-row-d">{assistant.thinkingEnabled ? "Reasons before replying" : "Off — replies immediately, faster"}</span>
      </span>
      <span class="rift-toggle" class:on={assistant.thinkingEnabled} aria-hidden="true">
        <span class="rift-toggle-knob"></span>
      </span>
    </button>
  {/if}
  {#if effortApplies && assistant.thinkingEnabled}
    <div class="effort-head" class:ultra={currentEffort.id === "ultra"}>
      <span class="effort-head-l">Effort <b>{currentEffort.label}</b>{#if effortFlagLabel}<span class="effort-head-flag" use:tooltip={"The effort level actually sent to Claude"}>{effortFlagLabel}</span>{/if}</span>
      <button
        type="button"
        role="menuitem"
        class="effort-help"
        use:tooltip={currentEffort.hint}
        onmousedown={(e) => e.preventDefault()}
        aria-label="What does effort do?"
      ><HelpCircle size={12} /></button>
    </div>
    <div
      class="effort-slider"
      class:active={activeKind === "effort"}
      class:ultra={currentEffort.id === "ultra"}
      class:dragging={draggingEffort}
      role="slider"
      tabindex="0"
      aria-label="Effort"
      onkeydown={(e) => { if (e.key === 'ArrowRight') { e.preventDefault(); setEffortByIdx(effortIdx + 1); } else if (e.key === 'ArrowLeft') { e.preventDefault(); setEffortByIdx(effortIdx - 1); } }}
      aria-valuemin={1}
      aria-valuemax={stops}
      aria-valuenow={effortIdx + 1}
      aria-valuetext={currentEffort.label}
    >
      <div
        class="effort-track"
        role="presentation"
        bind:this={effortTrackEl}
        onpointerdown={startEffortDrag}
        onpointermove={moveEffortDrag}
        onpointerup={endEffortDrag}
        onpointercancel={endEffortDrag}
      >
        <div class="effort-fill" style="width: {effortPct}%"></div>
        {#each effortStops as e, i (e.id)}
          <button
            type="button"
            class="effort-notch"
            class:on={i <= effortIdx}
            class:cur={i === effortIdx}
            class:ultra={e.id === "ultra"}
            style="left: {stops > 1 ? (i / (stops - 1)) * 100 : 0}%"
            use:tooltip={e.hint}
            aria-label={e.label}
            onmousedown={(ev) => { ev.preventDefault(); setEffortByIdx(i); }}
          ></button>
        {/each}
        <div class="effort-knob" style="left: {effortPct}%"></div>
      </div>
      <div class="effort-ends"><span>Faster</span><span>Smarter</span></div>
    </div>
  {/if}
  <p class="model-caption" class:warn={effortApplies && assistant.thinkingEnabled && currentEffort.id === "ultra"}>{modelCaption}</p>

  <div class="rift-menu-hint">
    <span><kbd>1–{MODEL_OPTIONS.length}</kbd>model</span>
    {#if effortApplies}<span><kbd>←→</kbd>effort</span>{/if}
    <span><kbd>↵</kbd>pick</span>
    <span><kbd>Esc</kbd>close</span>
  </div>
</div>

<style>
  /* Unified settings panel — flat single-column list (Claude-Code-Desktop
     layout) on the shared .rift-menu chrome: model rows, a fast-mode toggle,
     and a Faster↔Smarter effort slider. Right-anchored, content-width. */
  /* Panel — spec `.pop` glass popover (docs/design/rift-redesign.html). The
     compound selector overrides the shared .rift-menu base chrome. */
  :global(.rift-menu.settings-menu) {
    position: fixed;
    width: 264px; min-width: 240px;
    max-height: min(82vh, 560px);
    overflow-y: auto;
    z-index: 9998;
    padding: 6px; border-radius: 14px;
    transform-origin: bottom right;
    background:
      radial-gradient(130% 70% at 50% -10%,
        color-mix(in oklab, var(--accent) 6%, transparent), transparent 58%),
      color-mix(in oklab, var(--bg-elev-2) 58%, transparent);
    -webkit-backdrop-filter: blur(28px) saturate(1.7);
    backdrop-filter: blur(28px) saturate(1.7);
    border: 1px solid color-mix(in oklab, var(--fg) 12%, transparent);
    box-shadow:
      inset 0 1px 0 oklch(1 0 0 / 0.09),
      0 28px 64px -28px oklch(0 0 0 / 0.72),
      var(--shadow-lg);
    animation: pop-in 0.28s var(--ease-page) both;
  }
  /* pop-in keyframe → app.css (shared). */
  /* Per-child staggered rise — the list materializes top-to-bottom instead of
     all at once, giving the open a sense of assembly. Disabled below for
     reduced-motion. */
  @keyframes row-rise {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: none; }
  }
  :global(.settings-menu > *) { animation: row-rise 0.26s var(--ease-page) both; }
  :global(.settings-menu > :nth-child(1)) { animation-delay: 0.02s; }
  :global(.settings-menu > :nth-child(2)) { animation-delay: 0.04s; }
  :global(.settings-menu > :nth-child(3)) { animation-delay: 0.06s; }
  :global(.settings-menu > :nth-child(4)) { animation-delay: 0.08s; }
  :global(.settings-menu > :nth-child(5)) { animation-delay: 0.10s; }
  :global(.settings-menu > :nth-child(6)) { animation-delay: 0.12s; }
  :global(.settings-menu > :nth-child(n+7)) { animation-delay: 0.14s; }
  /* Section head — matched to PermMenu's `.pop-label` (uppercase, 700). */
  :global(.settings-menu .rift-menu-head) {
    display: flex; align-items: center; gap: 7px;
    font-size: 9.5px; font-weight: 700; letter-spacing: 0.1em; text-transform: uppercase;
    color: var(--fg-faint); padding: 6px 8px 5px;
  }
  :global(.settings-menu .pop-label-sub) {
    font-size: 9px; font-weight: 700; letter-spacing: 0.1em; text-transform: uppercase;
    color: var(--fg-faint); padding: 9px 8px 4px;
  }

  /* Compact model rows — no icon tile (text-led), tight PermMenu rhythm. */
  :global(.settings-menu .pop-item) {
    position: relative; overflow: hidden;
    display: flex; align-items: center; gap: 9px; width: 100%;
    padding: 7px 9px; border-radius: 9px; border: 0; background: transparent;
    color: var(--fg-2); cursor: pointer; font: inherit; text-align: left;
    transition: background var(--dur-fast), transform 0.18s var(--ease-page);
  }
  /* Sliding accent left-rail — grows from the row's vertical center on hover /
     selection, giving each row a tactile "lit" edge instead of a flat fill. */
  :global(.settings-menu .pop-item)::before {
    content: ""; position: absolute; left: 0; top: 50%; width: 2.5px; height: 0;
    border-radius: 0 3px 3px 0; background: var(--accent);
    transform: translateY(-50%); opacity: 0;
    transition: height 0.2s var(--ease-page), opacity 0.16s ease;
  }
  :global(.settings-menu .pop-item:hover),
  :global(.settings-menu .pop-item.active) {
    background: var(--surface-hover); color: var(--fg); transform: translateX(2px);
  }
  :global(.settings-menu .pop-item:hover)::before,
  :global(.settings-menu .pop-item.active)::before { height: 50%; opacity: 0.6; }
  :global(.settings-menu .pop-item:active) { transform: translateX(2px) scale(0.985); }
  :global(.settings-menu .pi-text) { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  :global(.settings-menu .pi-name) { display: flex; align-items: baseline; gap: 8px; font-size: 12.5px; font-weight: 600; color: var(--fg-2); line-height: 1.2; }
  :global(.settings-menu .pi-name .model-name) { flex: 0 0 auto; }
  /* Context-window tag — calm muted label, not a shouty uppercase badge. */
  :global(.settings-menu .pi-tag) {
    margin-left: auto; font-size: 10px; font-weight: 500; letter-spacing: 0;
    color: var(--fg-faint); font-variant-numeric: tabular-nums; white-space: nowrap;
  }
  :global(.settings-menu .pi-tag.accent) {
    color: var(--accent); padding: 2px 6px; border-radius: 999px; text-transform: none; letter-spacing: 0.02em;
    background: color-mix(in oklab, var(--accent) 14%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 35%, transparent);
  }
  :global(.settings-menu .pi-sub) {
    font-size: 11px; color: var(--fg-faint); line-height: 1.3;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  :global(.settings-menu .pop-item:hover .pi-name),
  :global(.settings-menu .pop-item.active .pi-name) { color: var(--fg); }
  /* selected model — accent-soft slab + full lit rail + checkmark */
  :global(.settings-menu .model-row.sel) {
    background:
      linear-gradient(90deg, color-mix(in oklab, var(--accent) 7%, transparent), transparent 70%),
      var(--accent-soft);
  }
  :global(.settings-menu .model-row.sel)::before {
    height: 62%; opacity: 1;
    box-shadow: 0 0 10px color-mix(in oklab, var(--accent) 55%, transparent);
  }
  :global(.settings-menu .model-row.sel .pi-name) { color: var(--fg); }
  :global(.settings-menu .model-row.limited .model-name) { color: var(--accent); }
  :global(.settings-menu .pop-ck) {
    flex: none; color: var(--accent);
    animation: ck-in 0.28s var(--ease-page) both;
  }
  @keyframes ck-in {
    from { opacity: 0; transform: scale(0.4) rotate(-12deg); }
    to { opacity: 1; transform: none; }
  }

  /* Fixed-width trailing slot so the ✓ (selected) and the number badge
     (unselected) occupy identical space — selecting a row never reflows it. */
  :global(.settings-menu .model-trail) {
    flex: none; display: inline-flex; align-items: center; justify-content: center; width: 20px;
  }
  :global(.settings-menu .model-num) {
    flex: none;
    font-family: var(--font-mono); font-size: 9.5px; font-weight: 600; line-height: 1;
    color: var(--fg-faint);
    background: color-mix(in oklab, var(--fg) 7%, transparent);
    border: 1px solid color-mix(in oklab, var(--fg) 10%, transparent);
    border-radius: 4px; padding: 2px 5px;
  }
  :global(.settings-menu .model-row:hover .model-num),
  :global(.settings-menu .model-row.active .model-num) { color: var(--fg-muted); }

  /* Fast-mode toggle row. */
  :global(.settings-menu .toggle-row) { align-items: center; padding: 7px 9px; }
  :global(.settings-menu .rift-toggle) {
    position: relative; flex-shrink: 0; align-self: center;
    width: 30px; height: 17px; border-radius: 999px;
    background: color-mix(in oklch, var(--fg-faint) 38%, transparent);
    border: 1px solid var(--border);
    transition: background 160ms ease, border-color 160ms ease;
  }
  :global(.settings-menu .rift-toggle.on) { background: var(--accent); border-color: transparent; }
  :global(.settings-menu .rift-toggle-knob) {
    position: absolute; top: 1px; left: 1px;
    width: 13px; height: 13px; border-radius: 999px;
    background: oklch(0.97 0 0);
    box-shadow: 0 1px 2px oklch(0 0 0 / 0.4);
    transition: transform 160ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  :global(.settings-menu .rift-toggle.on .rift-toggle-knob) { transform: translateX(13px); }

  /* Effort slider — Faster↔Smarter, stops = EFFORT_OPTIONS levels. */
  :global(.settings-menu .effort-head) {
    display: flex; align-items: center;
    padding: 6px 8px 3px;
    font-size: 11px; color: var(--fg-muted);
  }
  :global(.settings-menu .effort-head .effort-head-l) { letter-spacing: 0.01em; }
  :global(.settings-menu .effort-head .effort-head-l b) {
    color: var(--fg); font-weight: 650; margin-left: 2px;
    transition: color 180ms ease;
  }
  :global(.settings-menu .effort-head.ultra .effort-head-l b) { color: var(--accent); }
  /* Real --effort label — borderless quiet annotation, not a boxed badge. */
  :global(.settings-menu .effort-head .effort-head-flag) {
    margin-left: 7px;
    font-family: var(--font-mono);
    font-size: 9.5px;
    font-weight: 500;
    color: var(--fg-faint);
    letter-spacing: 0;
    opacity: 0.8;
    line-height: 1.4;
  }
  :global(.settings-menu .effort-head.ultra .effort-head-flag) {
    color: color-mix(in oklab, var(--accent) 70%, var(--fg-faint));
    opacity: 1;
  }
  :global(.settings-menu .effort-head .effort-help) {
    margin-left: auto; display: inline-flex; padding: 2px; border: 0;
    background: transparent; color: var(--fg-faint); cursor: help;
    transition: color 140ms ease;
  }
  :global(.settings-menu .effort-head .effort-help:hover) { color: var(--fg-muted); }
  :global(.settings-menu .effort-slider) {
    padding: 12px 14px 8px; margin: 0;
    transition: none;
  }
  /* Glassy capsule track — soft groove, no near-black well. A faint top sheen +
     1px inner ring read as a polished rail rather than a flat line. */
  :global(.settings-menu .effort-track) {
    position: relative; height: 6px; border-radius: 999px;
    background: linear-gradient(180deg,
      color-mix(in oklch, var(--fg) 7%, transparent),
      color-mix(in oklch, var(--fg) 13%, transparent));
    box-shadow: inset 0 0 0 1px oklch(1 0 0 / 0.05),
                inset 0 1px 1px oklch(0 0 0 / 0.18);
    cursor: grab; touch-action: none;
  }
  /* Invisible vertical hit-area so the 6px track is easy to grab. */
  :global(.settings-menu .effort-track::before) { content: ""; position: absolute; inset: -11px 0; }
  :global(.settings-menu .effort-slider.dragging .effort-track) { cursor: grabbing; }
  :global(.settings-menu .effort-fill) {
    position: absolute; left: 0; top: 0; height: 100%; border-radius: 999px;
    background: linear-gradient(
      90deg,
      color-mix(in oklab, var(--accent) 55%, var(--fg-faint)),
      var(--accent)
    );
    box-shadow: 0 0 10px color-mix(in oklab, var(--accent) 45%, transparent),
                inset 0 1px 0 oklch(1 0 0 / 0.18);
    transition: width 280ms cubic-bezier(0.22, 1, 0.36, 1),
                background 220ms ease, box-shadow 220ms ease;
  }
  :global(.settings-menu .effort-slider.ultra .effort-fill) {
    box-shadow: 0 0 14px color-mix(in oklab, var(--accent) 60%, transparent),
                inset 0 1px 0 oklch(1 0 0 / 0.22);
  }
  :global(.settings-menu .effort-notch) {
    position: absolute; top: 50%; width: 4px; height: 4px; padding: 0;
    transform: translate(-50%, -50%);
    border-radius: 999px; border: 0;
    background: oklch(1 0 0 / 0.22); cursor: pointer;
    transition: background 160ms ease, transform 220ms cubic-bezier(0.34, 1.4, 0.5, 1);
  }
  :global(.settings-menu .effort-notch:hover) { transform: translate(-50%, -50%) scale(1.6); }
  :global(.settings-menu .effort-notch.on) { background: oklch(1 0 0 / 0.5); }
  :global(.settings-menu .effort-notch.cur) { transform: translate(-50%, -50%) scale(0); }
  :global(.settings-menu .effort-knob) {
    position: absolute; top: 50%; width: 16px; height: 16px; z-index: 2;
    transform: translate(-50%, -50%);
    border-radius: 999px;
    background: radial-gradient(circle at 36% 28%, oklch(1 0 0), oklch(0.9 0 0) 60%, oklch(0.82 0 0));
    border: 2px solid var(--accent);
    box-shadow: 0 0 0 4px color-mix(in oklab, var(--accent) 16%, transparent),
                0 0 9px color-mix(in oklab, var(--accent) 40%, transparent),
                0 2px 5px oklch(0 0 0 / 0.45);
    transition: left 280ms cubic-bezier(0.34, 1.4, 0.5, 1),
                width 160ms ease, height 160ms ease,
                border-color 220ms ease, box-shadow 220ms ease;
    pointer-events: none;
  }
  :global(.settings-menu .effort-slider.dragging .effort-fill) { transition: background 220ms ease, box-shadow 220ms ease; }
  :global(.settings-menu .effort-slider.dragging .effort-knob) {
    transition: width 120ms ease, height 120ms ease, border-color 220ms ease, box-shadow 220ms ease;
    width: 18px; height: 18px;
  }
  :global(.settings-menu .effort-slider.active .effort-knob),
  :global(.settings-menu .effort-slider.ultra .effort-knob) {
    box-shadow: 0 0 0 5px color-mix(in oklab, var(--accent) 22%, transparent),
                0 0 13px color-mix(in oklab, var(--accent) 55%, transparent),
                0 2px 6px oklch(0 0 0 / 0.45);
  }
  :global(.settings-menu .effort-ends) {
    display: flex; justify-content: space-between;
    margin-top: 10px; font-size: 9px; font-weight: 600;
    letter-spacing: 0.06em; text-transform: uppercase; color: var(--fg-subtle);
  }
  /* Plain-language "what you're getting" line under the effort slider. Amber on
     the Ultracode tier to flag its higher cost / autonomous behavior. */
  :global(.settings-menu .model-caption) {
    margin: 8px 10px 2px; padding: 0;
    font-size: 10.5px; line-height: 1.4; color: var(--fg-muted);
    transition: color 180ms ease;
  }
  :global(.settings-menu .model-caption.warn) { color: var(--warn); }

  /* Footer hotkey strip — hairline top-rule closes the panel without a heavy
     inset slab; keys sit on the glass, not in a black well. */
  :global(.settings-menu .rift-menu-hint) {
    gap: 9px; flex-wrap: nowrap;
    margin: 6px 2px 1px; padding: 7px 6px 3px;
    border-top: 1px solid color-mix(in oklab, var(--fg) 8%, transparent);
    font-size: 9.5px; color: var(--fg-faint);
  }
  :global(.settings-menu .rift-menu-hint span) { display: inline-flex; align-items: center; white-space: nowrap; }
  :global(.settings-menu .rift-menu-hint kbd) {
    margin-right: 3px;
    background: color-mix(in oklab, var(--fg) 7%, transparent);
    border: 1px solid color-mix(in oklab, var(--fg) 10%, transparent);
  }

  @media (prefers-reduced-motion: reduce) {
    :global(.settings-menu),
    :global(.settings-menu > *),
    :global(.settings-menu .pop-ck),
    :global(.settings-menu .effort-fill),
    :global(.settings-menu .effort-knob),
    :global(.settings-menu .effort-notch) { animation: none; transition: none; }
  }
</style>

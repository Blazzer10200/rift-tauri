<script lang="ts">
  // C7 (per docs/design/composer-split.md) — the unified model / fast-mode /
  // effort settings popover, lifted verbatim from Composer.svelte 2026-06-10.
  // Keyboard nav stays parent-owned (onKey drives settingsIdx/activeKind and
  // ←/→ effort nudges); this child renders, handles clicks, and owns the
  // pointer-drag slider. Derives re-compute here from the shared modelMatrix
  // + assistant store — same pure helpers the parent uses, so they can't drift.
  import { Check, HelpCircle } from "lucide-svelte";
  import { assistant } from "../../../state/assistant.svelte";
  import { uiPrefs } from "../../../state/ui-prefs.svelte";
  import { effortToFlag } from "../../../state/assistant/helpers";
  import { tooltip } from "$lib/actions/tooltip";
  import { effortIdxFromX } from "./helpers";
  import {
    EFFORT_OPTIONS, MODEL_OPTIONS, modelShortcut, effortStopsFor, clampEffortIdx,
    FAST_MODE_WIRED, type ModelOpt,
  } from "./modelMatrix";

  let {
    settingsIdx,
    activeKind,
    onPickModel,
  }: {
    settingsIdx: number;
    activeKind: "model" | "fast" | "effort" | null;
    onPickModel: (m: ModelOpt) => void;
  } = $props();

  const currentModel = $derived(MODEL_OPTIONS.find((m) => m.id === assistant.effectiveModel));
  const currentEffort = $derived(EFFORT_OPTIONS.find((e) => e.id === assistant.thinkingEffort) ?? EFFORT_OPTIONS[2]);
  // The real CLI flag for the current tier — shown beside the tier name so the
  // panel never hides what actually gets sent.
  const effortFlagLabel = $derived(effortToFlag(assistant.thinkingEffort, assistant.effectiveModel));
  const effortApplies = $derived(currentModel?.effort ?? true);
  const effortStops = $derived(effortStopsFor(currentModel));
  const fastModeApplies = $derived((currentModel?.fastMode ?? false) && FAST_MODE_WIRED);
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

<div class="rift-menu settings-menu" role="menu">
  <div class="rift-menu-head">Model</div>
  {#each MODEL_OPTIONS as m, i (m.id)}
    {#if m.legacy && (i === 0 || !MODEL_OPTIONS[i - 1].legacy)}
      <div class="rift-menu-sub">Legacy</div>
    {/if}
    <button
      type="button"
      role="menuitemradio"
      aria-checked={m.id === assistant.effectiveModel}
      class="rift-menu-row model-row"
      class:active={i === settingsIdx}
      class:current={m.id === assistant.effectiveModel}
      use:tooltip={m.tagline}
      onmousedown={(e) => { e.preventDefault(); onPickModel(m); }}
    >
      <span class="rift-menu-row-t model-row-name" class:limited={m.limited}>{m.label} {m.version}</span>
      {#if m.limited}<span class="model-badge">Until Jun 22</span>{/if}
      {#if m.suffix}<span class="model-suffix" class:legacy={m.legacy}>{m.suffix}</span>{/if}
      {#if m.id === assistant.effectiveModel}
        <Check size={14} class="rift-menu-row-chk" />
      {:else}
        <kbd class="model-num">{modelShortcut(m.id)}</kbd>
      {/if}
    </button>
  {/each}

  {#if fastModeApplies}
    <div class="rift-menu-divider"></div>
    <div class="rift-menu-sub">Fast mode</div>
    <button
      type="button"
      class="rift-menu-row toggle-row"
      class:active={activeKind === "fast"}
      onmousedown={(e) => { e.preventDefault(); uiPrefs.toggleFastMode(); }}
      role="menuitemcheckbox"
      aria-checked={uiPrefs.fastMode}
    >
      <span class="rift-menu-row-body">
        <span class="rift-menu-row-t">Enable fast mode</span>
        <span class="rift-menu-row-d">Opus with faster output</span>
      </span>
      <span class="rift-toggle" class:on={uiPrefs.fastMode} aria-hidden="true">
        <span class="rift-toggle-knob"></span>
      </span>
    </button>
  {/if}

  <div class="rift-menu-divider"></div>
  {#if effortApplies}
    <div class="effort-head" class:ultra={currentEffort.id === "ultra"}>
      <span class="effort-head-l">Effort <b>{currentEffort.label}</b>{#if effortFlagLabel}<span class="effort-head-flag" use:tooltip={"The --effort level actually sent to Claude"}>--effort {effortFlagLabel}</span>{/if}</span>
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
  <p class="model-caption" class:warn={effortApplies && currentEffort.id === "ultra"}>{modelCaption}</p>

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
  .settings-menu {
    position: absolute;
    bottom: calc(100% + 8px);
    left: auto; right: 0;
    width: 320px;
    max-height: min(82vh, 600px);
    overflow-y: auto;
    z-index: 10;
    animation: slash-in 160ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes slash-in {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }
  /* Model row — name + muted suffix on one line, number shortcut / ✓ trailing. */
  .model-row { align-items: center; gap: 8px; }
  .model-row .model-row-name { flex: 0 0 auto; }
  .model-suffix {
    font-size: 11px; font-weight: 500; color: var(--fg-subtle);
    margin-right: auto;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .model-suffix.legacy { color: var(--fg-faint); }
  /* Limited-run row (Fable) — accent name + uppercase "until" chip. */
  .model-row-name.limited { color: var(--accent); }
  .model-badge {
    flex-shrink: 0;
    font-size: 9px; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase;
    line-height: 1; padding: 3px 6px; border-radius: 999px;
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 14%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 35%, transparent);
  }
  .model-row.current .model-suffix { color: color-mix(in oklab, var(--accent) 65%, var(--fg-muted)); }
  .model-num {
    flex-shrink: 0;
    font-family: var(--font-mono); font-size: 10px; font-weight: 600; line-height: 1;
    color: var(--fg-faint); background: var(--bg-inset);
    border: 1px solid var(--border); border-radius: 4px; padding: 2px 5px;
  }
  .model-row:hover .model-num, .model-row.active .model-num { color: var(--fg-muted); }

  /* Fast-mode toggle row. */
  .toggle-row { align-items: center; }
  .rift-toggle {
    position: relative; flex-shrink: 0; align-self: center;
    width: 30px; height: 17px; border-radius: 999px;
    background: color-mix(in oklch, var(--fg-faint) 38%, transparent);
    border: 1px solid var(--border);
    transition: background 160ms ease, border-color 160ms ease;
  }
  .rift-toggle.on { background: var(--accent); border-color: transparent; }
  .rift-toggle-knob {
    position: absolute; top: 1px; left: 1px;
    width: 13px; height: 13px; border-radius: 999px;
    background: oklch(0.97 0 0);
    box-shadow: 0 1px 2px oklch(0 0 0 / 0.4);
    transition: transform 160ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .rift-toggle.on .rift-toggle-knob { transform: translateX(13px); }

  /* Effort slider — Faster↔Smarter, stops = EFFORT_OPTIONS levels. */
  .effort-head {
    display: flex; align-items: center;
    padding: 6px 8px 3px;
    font-size: 11px; color: var(--fg-muted);
  }
  .effort-head .effort-head-l { letter-spacing: 0.01em; }
  .effort-head .effort-head-l b {
    color: var(--fg); font-weight: 650; margin-left: 2px;
    transition: color 180ms ease;
  }
  .effort-head.ultra .effort-head-l b { color: var(--accent); }
  .effort-head .effort-head-flag {
    margin-left: 8px;
    font-family: var(--font-mono);
    font-size: 9.5px;
    font-weight: 500;
    color: var(--fg-faint);
    letter-spacing: 0.02em;
  }
  .effort-head.ultra .effort-head-flag { color: color-mix(in oklab, var(--accent) 55%, var(--fg-faint)); }
  .effort-head .effort-help {
    margin-left: auto; display: inline-flex; padding: 2px; border: 0;
    background: transparent; color: var(--fg-faint); cursor: help;
    transition: color 140ms ease;
  }
  .effort-head .effort-help:hover { color: var(--fg-muted); }
  .effort-slider {
    padding: 13px 16px 8px; margin: 0 2px; border-radius: 11px;
    transition: background 160ms ease, box-shadow 160ms ease;
  }
  .effort-slider.active {
    background: var(--surface-hover);
    box-shadow: inset 0 0 0 1px var(--border);
  }
  .effort-track {
    position: relative; height: 5px; border-radius: 999px;
    background: color-mix(in oklch, var(--fg-faint) 24%, transparent);
    box-shadow: inset 0 1px 2px oklch(0 0 0 / 0.28);
    cursor: grab; touch-action: none;
  }
  /* Invisible vertical hit-area so the 5px track is easy to grab. */
  .effort-track::before { content: ""; position: absolute; inset: -11px 0; }
  .effort-slider.dragging .effort-track { cursor: grabbing; }
  .effort-fill {
    position: absolute; left: 0; top: 0; height: 100%; border-radius: 999px;
    background: linear-gradient(
      90deg,
      color-mix(in oklab, var(--accent) 68%, var(--fg-faint)),
      var(--accent)
    );
    box-shadow: 0 0 8px color-mix(in oklab, var(--accent) 42%, transparent);
    transition: width 260ms cubic-bezier(0.22, 1, 0.36, 1),
                background 220ms ease, box-shadow 220ms ease;
  }
  .effort-slider.ultra .effort-fill {
    background: linear-gradient(90deg, color-mix(in oklab, var(--accent) 68%, var(--fg-faint)), var(--accent));
    box-shadow: 0 0 11px color-mix(in oklab, var(--accent) 55%, transparent);
  }
  .effort-notch {
    position: absolute; top: 50%; width: 9px; height: 9px; padding: 0;
    transform: translate(-50%, -50%);
    border-radius: 999px; border: 1.5px solid var(--border-strong);
    background: var(--surface); cursor: pointer;
    transition: border-color 160ms ease, background 160ms ease,
                transform 220ms cubic-bezier(0.34, 1.4, 0.5, 1);
  }
  .effort-notch:hover { transform: translate(-50%, -50%) scale(1.3); }
  .effort-notch.on {
    border-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 24%, var(--surface));
  }
  .effort-notch.cur { transform: translate(-50%, -50%) scale(0); }
  .effort-slider.ultra .effort-notch.on { border-color: var(--accent); }
  .effort-knob {
    position: absolute; top: 50%; width: 15px; height: 15px; z-index: 2;
    transform: translate(-50%, -50%);
    border-radius: 999px;
    background: radial-gradient(circle at 35% 30%, oklch(1 0 0), var(--fg));
    border: 2px solid var(--accent);
    box-shadow: 0 0 0 4px color-mix(in oklab, var(--accent) 15%, transparent),
                0 2px 5px oklch(0 0 0 / 0.4);
    transition: left 260ms cubic-bezier(0.34, 1.4, 0.5, 1),
                border-color 220ms ease, box-shadow 220ms ease;
    pointer-events: none;
  }
  .effort-slider.dragging .effort-fill { transition: background 220ms ease, box-shadow 220ms ease; }
  .effort-slider.dragging .effort-knob { transition: border-color 220ms ease, box-shadow 220ms ease; }
  .effort-slider.active .effort-knob {
    box-shadow: 0 0 0 5px color-mix(in oklab, var(--accent) 22%, transparent),
                0 2px 6px oklch(0 0 0 / 0.45);
  }
  .effort-slider.ultra .effort-knob {
    border-color: var(--accent);
    box-shadow: 0 0 0 4px color-mix(in oklab, var(--accent) 22%, transparent),
                0 0 11px color-mix(in oklab, var(--accent) 55%, transparent),
                0 2px 5px oklch(0 0 0 / 0.45);
  }
  .effort-ends {
    display: flex; justify-content: space-between;
    margin-top: 9px; font-size: 9px; font-weight: 600;
    letter-spacing: 0.06em; text-transform: uppercase; color: var(--fg-faint);
  }
  /* Plain-language "what you're getting" line under the effort slider. Amber on
     the Ultracode tier to flag its higher cost / autonomous behavior. */
  .model-caption {
    margin: 8px 10px 2px; padding: 0;
    font-size: 10.5px; line-height: 1.4; color: var(--fg-muted);
    transition: color 180ms ease;
  }
  .model-caption.warn { color: var(--warn); }
  @media (prefers-reduced-motion: reduce) {
    .effort-fill, .effort-knob, .effort-notch { transition: none; }
  }
</style>

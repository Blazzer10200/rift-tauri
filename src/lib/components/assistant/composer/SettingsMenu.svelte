<script lang="ts">
  // C7 (per docs/design/composer-split.md) — the unified model / fast-mode /
  // effort settings popover, lifted verbatim from Composer.svelte 2026-06-10.
  // Keyboard nav stays parent-owned (onKey drives settingsIdx/activeKind and
  // ←/→ effort nudges); this child renders, handles clicks, and owns the
  // pointer-drag slider. Derives re-compute here from the shared modelMatrix
  // + assistant store — same pure helpers the parent uses, so they can't drift.
  import { Check, HelpCircle, ChevronRight, Layers, Plus } from "lucide-svelte";
  import { tick } from "svelte";
  import { assistant } from "../../../state/assistant.svelte";
  import { uiPrefs } from "../../../state/ui-prefs.svelte";
  import { modelFamily as modelFamilyOf } from "../../../state/assistant/helpers";
  import { portal } from "$lib/actions/portal";
  import { tooltip } from "$lib/actions/tooltip";
  import { effortIdxFromX } from "./helpers";
  import {
    MODEL_OPTIONS, currentModels, legacyModels, modelShortcut,
    dialStopsFor, dialIdFor, clampEffortIdx,
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
  // ── Thinking dial (one control: Off · Low · Medium · High · Max) ──────────
  // The dial is a pure projection over the store's (thinkingEnabled,
  // thinkingEffort) pair — `dialIdFor` reads them, `assistant.setThinkingDial`
  // writes both. Rungs truncate at the model's effort ceiling; Off-only for
  // models with no effort capability (Haiku).
  const dialStops = $derived(dialStopsFor(currentModel));
  const dialApplies = $derived(dialStops.length > 1); // a model with on-rungs
  const dialId = $derived(dialIdFor(assistant.thinkingEnabled, assistant.thinkingEffort));
  const dialIdx = $derived(
    Math.min(
      Math.max(0, dialStops.findIndex((s) => s.id === dialId)),
      Math.max(0, dialStops.length - 1),
    ),
  );
  const currentDial = $derived(dialStops[dialIdx] ?? dialStops[0]);
  const stops = $derived(dialStops.length);
  const effortPct = $derived(stops > 1 ? (dialIdx / (stops - 1)) * 100 : 0);
  function setDialByIdx(i: number) {
    const c = clampEffortIdx(dialStops, i);
    const s = dialStops[c];
    if (!s) return;
    if (s.effort === null) assistant.setThinkingDial(false);
    else assistant.setThinkingDial(true, s.effort);
  }
  // Plain-language summary of what the current model + thinking selection
  // actually does — so users aren't guessing what a mode gets them into.
  const modelCaption = $derived.by(() => {
    const m = currentModel;
    if (!m) return "";
    if (!m.effort) return `${m.label} ${m.version} answers right away — it doesn't use extended thinking.`;
    return currentDial.hint;
  });
  // Pointer-drag the dial: map clientX → nearest stop. Pointer-capture on the
  // track keeps move/up flowing even when the cursor leaves the row.
  let effortTrackEl: HTMLDivElement | null = $state(null);
  let draggingEffort = $state(false);
  function dialIdxFromClientX(clientX: number): number {
    if (!effortTrackEl) return dialIdx;
    return effortIdxFromX(clientX, effortTrackEl.getBoundingClientRect(), dialStops.length);
  }
  function startEffortDrag(e: PointerEvent) {
    e.preventDefault();
    draggingEffort = true;
    setDialByIdx(dialIdxFromClientX(e.clientX));
    effortTrackEl?.setPointerCapture?.(e.pointerId);
  }
  function moveEffortDrag(e: PointerEvent) {
    if (!draggingEffort) return;
    setDialByIdx(dialIdxFromClientX(e.clientX));
  }
  function endEffortDrag() { draggingEffort = false; }

  // "More models" flyout — previous-generation models live behind this submenu
  // (matches the desktop picker) instead of a permanently-expanded Legacy block.
  // Opens on hover/focus of the trigger row; the active model being a legacy one
  // surfaces it (accent dot + auto-open) so the current pick is never hidden.
  const activeIsLegacy = $derived(legacyModels.some((m) => m.id === assistant.effectiveModel));
  // Keep the flyout open whenever the active model OR the keyboard cursor lands
  // on a legacy row — otherwise arrowing onto Opus 4.7 highlights nothing.
  const cursorIsLegacy = $derived(!!MODEL_OPTIONS[settingsIdx]?.legacy);
  // Transient open-drivers: mouse hover + an explicit pin via the "More models"
  // click. `legacyOpen` is fully derived so it auto-CLOSES when the keyboard
  // cursor arrows back off a legacy row (no hover/pin) — the old one-way $effect
  // left it stuck open for keyboard users.
  let legacyHover = $state(false);
  let legacyPinned = $state(false);
  const legacyOpen = $derived(activeIsLegacy || cursorIsLegacy || legacyHover || legacyPinned);
</script>

<div
  class="rift-menu settings-menu"
  role="menu"
  bind:this={menuEl}
  use:portal
  style="top: {pos.top}px; left: {pos.left}px;"
>
  {#snippet modelRow(m: ModelOpt)}
    {@const sel = m.id === assistant.effectiveModel}
    {@const Icon = m.icon}
    <button
      type="button"
      role="menuitemradio"
      aria-checked={sel}
      class="pop-item rich model-row"
      class:sel
      class:active={MODEL_OPTIONS[settingsIdx]?.id === m.id}
      class:limited={m.limited}
      use:tooltip={m.tagline}
      onmousedown={(e) => { e.preventDefault(); onPickModel(m); }}
    >
      <span class="pi-ic" data-family={modelFamilyOf(m.id)}><Icon size={15} /></span>
      <span class="pi-text">
        <span class="pi-name">
          <span class="model-name">{m.label} {m.version}</span>
          {#if m.id === assistant.sessionPinnedModel && assistant.sessionModelDiverged}<span class="pi-tag session">this chat</span>
          {:else if m.limited}<span class="pi-tag accent">Limited</span>
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
  {/snippet}

  <div class="rift-menu-head">Model</div>
  {#each currentModels as m (m.id)}
    {@render modelRow(m)}
  {/each}

  {#if legacyModels.length > 0}
    <div
      class="legacy-zone"
      role="presentation"
      onmouseenter={() => (legacyHover = true)}
      onmouseleave={() => { legacyHover = false; }}
    >
      <button
        type="button"
        class="pop-item rich more-row"
        class:expanded={legacyOpen}
        class:sel={activeIsLegacy}
        aria-expanded={legacyOpen}
        onmousedown={(e) => { e.preventDefault(); legacyPinned = !legacyOpen; }}
        use:tooltip={"Previous-generation models"}
      >
        <span class="pi-ic more-ic"><Layers size={14} /></span>
        <span class="pi-text">
          <span class="pi-name"><span class="model-name">More models</span>
            {#if activeIsLegacy}<span class="pi-tag accent">Active</span>{/if}
          </span>
          <span class="pi-sub">Previous generations</span>
        </span>
        <ChevronRight size={15} class="more-chev" />
      </button>
      {#if legacyOpen}
        <div class="legacy-flyout">
          {#each legacyModels as m (m.id)}
            {@render modelRow(m)}
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  {#if assistant.sessionModelDiverged}
    {@const pinned = MODEL_OPTIONS.find((m) => m.id === assistant.sessionPinnedModel)}
    {@const picked = currentModel}
    <div class="session-note" role="note">
      <span class="sn-text">
        This chat runs on <strong>{pinned ? `${pinned.label} ${pinned.version}` : assistant.sessionPinnedModel}</strong>.
        Switching models only applies to a new chat.
      </span>
      <button
        type="button"
        class="sn-action"
        onmousedown={(e) => { e.preventDefault(); void assistant.newChatWithModel((picked ?? pinned!).id); onRequestClose(); }}
      >
        <Plus size={12} /> New chat in {picked ? picked.label : "this model"}
      </button>
    </div>
  {/if}

  {#if dialApplies}
    <div class="rift-menu-divider"></div>
    <div class="effort-head" class:ultra={currentDial.id === "max"}>
      <span class="effort-head-k">Thinking</span>
      <span class="effort-head-v" aria-live="polite">{currentDial.label}</span>
      <button
        type="button"
        role="menuitem"
        class="effort-help"
        use:tooltip={currentDial.hint}
        onmousedown={(e) => e.preventDefault()}
        aria-label="What does thinking do?"
      ><HelpCircle size={12} /></button>
    </div>
    <div
      class="effort-slider"
      class:active={activeKind === "effort"}
      class:ultra={currentDial.id === "max"}
      class:dragging={draggingEffort}
      role="slider"
      tabindex="0"
      aria-label="Thinking"
      onkeydown={(e) => { if (e.key === 'ArrowRight') { e.preventDefault(); setDialByIdx(dialIdx + 1); } else if (e.key === 'ArrowLeft') { e.preventDefault(); setDialByIdx(dialIdx - 1); } }}
      aria-valuemin={1}
      aria-valuemax={stops}
      aria-valuenow={dialIdx + 1}
      aria-valuetext={currentDial.label}
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
        <div class="effort-fill" class:off={dialIdx === 0} style="width: {effortPct}%"></div>
        {#each dialStops as s, i (s.id)}
          <button
            type="button"
            class="effort-notch"
            class:on={i <= dialIdx}
            class:cur={i === dialIdx}
            class:ultra={s.id === "max"}
            class:off={s.id === "off"}
            style="left: {stops > 1 ? (i / (stops - 1)) * 100 : 0}%"
            use:tooltip={s.hint}
            aria-label={s.label}
            onmousedown={(ev) => { ev.preventDefault(); setDialByIdx(i); }}
          ></button>
        {/each}
        <div class="effort-knob" style="left: {effortPct}%"></div>
      </div>
      <div class="effort-ticks">
        {#each dialStops as s, i (s.id)}
          <button
            type="button"
            class="effort-tick"
            class:cur={i === dialIdx}
            class:done={i < dialIdx}
            style="left: {stops > 1 ? (i / (stops - 1)) * 100 : 0}%"
            tabindex="-1"
            aria-hidden="true"
            onmousedown={(ev) => { ev.preventDefault(); setDialByIdx(i); }}
          >{s.label}</button>
        {/each}
      </div>
    </div>
  {/if}
  <p class="model-caption" class:warn={dialApplies && currentDial.id === "max"}>{modelCaption}</p>

  <div class="rift-menu-hint">
    <span><kbd>1–{MODEL_OPTIONS.length}</kbd>model</span>
    {#if dialApplies}<span><kbd>←→</kbd>thinking</span>{/if}
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
    width: 300px; min-width: 280px;
    max-height: min(82vh, 580px);
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
  /* One deliberate type scale for the whole panel — every label pulls from
     these so sizes/weights never drift row to row. */
  :global(.rift-menu.settings-menu) {
    --sm-eyebrow: 9.5px;   /* MODEL / EFFORT section heads */
    --sm-title: 13px;      /* model name, toggle title */
    --sm-sub: 11px;        /* blurb, toggle description */
    --sm-meta: 10px;       /* context tag */
    --sm-mono: 9.5px;      /* hotkeys, tick labels */
  }
  /* Section head — uppercase eyebrow, consistent with PermMenu's `.pop-label`. */
  :global(.settings-menu .rift-menu-head) {
    display: flex; align-items: center; gap: 7px;
    font-size: var(--sm-eyebrow); font-weight: 700; letter-spacing: 0.1em; text-transform: uppercase;
    color: var(--fg-faint); padding: 9px 9px 6px;
  }
  /* "More models" flyout — legacy generations fold behind this trigger row
     (desktop-picker parity). The flyout expands in-flow below the trigger with
     an indent rail, so it reads as a nested group, not a second panel. */
  :global(.settings-menu .legacy-zone) { position: relative; }
  :global(.settings-menu .more-row .more-ic) { color: var(--fg-faint); }
  :global(.settings-menu .more-row:hover .more-ic),
  :global(.settings-menu .more-row.expanded .more-ic) { color: var(--fg-muted); }
  :global(.settings-menu .more-row .more-chev) {
    flex: none; color: var(--fg-faint);
    transition: transform 0.24s var(--ease-page), color var(--dur-fast);
  }
  :global(.settings-menu .more-row.expanded .more-chev) { transform: rotate(90deg); color: var(--fg-muted); }
  :global(.settings-menu .more-row.sel) {
    background: linear-gradient(90deg, color-mix(in oklab, var(--accent) 6%, transparent), transparent 70%);
  }
  :global(.settings-menu .legacy-flyout) {
    position: relative; margin: 2px 0 0 9px;
    padding-left: 9px;
    border-left: 1.5px solid color-mix(in oklab, var(--fg) 10%, transparent);
    display: flex; flex-direction: column; gap: 1px;
    animation: flyout-open 0.24s var(--ease-page) both;
  }
  @keyframes flyout-open {
    from { opacity: 0; transform: translateY(-4px); max-height: 0; }
    to { opacity: 1; transform: none; max-height: 200px; }
  }
  /* Legacy rows sit a touch quieter than current-gen ones. */
  :global(.settings-menu .legacy-flyout .pi-name .model-name) { color: var(--fg-muted); }
  :global(.settings-menu .legacy-flyout .model-row.sel .pi-name .model-name) { color: var(--fg); }

  /* Compact model rows — no icon tile (text-led), tight PermMenu rhythm. */
  :global(.settings-menu .pop-item) {
    position: relative; overflow: hidden;
    display: flex; align-items: center; gap: 10px; width: 100%;
    padding: 8px 10px; border-radius: 10px; border: 0; background: transparent;
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
  /* Per-model identity tile — a small accent-tinted glyph that gives each row a
     visual anchor (matches PermMenu's icon language). Family-tinted so Opus /
     Sonnet / Haiku read distinct at a glance without shouting. */
  :global(.settings-menu .pi-ic) {
    flex: none; width: 27px; height: 27px; display: grid; place-items: center;
    border-radius: 8px; background: var(--surface); color: var(--fg-muted);
    border: 1px solid var(--border);
    transition: transform 0.34s var(--ease-page), background var(--dur-fast),
                color var(--dur-fast), border-color var(--dur-fast);
  }
  :global(.settings-menu .pop-item:hover .pi-ic),
  :global(.settings-menu .pop-item.active .pi-ic) {
    transform: scale(1.08) rotate(-3deg); border-color: var(--border-strong); color: var(--fg-2);
  }
  /* Selected model — tile lights up with the accent. */
  :global(.settings-menu .model-row.sel .pi-ic) {
    background: color-mix(in oklab, var(--accent) 16%, transparent);
    border-color: color-mix(in oklab, var(--accent) 40%, transparent);
    color: var(--accent);
  }
  :global(.settings-menu .pi-ic[data-family="opus"]) { color: color-mix(in oklab, var(--accent) 80%, var(--fg-muted)); }
  :global(.settings-menu .model-row.limited .pi-ic) { color: var(--accent); }
  :global(.settings-menu .pi-text) { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  :global(.settings-menu .pi-name) {
    display: flex; align-items: baseline; gap: 8px;
    font-size: var(--sm-title); font-weight: 600; color: var(--fg-2);
    line-height: 1.2; letter-spacing: -0.005em;
  }
  :global(.settings-menu .pi-name .model-name) { flex: 0 0 auto; }
  /* Context-window tag — calm muted label pinned to the row's right edge,
     tabular so "1M" / "200K" align across rows. */
  :global(.settings-menu .pi-tag) {
    margin-left: auto; font-size: var(--sm-meta); font-weight: 500; letter-spacing: 0;
    color: var(--fg-faint); font-variant-numeric: tabular-nums; white-space: nowrap;
  }
  :global(.settings-menu .pi-tag.accent) {
    margin-left: auto; font-size: 9px; font-weight: 600; letter-spacing: 0.04em; text-transform: uppercase;
    color: var(--accent); padding: 2px 6px; border-radius: 999px;
    background: color-mix(in oklab, var(--accent) 14%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 35%, transparent);
  }
  /* "this chat" tag — marks the model the active session is pinned to when the
     picker selection has drifted ahead of it. Neutral (not accent) so the
     accent stays on the user's current pick. */
  :global(.settings-menu .pi-tag.session) {
    margin-left: auto; font-size: 9px; font-weight: 600; letter-spacing: 0.03em;
    color: var(--fg-muted); padding: 2px 6px; border-radius: 999px;
    background: color-mix(in oklab, var(--fg) 8%, transparent);
    border: 1px solid color-mix(in oklab, var(--fg) 14%, transparent);
  }
  /* Pinned-session divergence note — honest "switch applies to a new chat"
     surface with a one-click "New chat in <model>" action. */
  .session-note {
    display: flex; flex-direction: column; gap: 7px;
    margin: 4px 6px; padding: 9px 11px;
    border-radius: 10px;
    background: color-mix(in oklab, var(--accent) 7%, var(--bg-inset));
    border: 1px solid color-mix(in oklab, var(--accent) 20%, var(--border));
  }
  .session-note .sn-text { font-size: 11px; line-height: 1.45; color: var(--fg-muted); }
  .session-note .sn-text strong { color: var(--fg); font-weight: 650; }
  .session-note .sn-action {
    display: inline-flex; align-items: center; gap: 5px; align-self: flex-start;
    padding: 4px 10px; border-radius: 8px;
    background: color-mix(in oklab, var(--accent) 16%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 38%, var(--border));
    color: color-mix(in oklab, var(--accent) 92%, var(--fg));
    font: inherit; font-size: 11px; font-weight: 600; cursor: pointer;
    transition: background 140ms ease, color 140ms ease;
  }
  .session-note .sn-action:hover {
    background: color-mix(in oklab, var(--accent) 28%, transparent);
    color: color-mix(in oklab, var(--accent) 98%, white);
  }
  /* Blurb wraps to two lines instead of a hard ellipsis — the wider panel fits
     every current blurb on one line, but wrapping guarantees nothing is ever
     clipped mid-word as the row content grows. */
  :global(.settings-menu .pi-sub) {
    font-size: var(--sm-sub); font-weight: 450; color: var(--fg-faint); line-height: 1.4;
    display: -webkit-box; -webkit-box-orient: vertical;
    -webkit-line-clamp: 2; line-clamp: 2;
    overflow: hidden;
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
    font-family: var(--font-mono); font-size: var(--sm-mono); font-weight: 600; line-height: 1;
    color: var(--fg-faint);
    background: color-mix(in oklab, var(--fg) 7%, transparent);
    border: 1px solid color-mix(in oklab, var(--fg) 10%, transparent);
    border-radius: 5px; padding: 3px 5px;
  }
  :global(.settings-menu .model-row:hover .model-num),
  :global(.settings-menu .model-row.active .model-num) { color: var(--fg-muted); }

  /* Thinking dial — stepped tier rail (Off→Max), stops = DIAL_STOPS. */
  :global(.settings-menu .effort-head) {
    display: flex; align-items: center; gap: 8px;
    padding: 10px 9px 4px;
    color: var(--fg-muted);
  }
  /* "EFFORT" eyebrow — identical treatment to the MODEL section head. */
  :global(.settings-menu .effort-head .effort-head-k) {
    font-size: var(--sm-eyebrow); font-weight: 700; letter-spacing: 0.1em; text-transform: uppercase;
    color: var(--fg-faint);
  }
  /* Active tier value — accent chip that re-pops on every change. */
  :global(.settings-menu .effort-head .effort-head-v) {
    font-family: var(--font-mono); font-size: 11px; font-weight: 600;
    color: var(--accent); letter-spacing: 0.01em; line-height: 1;
    padding: 2px 7px; border-radius: 6px;
    background: color-mix(in oklab, var(--accent) 12%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 26%, transparent);
    animation: effort-pop 0.3s var(--ease-page);
  }
  :global(.settings-menu .effort-head.ultra .effort-head-v) {
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 20%, transparent);
    border-color: color-mix(in oklab, var(--accent) 45%, transparent);
    box-shadow: 0 0 10px color-mix(in oklab, var(--accent) 35%, transparent);
  }
  @keyframes effort-pop {
    0% { transform: scale(0.86); opacity: 0.4; }
    55% { transform: scale(1.06); }
    100% { transform: scale(1); opacity: 1; }
  }
  :global(.settings-menu .effort-head .effort-help) {
    margin-left: auto; display: inline-flex; padding: 2px; border: 0;
    background: transparent; color: var(--fg-faint); cursor: help;
    transition: color 140ms ease;
  }
  :global(.settings-menu .effort-head .effort-help:hover) { color: var(--fg-muted); }
  :global(.settings-menu .effort-slider) {
    padding: 14px 16px 6px; margin: 0;
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
  /* Off rung (dial at index 0) — neutral, no accent glow, so "thinking off"
     reads as a quiet state rather than a lit tier. The knob still sits at the
     left edge over the bare track. */
  :global(.settings-menu .effort-fill.off) {
    background: color-mix(in oklab, var(--fg-faint) 55%, transparent);
    box-shadow: none;
  }
  :global(.settings-menu .effort-slider:has(.effort-fill.off) .effort-knob) {
    border-color: var(--fg-faint);
  }
  :global(.settings-menu .effort-notch) {
    position: absolute; top: 50%; width: 5px; height: 5px; padding: 0;
    transform: translate(-50%, -50%);
    border-radius: 999px; border: 0;
    background: oklch(1 0 0 / 0.26); cursor: pointer;
    box-shadow: 0 0 0 0 color-mix(in oklab, var(--accent) 40%, transparent);
    transition: background 160ms ease, box-shadow 200ms ease,
                transform 220ms cubic-bezier(0.34, 1.4, 0.5, 1);
  }
  :global(.settings-menu .effort-notch:hover) {
    transform: translate(-50%, -50%) scale(1.5);
    background: oklch(1 0 0 / 0.6);
    box-shadow: 0 0 0 4px color-mix(in oklab, var(--accent) 14%, transparent);
  }
  /* Passed stops glow faintly with the accent so the filled track reads as
     "tiers crossed", not just a colored bar. */
  :global(.settings-menu .effort-notch.on) {
    background: oklch(1 0 0 / 0.85);
    box-shadow: 0 0 5px color-mix(in oklab, var(--accent) 50%, transparent);
  }
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
  /* Per-stop tier labels under the rail — each notch names its flag (Low /
     Medium / High / X-High). Absolutely positioned so they sit dead-under their
     notch; the active one lifts + goes accent, passed ones brighten. */
  :global(.settings-menu .effort-ticks) {
    position: relative; height: 14px; margin-top: 13px;
  }
  :global(.settings-menu .effort-tick) {
    position: absolute; top: 0; transform: translateX(-50%);
    padding: 0; border: 0; background: transparent; cursor: pointer;
    font-family: var(--font-mono); font-size: 9px; font-weight: 600;
    letter-spacing: 0.02em; line-height: 1; white-space: nowrap;
    color: var(--fg-subtle);
    transition: color 200ms ease, transform 240ms cubic-bezier(0.34, 1.4, 0.5, 1), opacity 200ms ease;
  }
  /* First/last hug the rail ends so they don't clip the panel edge. */
  :global(.settings-menu .effort-tick:first-child) { transform: translateX(0); left: 0 !important; }
  :global(.settings-menu .effort-tick:last-child) { transform: translateX(-100%); }
  :global(.settings-menu .effort-tick:hover) { color: var(--fg-muted); }
  :global(.settings-menu .effort-tick.done) { color: var(--fg-faint); }
  :global(.settings-menu .effort-tick.cur) {
    color: var(--accent); transform: translateX(-50%) translateY(-2px) scale(1.08);
  }
  :global(.settings-menu .effort-tick.cur:first-child) { transform: translateX(0) translateY(-2px) scale(1.08); }
  :global(.settings-menu .effort-tick.cur:last-child) { transform: translateX(-100%) translateY(-2px) scale(1.08); }
  /* Plain-language "what you're getting" line under the effort slider. Amber on
     the Ultracode tier to flag its higher cost / autonomous behavior. */
  :global(.settings-menu .model-caption) {
    margin: 10px 10px 2px; padding: 0;
    font-size: var(--sm-sub); font-weight: 450; line-height: 1.45; color: var(--fg-muted);
    transition: color 180ms ease;
  }
  :global(.settings-menu .model-caption.warn) { color: var(--warn); }

  /* Footer hotkey strip — hairline top-rule closes the panel without a heavy
     inset slab; keys sit on the glass, not in a black well. */
  :global(.settings-menu .rift-menu-hint) {
    gap: 10px; flex-wrap: nowrap;
    margin: 8px 2px 1px; padding: 8px 6px 3px;
    border-top: 1px solid color-mix(in oklab, var(--fg) 8%, transparent);
    font-size: var(--sm-mono); color: var(--fg-faint); letter-spacing: 0.01em;
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
    :global(.settings-menu .effort-head-v),
    :global(.settings-menu .effort-tick),
    :global(.settings-menu .effort-notch) { animation: none; transition: none; }
  }
</style>

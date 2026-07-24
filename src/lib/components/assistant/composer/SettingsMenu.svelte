<script lang="ts">
  // C7 (per docs/design/composer-split.md) — the unified model / fast-mode /
  // effort settings popover, lifted verbatim from Composer.svelte 2026-06-10.
  // Keyboard nav stays parent-owned (onKey drives settingsIdx/activeKind and
  // ←/→ effort nudges); this child renders, handles clicks, and owns the
  // pointer-drag slider. Derives re-compute here from the shared modelMatrix
  // + assistant store — same pure helpers the parent uses, so they can't drift.
  import { Check, ChevronRight, HelpCircle, Plus, SpellCheck, Zap } from "lucide-svelte";
  import { tick } from "svelte";
  import { assistant, type TabState } from "../../../state/assistant.svelte";
  import { fastEligible } from "../../../state/assistant/helpers";
  import { usage, limitZone, type ScopedLimit } from "../../../state/usage.svelte";
  import { portal } from "$lib/actions/portal";
  import { tooltip } from "$lib/actions/tooltip";
  import {
    MODEL_OPTIONS, currentModels, legacyModels, modelShortcut, modelWindowSuffix,
    dialStopsFor, dialIdxFor, clampEffortIdx,
    type ModelOpt, type SettingsRow,
  } from "./modelMatrix";

  let {
    tab = null,
    settingsIdx,
    activeKind,
    anchor,
    onPickModel,
    onRequestClose,
  }: {
    tab?: TabState | null;
    settingsIdx: number;
    activeKind: SettingsRow["kind"] | null;
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
    // Only touch state on real movement — this runs every frame (follow loop).
    if (pos.top !== top || pos.left !== left) pos = { top, left };
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
  // Anchor-follow loop: the composer FLIPs between hero-center and docked
  // postures WHILE this panel is open (engage/disengage animation) — a
  // position() computed once mid-flight strands the panel floating mid-screen.
  // Following the live anchor rect every frame keeps it glued to the pill
  // through any layout motion (FLIP, resize, panel height changes); position()
  // no-ops state when nothing moved.
  $effect(() => {
    void tick().then(position);
    let raf = requestAnimationFrame(function follow() {
      position();
      raf = requestAnimationFrame(follow);
    });
    return () => cancelAnimationFrame(raf);
  });

  const currentModel = $derived(MODEL_OPTIONS.find((m) => m.id === assistant.modelFor(tab)));
  // Fast mode — surfaces only on fast-eligible (Opus-family) rows. The stored
  // global pref survives on other models but is inert there (send.ts gates it).
  const fastApplies = $derived(!!currentModel && fastEligible(currentModel.id));
  // ── Reasoning: ONE ladder ─────────────────────────────────────────────────
  // The old Thinking toggle + effort slider were two knobs over one wire lever
  // (see modelMatrix DIAL_STOPS). The ladder is the honest control: rung 0 =
  // fastest (thinking off, `--effort low`); each higher rung sends its flag.
  // Writes go through assistant.setThinkingDial so both backing fields flip
  // atomically (one cache-bust hint, not two).
  const effortStops = $derived(dialStopsFor(currentModel));
  const dialApplies = $derived(effortStops.length > 0); // a model with effort
  const effortIdx = $derived(dialIdxFor(effortStops, assistant.thinkingOnFor(tab), assistant.effortFor(tab)));
  const currentEffort = $derived(effortStops[effortIdx] ?? effortStops[0]);
  function setEffortByIdx(i: number) {
    const s = effortStops[clampEffortIdx(effortStops, i)];
    if (!s) return;
    if (s.effort === null) assistant.setThinkingDial(false, undefined, tab);
    else assistant.setThinkingDial(true, s.effort, tab);
  }
  // ── Detent-slider mechanics (owner call 2026-07-22) — slot-machine feel:
  // while dragging, the knob FREE-FOLLOWS the pointer (clamped to the rail) and
  // the nearest DIAL_STOPS gear engages live; on release the knob spring-snaps
  // home to its gear. The wire still only knows the discrete stops.
  let trackEl = $state<HTMLDivElement | null>(null);
  let sliderDragging = $state(false);
  let dragPct = $state(0); // raw pointer % while dragging — the free-follow position
  const effortPct = $derived(effortStops.length > 1 ? (effortIdx / (effortStops.length - 1)) * 100 : 0);
  const knobPct = $derived(sliderDragging ? dragPct : effortPct);
  function fracFromPointer(ev: PointerEvent): number {
    if (!trackEl) return effortStops.length > 1 ? effortIdx / (effortStops.length - 1) : 0;
    const r = trackEl.getBoundingClientRect();
    return Math.min(1, Math.max(0, (ev.clientX - r.left) / r.width));
  }
  function onRailPointerDown(ev: PointerEvent) {
    ev.preventDefault(); // keep composer focus — a blur here unmounts the menu
    sliderDragging = true;
    (ev.currentTarget as HTMLElement).setPointerCapture(ev.pointerId);
    const f = fracFromPointer(ev);
    dragPct = f * 100;
    setEffortByIdx(Math.round(f * (effortStops.length - 1)));
  }
  function onRailPointerMove(ev: PointerEvent) {
    if (!sliderDragging) return;
    const f = fracFromPointer(ev);
    dragPct = f * 100;
    setEffortByIdx(Math.round(f * (effortStops.length - 1)));
  }
  function onRailPointerUp() { sliderDragging = false; } // knob springs home to its gear
  // Plain-language caption per rung — short, concrete, no marketing voice.
  // Fable is special-cased: its thinking is ALWAYS ON server-side (an explicit
  // disable 400s), so no rung "responds immediately" there.
  const CAPTIONS: Record<string, string> = {
    low: "Fastest responses with minimal reasoning. Best for quick questions and small edits.",
    medium: "Balanced speed and reasoning. The recommended default for everyday work.",
    high: "Reasons longer before replying. For complex work where quality matters more than speed.",
    xhigh: "Maximum reasoning depth with autonomous workflows. Slowest and most thorough.",
  };
  const modelCaption = $derived.by(() => {
    const m = currentModel;
    if (!m) return "";
    if (!m.effort) return `${m.label} ${m.version} responds immediately — it doesn't use extended reasoning.`;
    if (m.id === "claude-fable-5") return `Fable always reasons before replying — higher effort reasons more deeply. Currently ${currentEffort.label}.`;
    return CAPTIONS[currentEffort.id] ?? "";
  });
  // Live per-model weekly limit (usage endpoint `limits[]`, model-scoped rows
  // only). Matched by display-name substring — the endpoint names buckets after
  // the model ("Claude Fable 5 weekly…"). Null when the account has no such
  // bucket or limits haven't loaded; the row simply shows no chip.
  function modelLimitFor(m: ModelOpt): ScopedLimit | null {
    const fam = m.id === "claude-fable-5" ? "fable"
      : m.id === "haiku" ? "haiku"
      : m.id.includes("sonnet") ? "sonnet"
      : "opus"; // opus alias + pinned claude-opus-4-x share the Opus bucket
    const ls = usage.rateLimits?.limits ?? [];
    return ls.find((l) => (l.scope?.model?.displayName ?? "").toLowerCase().includes(fam)) ?? null;
  }
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
  // Hover grace: a short leave-delay so grazing the row's edge doesn't snap the
  // flyout shut mid-reach — the fast open/close flicker read as broken.
  let legacyLeaveTimer: ReturnType<typeof setTimeout> | undefined;
  function legacyEnter() { clearTimeout(legacyLeaveTimer); legacyHover = true; }
  function legacyLeave() { legacyLeaveTimer = setTimeout(() => (legacyHover = false), 260); }
  const legacyOpen = $derived(activeIsLegacy || cursorIsLegacy || legacyHover || legacyPinned);
</script>

<!-- Root-level mousedown preventDefault: a click on panel padding/dividers/
     captions must NOT steal focus from the composer textarea — the blur would
     disengage the hero posture (composer flies back up, this toolbar unmounts,
     menu vanishes). Buttons inside already preventDefault; this catches the
     misclick zones between them. -->
<div
  class="rift-menu settings-menu"
  role="menu"
  tabindex="-1"
  bind:this={menuEl}
  use:portal
  style="top: {pos.top}px; left: {pos.left}px;"
  onmousedown={(e) => e.preventDefault()}
>
  {#snippet modelRow(m: ModelOpt)}
    {@const sel = m.id === assistant.effectiveModel}
    {@const lim = modelLimitFor(m)}
    <!-- Dense one-line row (Claude-Desktop density): name + inline tags left,
         meta + ✓/hotkey right. The blurb/tagline lives in the tooltip. -->
    <button
      type="button"
      role="menuitemradio"
      aria-checked={sel}
      class="pop-item model-row"
      class:sel
      class:active={MODEL_OPTIONS[settingsIdx]?.id === m.id}
      class:limited={m.limited}
      use:tooltip={`${m.tagline} — ${m.blurb}`}
      onmousedown={(e) => { e.preventDefault(); onPickModel(m); }}
    >
      <span class="pi-name">
        <span class="model-name">{m.label} {m.version}</span>
        {#if m.id === assistant.sessionPinnedModel && assistant.sessionModelDiverged}<span class="pi-tag session">this chat</span>
        {:else if m.suffix}<span class="pi-tag">{modelWindowSuffix(m.id, assistant.planCap)}</span>{/if}
        {#if lim}<span
          class="pi-usage zone-{limitZone(lim.percent, lim.severity)}"
          use:tooltip={`Your weekly ${m.label} limit — ${Math.round(lim.percent)}% used${lim.isActive ? " · in use now" : ""}`}
        >{Math.round(lim.percent)}%</span>{/if}
      </span>
      <span class="model-trail">
        {#if sel}
          <Check size={14} class="pop-ck" />
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
      onmouseenter={legacyEnter}
      onmouseleave={legacyLeave}
    >
      <button
        type="button"
        class="pop-item more-row"
        class:expanded={legacyOpen}
        class:sel={activeIsLegacy}
        aria-expanded={legacyOpen}
        onmousedown={(e) => { e.preventDefault(); legacyPinned = !legacyOpen; }}
        use:tooltip={"Previous-generation models"}
      >
        <span class="pi-name"><span class="model-name more-name">More models</span>
          {#if activeIsLegacy}<span class="pi-tag accent">Active</span>{/if}
        </span>
        <ChevronRight size={14} class="more-chev" />
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
        This chat has been running on <strong>{pinned ? `${pinned.label} ${pinned.version}` : assistant.sessionPinnedModel}</strong>.
        Your next message switches it to <strong>{picked ? `${picked.label} ${picked.version}` : "the new model"}</strong>.
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

  {#if fastApplies}
    <div class="rift-menu-divider"></div>
    <!-- Fast mode — Opus fast output. Row matches the model-row anatomy; the
         per-turn "fast" chip in the transcript is the honest confirmation.
         BILLING: pay-per-use (usage credits, NOT plan limits) — the cost line
         + amber ON treatment are the consent surface, mirroring the CLI's own
         "Fast mode ON · Draws from usage credits" disclosure. Don't quiet it. -->
    <button
      type="button"
      role="menuitemcheckbox"
      aria-checked={assistant.fastMode}
      class="pop-item rich fast-row"
      use:tooltip={"Fast mode — quicker Opus output, billed from your usage credits (pay-per-use, NOT your plan limits). Applies from your next message."}
      onmousedown={(e) => { e.preventDefault(); assistant.setFastMode(!assistant.fastMode); }}
    >
      <span class="fast-glyph" class:on={assistant.fastMode} aria-hidden="true"><Zap size={13} /></span>
      <span class="pi-text">
        <span class="pi-name"><span class="model-name">Fast mode</span>
          {#if assistant.fastMode}<span class="pi-tag warn-tag">pay-per-use</span>{/if}
        </span>
        <span class="pi-sub">Quicker Opus replies — <span class="fast-cost">draws from usage credits, not plan limits</span></span>
      </span>
      <span class="fast-switch" class:on={assistant.fastMode} aria-hidden="true"><i></i></span>
    </button>
  {/if}

  <div class="rift-menu-divider"></div>
  <!-- Autocorrect — opt-in typing-time typo fix (word-finish boundary). Pure
       FE, free, so the ON tint is accent, not the pay-per-use amber. -->
  <button
    type="button"
    role="menuitemcheckbox"
    aria-checked={assistant.autocorrect}
    class="pop-item rich ac-row"
    use:tooltip={"Autocorrect — fixes common typos as you finish each word. Skips slash commands, paths, and code-like text."}
    onmousedown={(e) => { e.preventDefault(); assistant.setAutocorrect(!assistant.autocorrect); }}
  >
    <span class="ac-glyph" class:on={assistant.autocorrect} aria-hidden="true"><SpellCheck size={13} /></span>
    <span class="pi-text">
      <span class="pi-name"><span class="model-name">Autocorrect</span></span>
      <span class="pi-sub">Fix typos as you type — applied when a word is finished</span>
    </span>
    <span class="fast-switch ac-switch" class:on={assistant.autocorrect} aria-hidden="true"><i></i></span>
  </button>

  {#if dialApplies}
    <div class="rift-menu-divider"></div>
    <!-- Effort — Faster↔Smarter DETENT SLIDER (owner call 2026-07-22, Claude-
         Desktop anatomy): a row of dot detents under a blocky machined thumb.
         Drag free-follows, release spring-snaps; same DIAL_STOPS wire mechanics
         — rung 0 replies immediately (thinking off on the wire). -->
    <div class="effort-head">
      <span class="effort-head-k">Effort</span>
      <span class="effort-head-cur">{currentEffort.label}</span>
      <span
        class="effort-help"
        use:tooltip={currentModel?.id === "claude-fable-5"
          ? "Fable always reasons before replying — effort sets how deeply."
          : "Higher effort reasons longer for better answers. Lower effort responds faster."}
        aria-label="About effort"
      ><HelpCircle size={12} /></span>
    </div>
    <div
      class="effort-rail"
      class:active={activeKind === "effort"}
      class:dragging={sliderDragging}
      role="radiogroup"
      tabindex="0"
      aria-label="Effort"
      onkeydown={(e) => { if (e.key === 'ArrowRight') { e.preventDefault(); setEffortByIdx(effortIdx + 1); } else if (e.key === 'ArrowLeft') { e.preventDefault(); setEffortByIdx(effortIdx - 1); } }}
    >
      <div class="er-ends" aria-hidden="true"><span>Faster</span><span>Smarter</span></div>
      <div
        class="er-track"
        role="presentation"
        style="--kx: {knobPct}%"
        bind:this={trackEl}
        onpointerdown={onRailPointerDown}
        onpointermove={onRailPointerMove}
        onpointerup={onRailPointerUp}
        onpointercancel={onRailPointerUp}
      >
        {#each effortStops as s, i (s.id)}
          <button
            type="button"
            role="radio"
            aria-checked={i === effortIdx}
            aria-label={s.label}
            class="er-stop"
            class:on={i === effortIdx}
            class:passed={i < effortIdx}
            class:max={i === effortStops.length - 1}
            style="left: {effortStops.length > 1 ? (i / (effortStops.length - 1)) * 100 : 0}%"
            use:tooltip={`${s.label} — ${s.hint.split(" — ")[1] ?? s.hint}`}
            onmousedown={(ev) => { ev.preventDefault(); setEffortByIdx(i); }}
          ><i></i></button>
        {/each}
        <div class="er-knob" class:xhigh={currentEffort.id === "xhigh"} style="left: {knobPct}%" aria-hidden="true"></div>
      </div>
    </div>
  {/if}
  <p class="model-caption" class:warn={dialApplies && currentEffort.id === "xhigh"}>{modelCaption}</p>
</div>

<style>
  /* Unified settings panel — flat single-column list (Claude-Code-Desktop
     layout) on the shared .rift-menu chrome: model rows, a fast-mode toggle,
     and a Faster↔Smarter effort slider. Right-anchored, content-width. */
  /* Panel — spec `.pop` glass popover (docs/design/rift-redesign.html). The
     compound selector overrides the shared .rift-menu base chrome. */
  /* Panel — flat professional surface (Claude-Desktop grade): near-solid fill,
     mild blur, quick pop-in. No radial glow, no per-row stagger theater.
     LOCKSTEP w/ PermMenu — keep the two recipes identical. */
  :global(.rift-menu.settings-menu) {
    position: fixed;
    width: 296px; min-width: 280px;
    max-height: min(82vh, 580px);
    overflow-y: auto;
    z-index: 9998;
    padding: 5px; border-radius: 12px;
    transform-origin: bottom right;
    background: color-mix(in oklab, var(--bg-elev-2) 94%, transparent);
    -webkit-backdrop-filter: blur(20px) saturate(1.4);
    backdrop-filter: blur(20px) saturate(1.4);
    border: 1px solid color-mix(in oklab, var(--fg) 10%, transparent);
    box-shadow:
      inset 0 1px 0 oklch(1 0 0 / 0.05),
      0 20px 48px -20px oklch(0 0 0 / 0.65),
      var(--shadow-lg);
    animation: pop-in var(--dur-base) var(--ease-page) both;
  }
  /* pop-in keyframe → app.css (shared). */
  /* One deliberate type scale for the whole panel — every label pulls from
     these so sizes/weights never drift row to row. */
  :global(.rift-menu.settings-menu) {
    --sm-eyebrow: 9.5px;   /* MODEL / EFFORT section heads */
    --sm-title: 12.5px;    /* model name, toggle title */
    --sm-sub: 11px;        /* fast-row cost line */
    --sm-meta: 10px;       /* context tag */
    --sm-mono: 10px;       /* hotkey digits */
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
  :global(.settings-menu .more-name) { color: var(--fg-muted); font-weight: 500; }
  :global(.settings-menu .more-row:hover .more-name) { color: var(--fg-2); }
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
    animation: flyout-open var(--dur-rise) var(--ease-page) both;
  }
  @keyframes flyout-open {
    from { opacity: 0; transform: translateY(-4px); max-height: 0; }
    to { opacity: 1; transform: none; max-height: 200px; }
  }
  /* Legacy rows sit a touch quieter than current-gen ones. */
  :global(.settings-menu .legacy-flyout .pi-name .model-name) { color: var(--fg-muted); }
  :global(.settings-menu .legacy-flyout .model-row.sel .pi-name .model-name) { color: var(--fg); }

  /* Rows — dense one-line text-led (Claude-Desktop density). Blurbs and
     taglines live in tooltips; the row is name + meta, nothing else. */
  :global(.settings-menu .pop-item) {
    position: relative;
    display: flex; align-items: center; gap: 8px; width: 100%;
    min-height: 30px; padding: 0 9px; border-radius: 8px; border: 0; background: transparent;
    color: var(--fg-2); cursor: pointer; font: inherit; text-align: left;
    transition: background var(--dur-fast), color var(--dur-fast);
  }
  :global(.settings-menu .pop-item:hover),
  :global(.settings-menu .pop-item.active) {
    background: var(--surface-hover); color: var(--fg);
  }
  /* Fast row is the ONE two-line row — its cost disclosure needs the space. */
  :global(.settings-menu .fast-row) { padding: 6px 9px; }
  :global(.settings-menu .pi-text) { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  :global(.settings-menu .pi-name) {
    flex: 1; min-width: 0;
    display: flex; align-items: baseline; gap: 7px;
    font-size: var(--sm-title); font-weight: 500; color: var(--fg-2);
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
  /* Live weekly-limit chip — % used of the account's model-scoped usage bucket
     (usage endpoint limits[]). Tinted by the shared limitZone thresholds so it
     agrees with the status bar + UsagePanel. Absent when no bucket matches. */
  :global(.settings-menu .pi-usage) {
    flex: none; margin-left: 6px;
    font-family: var(--font-mono); font-size: 9px; font-weight: 600; line-height: 1;
    font-variant-numeric: tabular-nums; white-space: nowrap;
    padding: 2px 5px; border-radius: 999px;
    color: var(--fg-faint);
    background: color-mix(in oklab, var(--fg) 6%, transparent);
    border: 1px solid color-mix(in oklab, var(--fg) 10%, transparent);
  }
  :global(.settings-menu .pi-usage.zone-warn) {
    color: var(--warn);
    background: color-mix(in oklab, var(--warn) 10%, transparent);
    border-color: color-mix(in oklab, var(--warn) 28%, transparent);
  }
  :global(.settings-menu .pi-usage.zone-hot) {
    color: var(--danger);
    background: color-mix(in oklab, var(--danger) 10%, transparent);
    border-color: color-mix(in oklab, var(--danger) 30%, transparent);
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
    transition: background var(--dur-fast) ease, color var(--dur-fast) ease;
  }
  .session-note .sn-action:hover {
    background: color-mix(in oklab, var(--accent) 28%, transparent);
    color: color-mix(in oklab, var(--accent) 98%, white);
  }
  /* Blurb wraps to two lines instead of a hard ellipsis — the wider panel fits
     every current blurb on one line, but wrapping guarantees nothing is ever
     clipped mid-word as the row content grows. */
  :global(.settings-menu .pi-sub) {
    font-size: 10.5px; font-weight: 450; color: var(--fg-faint); line-height: 1.35;
    display: -webkit-box; -webkit-box-orient: vertical;
    -webkit-line-clamp: 2; line-clamp: 2;
    overflow: hidden;
  }
  :global(.settings-menu .pop-item:hover .pi-name),
  :global(.settings-menu .pop-item.active .pi-name) { color: var(--fg); }
  /* selected model — quiet accent wash + static left rail + checkmark */
  :global(.settings-menu .model-row.sel) { background: var(--accent-soft); }
  :global(.settings-menu .model-row.sel)::before {
    content: ""; position: absolute; left: 0; top: 50%; transform: translateY(-50%);
    width: 2px; height: 60%; border-radius: 0 2px 2px 0; background: var(--accent);
  }
  :global(.settings-menu .model-row.sel .pi-name) { color: var(--fg); font-weight: 600; }
  :global(.settings-menu .model-row.limited .model-name) { color: var(--accent); }
  /* Coming-soon teaser — present but visibly not-yet: dimmed, inert, no hover lift. */
  :global(.settings-menu .model-row.soon) { cursor: default; opacity: 0.68; }
  :global(.settings-menu .model-row.soon:hover) { background: transparent; }
  :global(.settings-menu .model-row.soon:hover .pi-name) { color: var(--fg-muted); }
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
    flex: none; display: inline-flex; align-items: center; justify-content: center; width: 16px;
  }
  /* Hotkey digit — plain quiet numeral (Claude style), no box. */
  :global(.settings-menu .model-num) {
    flex: none;
    font-family: var(--font-mono); font-size: var(--sm-mono); font-weight: 500; line-height: 1;
    color: var(--fg-faint);
    background: transparent; border: 0; padding: 0;
  }
  :global(.settings-menu .model-row:hover .model-num),
  :global(.settings-menu .model-row.active .model-num) { color: var(--fg-muted); }

  /* Fast-mode row — same pop-item anatomy as model rows; the trailing switch
     is a compact track+knob. ON state is WARN amber, not accent: fast mode is
     pay-per-use (usage credits), and the color must say "costs money" the same
     way the bypass pill does — don't re-tint to accent. */
  :global(.settings-menu .fast-row .fast-cost) { color: var(--warn); }
  :global(.settings-menu .fast-row .pi-tag.warn-tag) {
    margin-left: auto; font-size: 9px; font-weight: 600; letter-spacing: 0.04em; text-transform: uppercase;
    color: var(--warn); padding: 2px 6px; border-radius: 999px;
    background: color-mix(in oklab, var(--warn) 12%, transparent);
    border: 1px solid color-mix(in oklab, var(--warn) 34%, transparent);
  }
  :global(.settings-menu .fast-glyph) { flex: none; display: inline-flex; align-self: flex-start; margin-top: 3px; color: var(--fg-faint); }
  :global(.settings-menu .fast-glyph.on) { color: var(--warn); }
  :global(.settings-menu .fast-switch) {
    flex: none; position: relative;
    width: 30px; height: 17px; border-radius: 999px;
    background: color-mix(in oklab, var(--fg) 10%, transparent);
    border: 1px solid color-mix(in oklab, var(--fg) 14%, transparent);
    transition: background var(--dur-fast) ease, border-color var(--dur-fast) ease;
  }
  :global(.settings-menu .fast-switch i) {
    position: absolute; top: 2px; left: 2px;
    width: 11px; height: 11px; border-radius: 50%;
    background: var(--fg-muted);
    transition: transform var(--dur-base) var(--ease-page), background var(--dur-fast) ease;
  }
  :global(.settings-menu .fast-switch.on) {
    background: color-mix(in oklab, var(--warn) 26%, transparent);
    border-color: color-mix(in oklab, var(--warn) 45%, transparent);
  }
  :global(.settings-menu .fast-switch.on i) { transform: translateX(13px); background: var(--warn); }

  /* Autocorrect row — free feature, so ON tint is accent (fast-switch anatomy,
     re-tinted; the amber is reserved for costs-money states). */
  :global(.settings-menu .ac-glyph) { flex: none; display: inline-flex; align-self: flex-start; margin-top: 3px; color: var(--fg-faint); }
  :global(.settings-menu .ac-glyph.on) { color: var(--accent); }
  :global(.settings-menu .ac-switch.on) {
    background: color-mix(in oklab, var(--accent) 26%, transparent);
    border-color: color-mix(in oklab, var(--accent) 45%, transparent);
  }
  :global(.settings-menu .ac-switch.on i) { background: var(--accent); }

  /* Effort — Faster↔Smarter stop-rail (Claude-Desktop flow). One tick per
     dialStopsFor rung; the active stop is the accent thumb; the current label
     rides the head. Discrete gears — clicking a tick / ←→ arrows, nothing
     continuous. */
  :global(.settings-menu .effort-head) {
    display: flex; align-items: baseline; gap: 8px;
    padding: 10px 9px 4px;
    color: var(--fg-muted);
  }
  /* "EFFORT" eyebrow — identical treatment to the MODEL section head. */
  :global(.settings-menu .effort-head .effort-head-k) {
    font-size: var(--sm-eyebrow); font-weight: 700; letter-spacing: 0.1em; text-transform: uppercase;
    color: var(--fg-faint);
  }
  /* Current rung label — the head names the setting (rail ticks stay bare). */
  :global(.settings-menu .effort-head .effort-head-cur) {
    font-size: 11.5px; font-weight: 600; color: var(--fg-2); line-height: 1;
  }
  /* Help glyph — hover carries the one-line explanation (Claude-Desktop "?"). */
  :global(.settings-menu .effort-help) {
    margin-left: auto; display: inline-flex; align-items: center; align-self: center;
    color: var(--fg-faint); cursor: default;
    transition: color var(--dur-fast);
  }
  :global(.settings-menu .effort-help:hover) { color: var(--fg-muted); }
  :global(.settings-menu .effort-rail) {
    margin: 0 8px 4px; padding: 4px 3px 6px;
    border-radius: 8px; outline: none;
    transition: box-shadow var(--dur-fast) ease;
  }
  /* Keyboard cursor parked on the effort row (Composer onKey ↑↓). */
  :global(.settings-menu .effort-rail.active) {
    box-shadow: 0 0 0 1px color-mix(in oklab, var(--accent) 32%, transparent);
  }
  /* Faster / Smarter ride ABOVE the rail (Claude-Desktop layout). */
  :global(.settings-menu .er-ends) {
    display: flex; justify-content: space-between;
    padding: 0 3px 6px;
    font-size: 10px; color: var(--fg-faint); line-height: 1; user-select: none;
  }
  /* Detent-slider anatomy (owner call 2026-07-23, third pass): the slider
     speaks the menu's SWITCH dialect — a slim pill track in the .fast-switch
     off-recipe, the traveled side tinted switch-ON accent, dot detents + the
     machined thumb riding on top. Drag free-follows; release spring-snaps to
     the nearest DIAL_STOPS gear. */
  :global(.settings-menu .er-track) {
    position: relative;
    height: 26px; margin: 0 12px; /* half-knob breathing room at 0% / 100% */
    cursor: grab; touch-action: none;
  }
  /* Pill track — .fast-switch off-state recipe, stretched. Extends past the
     stop range by the 12px knob margin so the thumb never overhangs its ends. */
  :global(.settings-menu .er-track)::before {
    content: ""; position: absolute; z-index: 0;
    left: -12px; right: -12px; top: 50%; height: 14px;
    transform: translateY(-50%);
    border-radius: 999px;
    background: color-mix(in oklab, var(--fg) 12%, transparent);
    border: 1px solid color-mix(in oklab, var(--fg) 20%, transparent);
    box-shadow: inset 0 1px 0 oklch(1 0 0 / 0.05);
    pointer-events: none;
  }
  /* Traveled range — switch-ON accent tint up to the thumb (right cap hides
     under it). Springs with the knob on release, raw-follows while dragging. */
  :global(.settings-menu .er-track)::after {
    content: ""; position: absolute; z-index: 0;
    left: -12px; top: 50%; height: 14px;
    width: calc(12px + var(--kx, 0%));
    transform: translateY(-50%);
    border-radius: 999px;
    background: color-mix(in oklab, var(--accent) 30%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 55%, transparent);
    box-shadow: inset 0 1px 0 oklch(1 0 0 / 0.08);
    pointer-events: none;
    transition: width var(--dur-base) var(--ease-spring);
  }
  :global(.settings-menu .effort-rail.dragging .er-track)::after { transition: none; }
  :global(.settings-menu .effort-rail.dragging .er-track) { cursor: grabbing; }
  :global(.settings-menu .er-stop) {
    position: absolute; top: 0; z-index: 1;
    width: 18px; height: 26px; padding: 0; margin: 0;
    transform: translateX(-50%);
    display: grid; place-items: center;
    background: transparent; border: 0; cursor: pointer;
  }
  :global(.settings-menu .er-stop i) {
    width: 5px; height: 5px; border-radius: 50%;
    background: color-mix(in oklab, var(--fg) 34%, transparent);
    transition: background var(--dur-fast), opacity var(--dur-fast), transform var(--dur-fast) ease;
  }
  :global(.settings-menu .er-stop:hover i) {
    background: color-mix(in oklab, var(--fg) 55%, transparent);
    transform: scale(1.3);
  }
  /* Detents behind the thumb glow a touch brighter — the traveled range. */
  :global(.settings-menu .er-stop.passed i) { background: color-mix(in oklab, var(--fg) 58%, transparent); }
  /* The engaged detent hides under the thumb (swallowed by the mechanism). */
  :global(.settings-menu .er-stop.on i) { opacity: 0; }
  /* Top-gear detent carries a quiet accent — the ceiling is visible at a glance. */
  :global(.settings-menu .er-stop.max i) { background: color-mix(in oklab, var(--accent) 70%, transparent); }
  /* Blocky machined thumb — rounded-rect, bevel via inset catches (slab
     language, §5 craft-not-illumination). */
  :global(.settings-menu .er-knob) {
    position: absolute; top: 50%; z-index: 2;
    width: 18px; height: 22px; border-radius: 6px;
    transform: translate(-50%, -50%);
    background: linear-gradient(180deg,
      color-mix(in oklab, var(--fg) 100%, transparent),
      color-mix(in oklab, var(--fg) 86%, transparent));
    border: 1px solid oklch(0 0 0 / 0.5);
    box-shadow:
      0 1px 3px oklch(0 0 0 / 0.5),
      inset 0 1px 0 oklch(1 0 0 / 0.4),
      inset 0 -1px 0 oklch(0 0 0 / 0.25);
    pointer-events: none;
    transition: left var(--dur-base) var(--ease-spring), transform var(--dur-fast) ease;
  }
  :global(.settings-menu .effort-rail:hover .er-knob) { transform: translate(-50%, -50%) scale(1.05); }
  /* Dragging: thumb free-follows raw (no easing lag) and lifts in the hand. */
  :global(.settings-menu .effort-rail.dragging .er-knob) {
    transition: transform var(--dur-fast) ease;
    transform: translate(-50%, -50%) scale(1.12);
    box-shadow:
      0 4px 10px oklch(0 0 0 / 0.5),
      inset 0 1px 0 oklch(1 0 0 / 0.35),
      inset 0 -1px 0 oklch(0 0 0 / 0.2);
  }
  /* X-High lit = the hot gear — a whisper of accent flags its cost/autonomy. */
  :global(.settings-menu .er-knob.xhigh) {
    box-shadow: 0 2px 6px oklch(0 0 0 / 0.45),
                inset 0 1px 0 oklch(1 0 0 / 0.35),
                0 0 10px color-mix(in oklab, var(--accent) 45%, transparent);
  }
  /* Plain-language "what you're getting" line under the rung cards. Amber on
     the X-High tier to flag its higher cost / autonomous behavior. */
  :global(.settings-menu .model-caption) {
    margin: 10px 10px 4px; padding: 0;
    font-size: var(--sm-sub); font-weight: 450; line-height: 1.45; color: var(--fg-muted);
    transition: color var(--dur-fast) ease;
  }
  :global(.settings-menu .model-caption.warn) { color: var(--warn); }

  @media (prefers-reduced-motion: reduce) {
    :global(.settings-menu),
    :global(.settings-menu .pop-ck),
    :global(.settings-menu .legacy-flyout),
    :global(.settings-menu .er-knob),
    :global(.settings-menu .er-track)::after,
    :global(.settings-menu .er-stop i) { animation: none; transition: none; }
  }
</style>

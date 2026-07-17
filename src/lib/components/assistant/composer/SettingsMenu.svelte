<script lang="ts">
  // C7 (per docs/design/composer-split.md) — the unified model / fast-mode /
  // effort settings popover, lifted verbatim from Composer.svelte 2026-06-10.
  // Keyboard nav stays parent-owned (onKey drives settingsIdx/activeKind and
  // ←/→ effort nudges); this child renders, handles clicks, and owns the
  // pointer-drag slider. Derives re-compute here from the shared modelMatrix
  // + assistant store — same pure helpers the parent uses, so they can't drift.
  import { ArrowUpRight, Check, ChevronRight, Cpu, HelpCircle, Plus, Zap } from "lucide-svelte";
  import { tick } from "svelte";
  import { assistant } from "../../../state/assistant.svelte";
  import { providers, providerModelCtx } from "../../../state/providers.svelte";
  import { workspace } from "../../../state/workspace.svelte";
  import { fastEligible } from "../../../state/assistant/helpers";
  import { usage, limitZone, type ScopedLimit } from "../../../state/usage.svelte";
  import { portal } from "$lib/actions/portal";
  import { tooltip } from "$lib/actions/tooltip";
  import {
    MODEL_OPTIONS, currentModels, legacyModels, modelShortcut, modelWindowSuffix,
    dialStopsFor, dialIdxFor, clampEffortIdx, providerEffortCaps, settingsRowsFor,
    type ModelOpt, type SettingsRow,
  } from "./modelMatrix";

  let {
    settingsIdx,
    activeKind,
    anchor,
    onPickModel,
    onPickProviderModel,
    onPickProvider,
    onRequestClose,
  }: {
    settingsIdx: number;
    activeKind: SettingsRow["kind"] | null;
    anchor: HTMLElement | null;
    onPickModel: (m: ModelOpt) => void;
    onPickProviderModel: (id: string) => void;
    onPickProvider: (id: string | null) => void;
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
    // re-position when the panel's height changes (ladder show/hide on model
    // swap, provider-mode row-count changes)
    const _m = assistant.effectiveModel; void _m;
    const _p = providers.active?.model; void _p;
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
  // Fast mode — surfaces only on fast-eligible (Opus-family) rows. The stored
  // global pref survives on other models but is inert there (send.ts gates it).
  const fastApplies = $derived(!!currentModel && fastEligible(currentModel.id));
  // ── Reasoning: ONE ladder ─────────────────────────────────────────────────
  // The old Thinking toggle + effort slider were two knobs over one wire lever
  // (see modelMatrix DIAL_STOPS). The ladder is the honest control: rung 0 =
  // fastest (thinking off, `--effort low`); each higher rung sends its flag.
  // Writes go through assistant.setThinkingDial so both backing fields flip
  // atomically (one cache-bust hint, not two).
  // Provider mode (a third-party brain is active): the panel lists the active
  // profile's models instead of the Claude table, and the ladder derives from
  // the PROFILE's effort capability (Kimi/DeepSeek/GLM honor the tiers;
  // Ollama/unknown hide it). Same DIAL_STOPS wire mechanics either way.
  const providerMode = $derived(providers.enabled);
  const effortStops = $derived(
    providerMode ? dialStopsFor(providerEffortCaps(providers.effortCapable)) : dialStopsFor(currentModel),
  );
  const dialApplies = $derived(effortStops.length > 0); // a model with effort
  // The SAME row list Composer navigates (settingsRowsFor) — cursor highlight
  // for pmodel/provider rows keys off it so keyboard + render can't drift.
  const rows = $derived(settingsRowsFor({
    providerMode,
    providerModels: providers.activeModels,
    providerIds: providers.list.map((p) => p.id),
    dialApplies,
  }));
  function cursorOnPModel(id: string): boolean {
    const r = rows[settingsIdx];
    return r?.kind === "pmodel" && r.id === id;
  }
  function cursorOnProvider(id: string): boolean {
    const r = rows[settingsIdx];
    return r?.kind === "provider" && r.id === id;
  }
  /** Ctx tag for a provider model — best-effort table, 200K conservative. */
  function providerCtxTag(id: string): string {
    const w = providerModelCtx(id);
    return w >= 1_000_000 ? "1M context" : `${Math.round(w / 1000)}K context`;
  }
  const effortIdx = $derived(dialIdxFor(effortStops, assistant.thinkingEnabled, assistant.thinkingEffort));
  const currentEffort = $derived(effortStops[effortIdx] ?? effortStops[0]);
  function setEffortByIdx(i: number) {
    const s = effortStops[clampEffortIdx(effortStops, i)];
    if (!s) return;
    if (s.effort === null) assistant.setThinkingDial(false);
    else assistant.setThinkingDial(true, s.effort);
  }
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
    if (providerMode) {
      const a = providers.active;
      if (!a) return "";
      if (!dialApplies) return `${a.model?.trim() || a.name} runs via ${a.name} — reasoning is managed by the endpoint.`;
      return CAPTIONS[currentEffort.id] ?? "";
    }
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
      : m.id === "sonnet" ? "sonnet"
      : "opus"; // opus + claude-opus-4-7 share the Opus bucket
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
        {:else if m.limited}<span class="pi-tag accent">Limited</span>
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

  {#if providerMode}
    <!-- Provider mode — the active profile's models replace the Claude table.
         "Experimental" lives in the section head (design doc §risks: degraded
         model behavior must never read as a Rift bug) instead of per-row
         badges, so long model ids keep one clean line. -->
    <div class="rift-menu-head">
      <Cpu size={11} aria-hidden="true" /> {providers.active?.name ?? "Provider"}
      <span class="head-tag">experimental</span>
    </div>
    {#each providers.activeModels as id, i (id)}
      {@const sel = id === providers.active?.model}
      <button
        type="button"
        role="menuitemradio"
        aria-checked={sel}
        class="pop-item model-row pm"
        class:sel
        class:active={cursorOnPModel(id)}
        use:tooltip={`${id} — via ${providers.active?.name ?? "provider"}. Switching keeps this chat.`}
        onmousedown={(e) => { e.preventDefault(); onPickProviderModel(id); }}
      >
        <span class="pi-name">
          <span class="model-name">{id}</span>
          <span class="pi-tag">{providerCtxTag(id)}</span>
        </span>
        <span class="model-trail">
          {#if sel}
            <Check size={14} class="pop-ck" />
          {:else if i < 9}
            <kbd class="model-num">{i + 1}</kbd>
          {/if}
        </span>
      </button>
    {/each}
  {:else}
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
  {/if}

  {#if dialApplies}
    <div class="rift-menu-divider"></div>
    <!-- Effort — segmented rung cards, one per CLI `--effort` flag. A slider
         implied a continuum that doesn't exist; these are four discrete gears.
         Rung 0 replies immediately (thinking off on the wire); higher rungs
         reason before replying at their depth. Every rung is live. -->
    <!-- Effort — Faster↔Smarter stop-rail (Claude-Desktop flow): N discrete
         stops on a track, active = accent thumb, current label in the head.
         Same DIAL_STOPS mechanics: click a stop / ←→ arrows; nothing continuous. -->
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
      role="radiogroup"
      tabindex="0"
      aria-label="Effort"
      onkeydown={(e) => { if (e.key === 'ArrowRight') { e.preventDefault(); setEffortByIdx(effortIdx + 1); } else if (e.key === 'ArrowLeft') { e.preventDefault(); setEffortByIdx(effortIdx - 1); } }}
    >
      <span class="er-end">Faster</span>
      <div class="er-track">
        {#each effortStops as s, i (s.id)}
          <button
            type="button"
            role="radio"
            aria-checked={i === effortIdx}
            aria-label={s.label}
            class="er-stop"
            class:on={i === effortIdx}
            class:passed={i < effortIdx}
            class:xhigh={s.id === "xhigh"}
            use:tooltip={`${s.label} — ${s.hint.split(" — ")[1] ?? s.hint}`}
            onmousedown={(ev) => { ev.preventDefault(); setEffortByIdx(i); }}
          ><i></i></button>
        {/each}
      </div>
      <span class="er-end">Smarter</span>
    </div>
  {/if}
  <p class="model-caption" class:warn={dialApplies && currentEffort.id === "xhigh"}>{modelCaption}</p>

  {#if providerMode}
    <!-- Brain switch + config jump — mouse-only footer (not in the keyboard
         row list; models + effort stay the arrow-nav surface). -->
    <div class="rift-menu-divider"></div>
    <button
      type="button"
      class="pop-item brain-row"
      use:tooltip={"Switch back to Claude — starts a fresh chat (different auth can't share a session)"}
      onmousedown={(e) => { e.preventDefault(); onPickProvider(null); }}
    >
      <span class="pi-name"><span class="model-name">Use Claude</span></span>
      <span class="model-trail"><ChevronRight size={14} class="more-chev" /></span>
    </button>
    <button
      type="button"
      class="pop-item brain-row"
      use:tooltip={"Models page — endpoints, keys, model lists, reasoning capability"}
      onmousedown={(e) => { e.preventDefault(); onRequestClose(); workspace.setActive("local-llm"); }}
    >
      <span class="pi-name"><span class="model-name">Manage providers</span></span>
      <span class="model-trail"><ArrowUpRight size={14} class="more-chev" /></span>
    </button>
  {:else if providers.list.length > 0}
    <!-- Saved providers — one-click brain switch (Claude Code parity: every
         brain reachable from the composer pill; config stays on the Models
         page). Rows ARE keyboard-navigable (settingsRowsFor appends them). -->
    <div class="rift-menu-divider"></div>
    <div class="rift-menu-head">Providers <span class="head-tag">experimental</span></div>
    {#each providers.list as p (p.id)}
      <button
        type="button"
        role="menuitemradio"
        aria-checked={false}
        class="pop-item model-row pm"
        class:active={cursorOnProvider(p.id)}
        use:tooltip={p.has_key || p.preset === "ollama" || p.preset === "litellm"
          ? `${p.name} — switch turns to ${p.base_url}. Starts a fresh chat.`
          : `${p.name} — no API key saved yet. Add one on the Models page first.`}
        onmousedown={(e) => { e.preventDefault(); onPickProvider(p.id); }}
      >
        <span class="pi-name">
          <Cpu size={12} class="prov-glyph" aria-hidden="true" />
          <span class="model-name">{p.name}</span>
          {#if p.model}<span class="pi-tag">{p.model}</span>{/if}
        </span>
        <span class="model-trail">
          {#if !p.has_key && p.preset !== "ollama" && p.preset !== "litellm"}<span class="pi-tag warn-tag">no key</span>{/if}
        </span>
      </button>
    {/each}
  {/if}
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
  /* Head-level "experimental" marker (provider sections) — one quiet badge in
     the eyebrow instead of per-row noise. */
  :global(.settings-menu .rift-menu-head .head-tag) {
    margin-left: auto;
    font-size: 8.5px; font-weight: 600; letter-spacing: 0.08em;
    color: var(--fg-faint);
    border: 1px solid color-mix(in oklab, var(--fg) 14%, transparent);
    border-radius: 4px; padding: 1px 5px;
  }
  /* Provider rows — long free-text model ids must ellipsize, never wrap. */
  :global(.settings-menu .model-row.pm .pi-name) { min-width: 0; }
  :global(.settings-menu .model-row.pm .model-name) {
    max-width: 158px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  :global(.settings-menu .model-row.pm .prov-glyph) { flex: none; color: var(--accent); }
  /* Footer brain-switch rows — quiet utility rows, same voice as "More models". */
  :global(.settings-menu .brain-row .model-name) { color: var(--fg-muted); font-weight: 500; }
  :global(.settings-menu .brain-row:hover .model-name) { color: var(--fg-2); }
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
    animation: flyout-open var(--dur-base) var(--ease-page) both;
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
    display: flex; align-items: center; gap: 10px;
    margin: 0 8px 4px; padding: 6px 3px;
    border-radius: 8px; outline: none;
    transition: box-shadow var(--dur-fast) ease;
  }
  /* Keyboard cursor parked on the effort row (Composer onKey ↑↓). */
  :global(.settings-menu .effort-rail.active) {
    box-shadow: 0 0 0 1px color-mix(in oklab, var(--accent) 32%, transparent);
  }
  :global(.settings-menu .er-end) {
    flex: none; font-size: 10px; color: var(--fg-faint); line-height: 1; user-select: none;
  }
  /* Claude-Desktop slider anatomy: a recessed 6px groove, faint dots inside,
     and a light round knob on the active stop. Still N discrete stops. */
  :global(.settings-menu .er-track) {
    position: relative; flex: 1;
    display: flex; align-items: center; justify-content: space-between;
    height: 20px; padding: 0 2px;
  }
  :global(.settings-menu .er-track)::before {
    content: ""; position: absolute; left: 0; right: 0; top: 50%;
    height: 6px; transform: translateY(-50%);
    background: color-mix(in oklab, var(--fg) 6%, transparent);
    border: 1px solid color-mix(in oklab, var(--fg) 7%, transparent);
    border-radius: 999px;
  }
  :global(.settings-menu .er-stop) {
    position: relative; z-index: 1;
    width: 16px; height: 20px; padding: 0;
    display: grid; place-items: center;
    background: transparent; border: 0; cursor: pointer;
  }
  :global(.settings-menu .er-stop i) {
    width: 3.5px; height: 3.5px; border-radius: 50%;
    background: color-mix(in oklab, var(--fg) 30%, transparent);
    transition: background var(--dur-fast), width var(--dur-base) var(--ease-page),
                height var(--dur-base) var(--ease-page), box-shadow var(--dur-fast);
  }
  :global(.settings-menu .er-stop:hover i) { background: color-mix(in oklab, var(--fg) 55%, transparent); }
  :global(.settings-menu .er-stop.on i) {
    width: 14px; height: 14px;
    background: var(--fg);
    box-shadow: 0 1px 4px oklch(0 0 0 / 0.5), inset 0 -1px 0 oklch(0 0 0 / 0.14);
  }
  /* X-High lit = the hot gear — a whisper of accent flags its cost/autonomy. */
  :global(.settings-menu .er-stop.xhigh.on i) {
    box-shadow: 0 1px 4px oklch(0 0 0 / 0.5),
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
    :global(.settings-menu .er-stop i) { animation: none; transition: none; }
  }
</style>

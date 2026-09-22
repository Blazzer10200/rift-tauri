<script lang="ts">
  // Shared usage/limit progress-bar primitive — the track+fill markup that was
  // duplicated (with per-site visual treatment) across UsagePanel's plan-limit
  // rows, StatusBar's compact usage pills, and AiHealthPage's plan-limit cards.
  // Callers still own their own width-clamp math, zone thresholds, and the
  // surrounding label/percentage/reset row — this owns only the bar itself.
  let {
    pct,
    zone = "ok",
    variant = "panel",
    live = false,
    class: className = "",
  }: {
    /** Already-clamped fill width, 0–100. */
    pct: number;
    /** Caller-computed severity bucket driving fill color. */
    zone?: string;
    /** Which call site's visual treatment to render. */
    variant?: "panel" | "card" | "compact";
    /** Adds the glow used for an in-use/active window (panel variant only). */
    live?: boolean;
    class?: string;
  } = $props();
</script>

<div class="limitbar limitbar-{variant} {className}" class:live data-zone={zone}>
  <span class="limitbar-fill" style="width:{pct}%"></span>
</div>

<style>
  .limitbar { border-radius: 999px; overflow: hidden; position: relative; }
  .limitbar-fill { display: block; height: 100%; border-radius: 999px; }

  /* panel — UsagePanel's .up-track / .up-fill */
  .limitbar-panel { height: 8px; background: var(--bg-inset); }
  .limitbar-panel .limitbar-fill {
    position: absolute; inset: 0 auto 0 0;
    transition: width var(--dur-slow) var(--ease-page);
    background: linear-gradient(90deg, oklch(0.62 0.15 var(--accent-h)), oklch(0.78 0.16 var(--accent-h)));
    animation: limitbar-grow 480ms var(--ease-page) backwards;
  }
  .limitbar-panel[data-zone="warn"] .limitbar-fill { background: linear-gradient(90deg, color-mix(in oklab, var(--warn) 80%, black), var(--warn)); }
  .limitbar-panel[data-zone="hot"] .limitbar-fill {
    background: linear-gradient(90deg, color-mix(in oklab, var(--danger) 80%, black), var(--danger));
    animation: limitbar-grow 480ms var(--ease-page) backwards, limitbar-pulse 2.2s ease-in-out 600ms infinite;
  }
  .limitbar-panel.live .limitbar-fill { box-shadow: 0 0 10px color-mix(in oklab, var(--accent) 45%, transparent); }
  @keyframes limitbar-grow { from { width: 0; } }
  @keyframes limitbar-pulse { 50% { filter: brightness(1.3); } }

  /* card — AiHealthPage's .ah-track / .ah-fill */
  .limitbar-card { height: 7px; background: var(--ghost-border); }
  .limitbar-card .limitbar-fill { background: var(--accent); transition: width 0.3s ease; }
  .limitbar-card[data-zone="warn"] .limitbar-fill { background: var(--warn); }
  .limitbar-card[data-zone="hot"] .limitbar-fill { background: var(--danger); }
  /* Hot only: a slow sheen sweeps the fill so motion = urgency, never decoration.
     ok/warn bars stay still. */
  .limitbar-card[data-zone="hot"] .limitbar-fill { position: relative; overflow: hidden; }
  .limitbar-card[data-zone="hot"] .limitbar-fill::after {
    content: ""; position: absolute; inset: 0; width: 45%;
    background: linear-gradient(90deg, transparent, color-mix(in oklab, white 42%, transparent), transparent);
    animation: limitbar-sheen 2.6s var(--ease-soft) 900ms infinite;
  }
  @keyframes limitbar-sheen { from { transform: translateX(-120%); } to { transform: translateX(260%); } }

  /* compact — StatusBar's .rl-bar / i */
  .limitbar-compact { width: 46px; height: 4px; background: color-mix(in oklab, var(--fg) 8%, transparent); }
  .limitbar-compact .limitbar-fill { background: var(--accent); transition: width var(--dur-slow) var(--ease-soft); }
  .limitbar-compact[data-zone="warn"] .limitbar-fill { background: var(--warn); }
  .limitbar-compact[data-zone="hot"] .limitbar-fill { background: var(--danger); box-shadow: 0 0 6px color-mix(in oklab, var(--danger) 55%, transparent); }
  @media (max-width: 720px) {
    .limitbar-compact { width: 34px; }
  }

  @media (prefers-reduced-motion: reduce) {
    .limitbar-fill, .limitbar-fill::after { animation: none !important; }
    /* StatusBar's rl-bar killed its width transition outright under reduced
       motion (panel/card variants only disabled the animation, kept the
       transition) — preserved per-variant rather than widened to all. */
    .limitbar-compact .limitbar-fill { transition: none; }
  }
</style>

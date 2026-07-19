<script lang="ts">
  // Shimmer placeholder block. A muted island-toned fill with a soft accent-
  // cooled sheen sweeping across; `delay` staggers the sweep phase so a column
  // of them reads alive, not in lockstep. Reduced-motion → a calm opacity breathe.
  type Props = {
    w?: string;
    h?: string;
    radius?: string;
    /** Sweep phase offset (ms) — stagger rows so they don't shimmer in unison. */
    delay?: number;
  };
  let { w = "100%", h = "12px", radius = "7px", delay = 0 }: Props = $props();
</script>

<span
  class="sk"
  style="width:{w};height:{h};border-radius:{radius};--sk-delay:{delay}ms"
  aria-hidden="true"
></span>

<style>
  .sk {
    position: relative;
    display: block;
    flex: none;
    overflow: hidden;
    background: color-mix(in oklab, var(--fg) 6.5%, transparent);
  }
  .sk::after {
    content: "";
    position: absolute;
    inset: 0;
    transform: translateX(-100%);
    background: linear-gradient(
      90deg,
      transparent 0%,
      color-mix(in oklab, var(--fg) 7%, transparent) 42%,
      color-mix(in oklab, var(--accent) 14%, transparent) 50%,
      color-mix(in oklab, var(--fg) 7%, transparent) 58%,
      transparent 100%
    );
    animation: sk-sweep 1.6s var(--ease-soft) infinite;
    animation-delay: var(--sk-delay, 0ms);
  }
  @keyframes sk-sweep {
    from { transform: translateX(-100%); }
    to   { transform: translateX(100%); }
  }
  @media (prefers-reduced-motion: reduce) {
    .sk::after { animation: none; }
    .sk { animation: sk-breathe 2.4s ease-in-out infinite; }
  }
  @keyframes sk-breathe {
    0%, 100% { opacity: 0.55; }
    50%      { opacity: 0.85; }
  }
</style>

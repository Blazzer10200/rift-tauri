<script lang="ts">
  // Odometer count — eases the displayed number toward `value` with a rAF tween
  // (ease-out) instead of snapping, and rolls the glyph on each settle. Spec:
  // rift-redesign.html `.acount` / `acRoll` ("rolls while live, settles on
  // completion"). Used for the live token readout + diff +N/−M counts so the
  // numbers *climb* instead of popping. `format` lets callers reuse fmtTokens.
  import { untrack } from "svelte";

  let {
    value,
    format = (n: number) => String(Math.round(n)),
    live = true,
    durationMs = 420,
  }: {
    value: number;
    format?: (n: number) => string;
    // When false (turn settled), snap to the final value w/ no tween.
    live?: boolean;
    durationMs?: number;
  } = $props();

  const reduced =
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;

  let shown = $state(0);
  let raf = 0;
  let from = 0;
  let start = 0;
  // Roll direction + a key that flips whenever the rendered text changes, so the
  // glyph re-mounts and replays the var(--dur-fast) roll keyframe.
  let dir = $state<"up" | "down">("up");
  let rollKey = $state(0);
  let lastText = "";

  const easeOut = (t: number) => 1 - Math.pow(1 - t, 3);

  function tick(ts: number) {
    if (!start) start = ts;
    const t = Math.min(1, (ts - start) / durationMs);
    shown = from + (value - from) * easeOut(t);
    if (t < 1) {
      raf = requestAnimationFrame(tick);
    } else {
      shown = value;
      raf = 0;
    }
  }

  $effect(() => {
    const target = value;
    if (reduced || !live) {
      shown = target;
      return;
    }
    untrack(() => {
      if (Math.abs(target - shown) < 0.5) return;
      dir = target >= shown ? "up" : "down";
      from = shown;
      start = 0;
      if (raf) cancelAnimationFrame(raf);
      raf = requestAnimationFrame(tick);
    });
    return () => {
      if (raf) cancelAnimationFrame(raf);
      raf = 0;
    };
  });

  const text = $derived(format(shown));
  // Bump rollKey on text change (the visible glyph actually moved).
  $effect(() => {
    if (text !== lastText) {
      lastText = text;
      untrack(() => (rollKey = rollKey + 1));
    }
  });
</script>

<span class="acount" class:live={live && !reduced} data-dir={dir}
  >{#key rollKey}<span class="acount-v">{text}</span>{/key}</span>

<style>
  /* The number must sit on the SAME text baseline as the adjacent unit ("2s",
     "312 tokens"). The glyph wrapper MUST stay `display:inline` — ANY
     inline-block (even without overflow) aligns ~2.4px off the text baseline,
     which is what made the digit ride higher than the "s"/"tokens" run
     (CDP-measured: inline-block → +2.4px, inline → 0.0px). A plain inline
     element still paints `transform` for the roll, and `clip-path` (which does
     NOT change box type or baseline) clips the roll overflow — so we keep the
     animation without `inline-block` or `overflow`. */
  .acount {
    font-variant-numeric: tabular-nums;
  }
  .acount-v {
    display: inline;
    clip-path: inset(0 -0.15em);
  }
  .acount.live .acount-v { animation: acRoll var(--dur-fast) cubic-bezier(0.22, 1, 0.36, 1); }
  .acount.live[data-dir="down"] .acount-v { animation: acRollDown var(--dur-fast) cubic-bezier(0.22, 1, 0.36, 1); }
  @keyframes acRoll {
    from { transform: translateY(0.62em); opacity: 0.25; }
    to   { transform: none; opacity: 1; }
  }
  @keyframes acRollDown {
    from { transform: translateY(-0.62em); opacity: 0.25; }
    to   { transform: none; opacity: 1; }
  }
  @media (prefers-reduced-motion: reduce) {
    .acount.live .acount-v { animation: none; }
  }
</style>

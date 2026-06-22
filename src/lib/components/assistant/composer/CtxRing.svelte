<script lang="ts">
  // Context-window gauge — a small filling ring (Claude-Desktop style) showing
  // how much of the model's context window the session has consumed. Accurate:
  // pct = (input + cacheRead + cacheCreate) / window from the CLI usage envelope
  // (see assistant.ctxPctFor). Calm by default; warms ≥75%, danger ≥90%.
  import { tooltip } from "$lib/actions/tooltip";
  import { fmtTokens } from "$lib/state/assistant/helpers";

  let { pct, tokens, window: win }: { pct: number; tokens: number; window: number } = $props();

  const R = 6;
  const C = 2 * Math.PI * R;
  const clamped = $derived(Math.max(0, Math.min(100, pct)));
  const offset = $derived(C * (1 - clamped / 100));
  const tone = $derived(clamped >= 90 ? "danger" : clamped >= 75 ? "warn" : "ok");
  const tip = $derived(
    `Context — ${fmtTokens(tokens)} / ${fmtTokens(win)} (${Math.round(clamped)}%)`,
  );
</script>

<span class="ctxring" data-tone={tone} use:tooltip={tip} aria-label={tip}>
  <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
    <circle class="track" cx="8" cy="8" r={R} fill="none" stroke-width="2" />
    <circle
      class="fill"
      cx="8"
      cy="8"
      r={R}
      fill="none"
      stroke-width="2"
      stroke-linecap="round"
      stroke-dasharray={C}
      stroke-dashoffset={offset}
      transform="rotate(-90 8 8)"
    />
  </svg>
</span>

<style>
  .ctxring {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: default;
  }
  .ctxring .track { stroke: color-mix(in oklab, var(--fg) 12%, transparent); }
  .ctxring .fill {
    stroke: var(--fg-faint);
    transition: stroke-dashoffset var(--dur-slow) var(--ease-soft), stroke var(--dur-base);
  }
  .ctxring[data-tone="warn"] .fill { stroke: var(--warn); }
  .ctxring[data-tone="danger"] .fill { stroke: var(--danger); }
</style>

<script lang="ts">
  // Per-step looping explainer visual. Pure markup — all styling + keyframes
  // live in $lib/styles/onboarding.css (loaded globally by OnboardingFlow).
  import { Sparkles, Palette } from "lucide-svelte";

  type Kind = "welcome" | "personalize" | "claude";
  let { kind, caption }: { kind: Kind; caption?: string } = $props();
</script>

<div class="ob-stage" aria-hidden="true">
  {#if kind === "welcome"}
    <span class="ob-ring r1"></span>
    <span class="ob-ring r2"></span>
    <span class="ob-ring r3"></span>
    <div class="ob-w-mark">
      <svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="currentColor"
        stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M8 3 5 12l4 2-2 7" />
        <path d="M17 3l-2 7 4 2-3 9" opacity="0.55" />
      </svg>
    </div>
  {:else if kind === "personalize"}
    <span class="ob-ring r1"></span>
    <span class="ob-ring r2"></span>
    <span class="ob-ring r3"></span>
    <div class="ob-w-mark"><Palette size={24} /></div>
  {:else if kind === "claude"}
    <div class="ob-cl">
      <div class="ob-cl-core">
        <Sparkles size={24} />
        <span class="ob-tw t1" style="top:-4px;left:8px"></span>
        <span class="ob-tw t2" style="bottom:2px;right:-2px"></span>
        <span class="ob-tw t3" style="top:10px;right:10px"></span>
      </div>
      <div class="ob-typing"><i></i><i></i><i></i></div>
    </div>
  {/if}
  {#if caption}<span class="ob-stage-cap">{caption}</span>{/if}
</div>

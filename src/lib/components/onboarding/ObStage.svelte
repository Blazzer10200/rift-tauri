<script lang="ts">
  // Per-step looping explainer visual. Pure markup — all styling + keyframes
  // live in $lib/styles/onboarding.css (loaded globally by OnboardingFlow).
  import { Sparkles, Key, Lock, Monitor, Server } from "lucide-svelte";

  type Kind = "welcome" | "ssh" | "server" | "claude" | "sync";
  let { kind, caption }: { kind: Kind; caption?: string } = $props();
</script>

<div class="ob-stage" aria-hidden="true">
  {#if kind === "welcome"}
    <span class="ob-ring r1"></span>
    <span class="ob-ring r2"></span>
    <span class="ob-ring r3"></span>
    <div class="ob-w-mark"><Sparkles size={26} /></div>
  {:else if kind === "ssh"}
    <div class="ob-ssh">
      <div class="ob-ssh-row">
        <div class="ob-ssh-ic"><Key size={20} /></div>
        <div class="ob-ssh-wire"><span class="ob-ssh-dot"></span></div>
        <div class="ob-ssh-ic lock"><Lock size={20} /></div>
      </div>
      <div class="ob-hexrow">3a f1 9c e2 7b 04 d8</div>
    </div>
  {:else if kind === "server"}
    <div class="ob-srv">
      <div class="ob-srv-node local"><Monitor size={22} /><span class="ob-srv-lab">local</span></div>
      <div class="ob-srv-wire">
        <span class="ob-srv-pkt up"></span>
        <span class="ob-srv-pkt down"></span>
      </div>
      <div class="ob-srv-node remote"><Server size={22} /><span class="ob-srv-lab">remote</span></div>
    </div>
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
  {:else if kind === "sync"}
    <div class="ob-scan">
      <span class="ob-scan-line"></span>
      <div class="ob-scan-row"><i style="width:82%"></i></div>
      <div class="ob-scan-row"><i style="width:64%"></i></div>
      <div class="ob-scan-row"><i style="width:91%"></i></div>
      <div class="ob-scan-row"><i style="width:48%"></i></div>
      <div class="ob-scan-row"><i style="width:73%"></i></div>
    </div>
  {/if}
  {#if caption}<span class="ob-stage-cap">{caption}</span>{/if}
</div>

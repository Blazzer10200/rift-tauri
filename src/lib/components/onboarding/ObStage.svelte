<script lang="ts">
  // Per-step looping product miniature — a tiny real moment of the app, not an
  // abstract icon. Pure markup — all styling + keyframes live in
  // $lib/styles/onboarding.css (loaded globally by OnboardingFlow).
  import {
    FolderOpen, FileCode2, FileJson2, BookText, KeyRound, ShieldCheck, Sparkles,
  } from "lucide-svelte";

  type Kind = "welcome" | "claude" | "openai" | "project" | "defaults";
  let { kind, caption }: { kind: Kind; caption?: string } = $props();
</script>

<div class="ob-stage" aria-hidden="true">
  {#if kind === "welcome"}
    <!-- miniature chat turn: ask → tools run → reply streams -->
    <div class="ob-mini ob-mini-chat">
      <div class="ob-mc-user">Fix the login redirect bug</div>
      <div class="ob-mc-chip c1"><i></i>Read&nbsp;<b>auth.ts</b></div>
      <div class="ob-mc-chip c2"><i></i>Edit&nbsp;<b>auth.ts</b></div>
      <div class="ob-mc-reply">
        <span class="l l1"></span>
        <span class="l l2"></span>
        <span class="l l3"></span>
      </div>
    </div>
  {:else if kind === "claude"}
    <!-- the CLI heartbeat: type `claude`, land connected -->
    <div class="ob-mini ob-mini-cli">
      <div class="ob-mcl-term">
        <span class="ps">$</span>
        <span class="cmd">claude</span>
        <span class="caret"></span>
      </div>
      <div class="ob-mcl-pill"><i></i>Connected — your auth, your billing</div>
    </div>
  {:else if kind === "openai"}
    <!-- secure API route: keychain → GPT model → Responses API ready -->
    <div class="ob-mini ob-mini-api">
      <div class="ob-ma-route">
        <span><KeyRound size={13} /> Keychain</span>
        <i></i>
        <span><Sparkles size={13} /> GPT 5.6</span>
      </div>
      <div class="ob-ma-pill"><i></i>Stateless Responses API</div>
    </div>
  {:else if kind === "project"}
    <!-- workspace scope: a real folder, scanned -->
    <div class="ob-mini ob-mini-tree">
      <div class="row r0"><FolderOpen size={13} /><span>my-app</span></div>
      <div class="row r1"><FileCode2 size={13} /><span>src/routes/+page.svelte</span></div>
      <div class="row r2"><FileJson2 size={13} /><span>package.json</span></div>
      <div class="row r3"><BookText size={13} /><span>README.md</span></div>
      <span class="scan"></span>
    </div>
  {:else}
    <!-- composer controls in miniature: model · effort dial · working style -->
    <div class="ob-mini ob-mini-def">
      <span class="ob-md-pill"><Sparkles size={12} /> Opus 5</span>
      <span class="ob-md-dial"><i class="d1"></i><i class="d2"></i><i class="d3"></i><i class="d4"></i></span>
      <span class="ob-md-pill perm"><ShieldCheck size={12} /> Balanced</span>
    </div>
  {/if}
  {#if caption}<span class="ob-stage-cap">{caption}</span>{/if}
</div>

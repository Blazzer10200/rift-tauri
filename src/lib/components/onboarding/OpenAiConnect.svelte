<script lang="ts">
  import { onMount } from "svelte";
  import { AlertCircle, CheckCircle2, Loader2, RefreshCw, Terminal } from "lucide-svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import "$lib/styles/onboarding.css";

  type Props = { onConnectedChange?: (connected: boolean) => void; standalone?: boolean };
  const { onConnectedChange, standalone = false }: Props = $props();

  let checking = $state(false);
  let error = $state<string | null>(null);
  const connected = $derived(assistant.codexStatus?.ready === true || assistant.openAiStatus?.ready === true);

  $effect(() => onConnectedChange?.(connected));
  onMount(() => void refresh());

  async function refresh() {
    if (checking) return;
    checking = true;
    error = null;
    try {
      await Promise.all([assistant.refreshCodexStatus(), assistant.refreshOpenAiStatus()]);
    } catch (e) {
      error = String(e);
    } finally {
      checking = false;
    }
  }

</script>

<header class="ob-head">
  {#if !standalone}<span class="ob-eyebrow">Step 2 · Connect ChatGPT</span>{/if}
  <h1 class="ob-title">Connect ChatGPT</h1>
  <p class="ob-sub">Sign in with your ChatGPT account through the official Codex CLI. Rift uses its local App Server and never reads or copies your credentials.</p>
</header>

<div class="ob-statcard">
  <div class="ob-statrow">
    <span class="k"><Terminal size={15} /> ChatGPT account</span>
    {#if connected}
      <span class="v ok"><CheckCircle2 size={14} /> connected</span>
    {:else}
      <span class="v warn"><AlertCircle size={14} /> sign-in needed</span>
    {/if}
  </div>
  {#if assistant.codexStatus?.summary}
    <div class="ob-statrow ob-statrow--summary"><span class="v">{assistant.codexStatus.summary}</span></div>
  {/if}
</div>

{#if connected}
  <div class="ob-connbar"><span class="dot"></span> ChatGPT is connected and your available GPT models are ready.</div>
{:else}
  {#if assistant.codexStatus?.cliPresent}
    <div class="ob-input-row">
      <button class="ob-btn primary" type="button" onclick={() => void assistant.startCodexLogin()} disabled={assistant.codexChecking}>
        <Terminal size={14} /> Sign in with ChatGPT
      </button>
    </div>
  {:else}
    <p class="ob-hint ob-hint--warn"><AlertCircle size={14} /><span>Install the standalone Codex CLI first: <code>npm install -g @openai/codex</code></span></p>
  {/if}
  <p class="ob-hint"><span>An API key is optional and separately billed. Add one later under Settings → Providers only if you need that route.</span></p>
{/if}

{#if error}<span class="ob-error"><AlertCircle size={14} /> {error}</span>{/if}

<div class="ob-input-row">
  <button class="ob-btn sm" type="button" onclick={() => void refresh()} disabled={checking}>
    {#if checking}<Loader2 size={13} class="spin" />{:else}<RefreshCw size={13} />{/if} Re-check
  </button>
</div>

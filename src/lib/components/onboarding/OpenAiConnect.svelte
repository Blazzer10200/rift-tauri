<script lang="ts">
  import { onMount } from "svelte";
  import { AlertCircle, CheckCircle2, KeyRound, Loader2, RefreshCw } from "lucide-svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import "$lib/styles/onboarding.css";

  type Props = { onConnectedChange?: (connected: boolean) => void; standalone?: boolean };
  const { onConnectedChange, standalone = false }: Props = $props();

  let apiKeyDraft = $state("");
  let saving = $state(false);
  let checking = $state(false);
  let error = $state<string | null>(null);
  const connected = $derived(assistant.openAiStatus?.ready === true);

  $effect(() => onConnectedChange?.(connected));
  onMount(() => void refresh());

  async function refresh() {
    if (checking) return;
    checking = true;
    error = null;
    try {
      await assistant.refreshOpenAiStatus();
    } catch (e) {
      error = String(e);
    } finally {
      checking = false;
    }
  }

  async function save() {
    const value = apiKeyDraft.trim();
    if (!value || saving) return;
    saving = true;
    error = null;
    try {
      await assistant.setOpenAiApiKey(value);
      apiKeyDraft = "";
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<header class="ob-head">
  {#if !standalone}<span class="ob-eyebrow">Step 2 · Connect OpenAI</span>{/if}
  <h1 class="ob-title">Connect OpenAI</h1>
  <p class="ob-sub">Use your own OpenAI API key for GPT models. Rift stores it in your operating-system credential vault and sends provider requests from the native app, never the web interface.</p>
</header>

<div class="ob-statcard">
  <div class="ob-statrow">
    <span class="k"><KeyRound size={15} /> OpenAI API</span>
    {#if connected}
      <span class="v ok"><CheckCircle2 size={14} /> connected</span>
    {:else}
      <span class="v warn"><AlertCircle size={14} /> key required</span>
    {/if}
  </div>
  {#if assistant.openAiStatus?.summary}
    <div class="ob-statrow ob-statrow--summary"><span class="v">{assistant.openAiStatus.summary}</span></div>
  {/if}
</div>

{#if connected}
  <div class="ob-connbar"><span class="dot"></span> OpenAI is connected and GPT models are ready.</div>
{:else}
  <div class="ob-input-row">
    <input
      class="ob-input"
      type="password"
      autocomplete="off"
      placeholder="sk-…"
      aria-label="OpenAI API key"
      bind:value={apiKeyDraft}
      onkeydown={(e) => { if (e.key === "Enter") void save(); }}
    />
    <button class="ob-btn primary" type="button" onclick={() => void save()} disabled={saving || apiKeyDraft.trim().length === 0}>
      {#if saving}<Loader2 size={14} class="spin" />{:else}<KeyRound size={14} />{/if} Save key
    </button>
  </div>
  <p class="ob-hint"><span>OpenAI API usage is billed separately from a ChatGPT subscription. You can add or remove this key later in Settings → Providers.</span></p>
{/if}

{#if error}<span class="ob-error"><AlertCircle size={14} /> {error}</span>{/if}

<div class="ob-input-row">
  <button class="ob-btn sm" type="button" onclick={() => void refresh()} disabled={checking}>
    {#if checking}<Loader2 size={13} class="spin" />{:else}<RefreshCw size={13} />{/if} Re-check
  </button>
</div>

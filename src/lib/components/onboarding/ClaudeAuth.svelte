<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Terminal, UserCheck, BadgeCheck, CheckCircle2, AlertCircle, Loader2, RefreshCw } from "lucide-svelte";

  type AuthStatus = {
    cliPresent: boolean;
    cliVersion: string | null;
    loggedIn: boolean;
    authMethod: string | null;
    apiProvider: string | null;
    email: string | null;
    subscriptionType: string | null;
    apiKeyConfigured: boolean;
    pill: "green" | "yellow" | "red";
    summary: string;
  };

  let probing = $state(false);
  let status = $state<AuthStatus | null>(null);
  let error = $state<string | null>(null);

  async function probe() {
    probing = true;
    error = null;
    try {
      status = await invoke<AuthStatus>("assistant_auth_probe");
    } catch (e) {
      error = String(e);
    } finally {
      probing = false;
    }
  }

  onMount(probe);
</script>

<header class="ob-head">
  <span class="ob-eyebrow">Step 3 · Connect Claude</span>
  <h1 class="ob-title">Connect Claude</h1>
  <p class="ob-sub">Rift runs the <code>claude</code> CLI under the hood. If it's already installed and logged in, you're set — otherwise you can finish this anytime in Settings → Assistant.</p>
</header>

{#if probing}
  <div class="ob-statcard">
    <div class="ob-statrow"><span class="k"><Loader2 size={15} class="spin" /> Checking CLI…</span></div>
  </div>
{:else if status}
  <div class="ob-statcard">
    <div class="ob-statrow">
      <span class="k"><Terminal size={15} /> claude CLI</span>
      {#if status.cliPresent}
        <span class="v ok"><CheckCircle2 size={14} /> {status.cliVersion ?? "installed"}</span>
      {:else}
        <span class="v warn"><AlertCircle size={14} /> not on PATH</span>
      {/if}
    </div>
    <div class="ob-statrow">
      <span class="k"><UserCheck size={15} /> Logged in</span>
      {#if status.loggedIn}
        <span class="v ok"><CheckCircle2 size={14} /> {status.email ?? status.authMethod ?? "yes"}</span>
      {:else}
        <span class="v warn"><AlertCircle size={14} /> not authenticated</span>
      {/if}
    </div>
    {#if status.subscriptionType}
      <div class="ob-statrow">
        <span class="k"><BadgeCheck size={15} /> Plan</span>
        <span class="v">{status.subscriptionType}</span>
      </div>
    {/if}
  </div>

  {#if !status.cliPresent}
    <p class="ob-hint"><span>Install the CLI from <code class="ob-mono-i">docs.claude.com/en/docs/claude-code</code>, then re-check. Or skip — you can do this anytime.</span></p>
  {:else if !status.loggedIn}
    <p class="ob-hint"><span>Run <code class="ob-mono-i">claude</code> once in a terminal to log in, then re-check.</span></p>
  {/if}
{/if}

{#if error}
  <span class="ob-error"><AlertCircle size={14} /> {error}</span>
{/if}

<div class="ob-input-row">
  <button class="ob-btn sm" type="button" onclick={probe} disabled={probing}>
    <RefreshCw size={13} /> Re-check
  </button>
</div>

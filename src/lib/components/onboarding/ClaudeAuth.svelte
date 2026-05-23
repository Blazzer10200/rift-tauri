<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Sparkles, ChevronRight, ChevronLeft, CheckCircle2, AlertCircle, Loader2, RefreshCw } from "lucide-svelte";

  type Props = {
    onNext: () => void;
    onBack?: () => void;
    onSkip?: () => void;
  };
  const { onNext, onBack, onSkip }: Props = $props();

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

<div class="onboarding-step">
  <header class="onboarding-head">
    <div class="onboarding-icon"><Sparkles size={28} /></div>
    <h2 class="onboarding-title">Connect Claude (optional)</h2>
    <p class="onboarding-sub">
      Rift's embedded assistant uses your existing <code>claude</code> CLI auth.
      You can skip this and configure it later from Settings → Assistant.
    </p>
  </header>

  <div class="status-block">
    {#if probing}
      <p class="status-line muted"><Loader2 size={14} class="spin" /> Checking CLI…</p>
    {:else if status}
      <div class="status-grid">
        <div class="status-row">
          <span class="label">claude CLI installed</span>
          {#if status.cliPresent}
            <span class="ok"><CheckCircle2 size={14} /> {status.cliVersion ?? "yes"}</span>
          {:else}
            <span class="warn"><AlertCircle size={14} /> not found on PATH</span>
          {/if}
        </div>
        <div class="status-row">
          <span class="label">Logged in</span>
          {#if status.loggedIn}
            <span class="ok"><CheckCircle2 size={14} /> {status.email ?? status.authMethod ?? "yes"}</span>
          {:else}
            <span class="warn"><AlertCircle size={14} /> not authenticated</span>
          {/if}
        </div>
        {#if status.subscriptionType}
          <div class="status-row">
            <span class="label">Plan</span>
            <span class="value">{status.subscriptionType}</span>
          </div>
        {/if}
      </div>
      {#if !status.cliPresent}
        <p class="install-hint">
          Install the CLI: see <code>https://docs.claude.com/en/docs/claude-code</code>
          then return here and click "Re-check". Or skip this step — Rift's
          sync features work without the assistant.
        </p>
      {:else if !status.loggedIn}
        <p class="install-hint">
          Run <code>claude</code> in your terminal once to log in interactively.
          Then click "Re-check".
        </p>
      {/if}
    {/if}
    {#if error}
      <p class="error-line"><AlertCircle size={14} /> {error}</p>
    {/if}
  </div>

  <button class="btn sm align-self" type="button" onclick={probe} disabled={probing}>
    <RefreshCw size={12} /> Re-check
  </button>

  <footer class="onboarding-foot">
    <div class="left-cluster">
      <button class="btn ghost sm" type="button" onclick={() => onBack?.()}>
        <ChevronLeft size={14} /> Back
      </button>
      <button class="btn ghost sm" type="button" onclick={() => onSkip?.()}>
        Skip
      </button>
    </div>
    <button class="btn primary" type="button" onclick={onNext}>
      Next <ChevronRight size={14} />
    </button>
  </footer>
</div>

<style>
  .onboarding-step {
    display: flex; flex-direction: column; gap: 16px;
    max-width: 560px; margin: 56px auto; padding: 28px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }
  .onboarding-head { display: flex; flex-direction: column; gap: 6px; }
  .onboarding-icon {
    width: 48px; height: 48px;
    display: flex; align-items: center; justify-content: center;
    border-radius: var(--radius-md);
    background: color-mix(in oklch, var(--accent) 14%, transparent);
    color: var(--accent);
  }
  .onboarding-title { font-size: 18px; font-weight: 600; color: var(--fg); margin: 0; }
  .onboarding-sub { font-size: 13px; color: var(--fg-muted); line-height: 1.5; margin: 0; }
  .status-block { display: flex; flex-direction: column; gap: 10px; }
  .status-line { display: flex; align-items: center; gap: 6px; font-size: 13px; margin: 0; }
  .status-line.muted { color: var(--fg-muted); }
  .status-grid {
    display: flex; flex-direction: column; gap: 6px;
    padding: 12px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .status-row {
    display: flex; justify-content: space-between; align-items: center;
    font-size: 12.5px;
  }
  .status-row .label { color: var(--fg-muted); }
  .status-row .value { color: var(--fg); }
  .ok { display: flex; align-items: center; gap: 4px; color: var(--success, oklch(0.78 0.16 145)); }
  .warn { display: flex; align-items: center; gap: 4px; color: var(--warn, oklch(0.82 0.15 80)); }
  .install-hint { font-size: 12px; color: var(--fg-muted); line-height: 1.5; margin: 0; }
  .install-hint code { font-family: var(--font-mono); font-size: 11.5px; }
  .error-line { display: flex; align-items: center; gap: 6px; color: var(--danger); font-size: 12.5px; margin: 0; }
  code { font-family: var(--font-mono); font-size: 12px; padding: 1px 5px; background: var(--bg-elev-2); border-radius: var(--radius-xs); }
  .align-self { align-self: flex-start; }
  .onboarding-foot {
    display: flex; justify-content: space-between; align-items: center;
    margin-top: 8px; padding-top: 16px;
    border-top: 1px solid var(--border);
  }
  .left-cluster { display: flex; gap: 6px; align-items: center; }
</style>

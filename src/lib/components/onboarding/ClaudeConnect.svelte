<script lang="ts">
  // Guided Claude connect — active setup step, not a passive status card.
  // Auto-polls the auth probe so finishing an install/login in a terminal
  // flips the card green without a manual re-check. Three paths to green:
  // install CLI (copy command), sign in (in-app OAuth via startLogin), or
  // paste an API key.
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { assistant } from "$lib/state/assistant.svelte";
  // Self-import so the ob-* classes resolve wherever this mounts — including the
  // Assistant welcome's needs-auth screen, where the onboarding flow may never
  // have mounted to load the stylesheet.
  import "$lib/styles/onboarding.css";
  import {
    Terminal, UserCheck, CheckCircle2, AlertCircle, Loader2,
    RefreshCw, Copy, Check, KeyRound, LogIn,
  } from "lucide-svelte";

  const CLI_DOCS_URL = "https://docs.claude.com/en/docs/claude-code";
  const INSTALL_CMD = "irm https://claude.ai/install.ps1 | iex";
  const INSTALL_CMD_NPM = "npm install -g @anthropic-ai/claude-code";

  type AuthStatus = {
    cliPresent: boolean;
    cliVersion: string | null;
    loggedIn: boolean;
    authMethod: string | null;
    apiProvider: string | null;
    email: string | null;
    subscriptionType: string | null;
    apiKeyConfigured: boolean;
    envApiKeyPresent: boolean;
    pill: "green" | "yellow" | "red";
    summary: string;
  };

  let probing = $state(false);
  let status = $state<AuthStatus | null>(null);
  let error = $state<string | null>(null);
  let copied = $state(false);
  let copiedNpm = $state(false);
  let showApiKey = $state(false);
  let apiKeyDraft = $state("");
  let savingKey = $state(false);

  // standalone = mounted outside the onboarding rail (e.g. the welcome needs-auth
  // screen): drop the "Step 2" eyebrow + onboarding-specific copy.
  type Props = { onConnectedChange?: (connected: boolean) => void; standalone?: boolean };
  const { onConnectedChange, standalone = false }: Props = $props();

  const connected = $derived(
    !!status && status.cliPresent && (status.loggedIn || status.apiKeyConfigured),
  );
  $effect(() => { onConnectedChange?.(connected); });

  async function probe() {
    if (probing) return;
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

  // Poll until connected — picks up installs/logins finished outside Rift.
  let poll: ReturnType<typeof setInterval> | null = null;
  onMount(() => {
    void probe();
    poll = setInterval(() => {
      if (!connected && !assistant.loginInProgress && !savingKey) void probe();
    }, 4000);
  });
  onDestroy(() => {
    if (poll) clearInterval(poll);
    if (copyTimer) clearTimeout(copyTimer);
    if (copyTimerNpm) clearTimeout(copyTimerNpm);
  });

  let copyTimer: ReturnType<typeof setTimeout> | null = null;
  let copyTimerNpm: ReturnType<typeof setTimeout> | null = null;
  async function copyCmd() {
    try {
      await navigator.clipboard.writeText(INSTALL_CMD);
      copied = true;
      if (copyTimer) clearTimeout(copyTimer);
      copyTimer = setTimeout(() => (copied = false), 1600);
    } catch (e) {
      error = `Copy failed: ${String(e)}`;
    }
  }
  async function copyCmdNpm() {
    try {
      await navigator.clipboard.writeText(INSTALL_CMD_NPM);
      copiedNpm = true;
      if (copyTimerNpm) clearTimeout(copyTimerNpm);
      copyTimerNpm = setTimeout(() => (copiedNpm = false), 1600);
    } catch (e) {
      error = `Copy failed: ${String(e)}`;
    }
  }

  async function signIn(useConsole = false) {
    await assistant.startLogin(useConsole);
    await probe();
  }

  async function saveKey() {
    const v = apiKeyDraft.trim();
    if (!v || savingKey) return;
    savingKey = true;
    error = null;
    try {
      await assistant.setApiKey(v);
      apiKeyDraft = "";
      showApiKey = false;
      await probe();
    } catch (e) {
      error = String(e);
    } finally {
      savingKey = false;
    }
  }
</script>

<header class="ob-head">
  {#if !standalone}<span class="ob-eyebrow">Step 2 · Connect Claude</span>{/if}
  <h1 class="ob-title">Connect Claude</h1>
  <p class="ob-sub">Rift is powered by the <code>claude</code> command-line tool and your own Claude account, so you stay in control of your sign-in and billing. You can finish this step now, or later in Settings → Claude.</p>
</header>

{#if !status && probing}
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
        <span class="v warn"><AlertCircle size={14} /> not found</span>
      {/if}
    </div>
    <div class="ob-statrow">
      <span class="k"><UserCheck size={15} /> Signed in</span>
      {#if status.loggedIn}
        <span class="v ok"><CheckCircle2 size={14} /> {status.email ?? status.authMethod ?? "yes"}</span>
      {:else if status.apiKeyConfigured}
        <span class="v ok"><KeyRound size={14} /> API key</span>
      {:else}
        <span class="v warn"><AlertCircle size={14} /> not yet</span>
      {/if}
    </div>
    {#if status.summary}
      <div class="ob-statrow ob-statrow--summary">
        <span class="v">{status.summary}</span>
      </div>
    {/if}
  </div>

  {#if !status.cliPresent}
    <div class="ob-field">
      <span class="ob-flabel">Install the Claude CLI — run this command in PowerShell</span>
      <div class="ob-copy-block">
        <span class="ob-copy-cmd">{INSTALL_CMD}</span>
        <button type="button" class="ob-copy-btn" class:ok={copied} onclick={copyCmd}>
          {#if copied}<Check size={13} /> Copied{:else}<Copy size={13} /> Copy{/if}
        </button>
      </div>
      <span class="ob-flabel ob-flabel--alt">If PowerShell blocks scripts, install with npm instead</span>
      <div class="ob-copy-block">
        <span class="ob-copy-cmd">{INSTALL_CMD_NPM}</span>
        <button type="button" class="ob-copy-btn" class:ok={copiedNpm} onclick={copyCmdNpm}>
          {#if copiedNpm}<Check size={13} /> Copied{:else}<Copy size={13} /> Copy{/if}
        </button>
      </div>
      <p class="ob-hint"><span>Rift detects the installation automatically. After installing, click <strong>Re-check</strong>. If the CLI still isn't found, restart Rift — Windows sometimes needs a restart before newly installed programs become visible. More options at <button type="button" class="ob-link" onclick={() => void openUrl(CLI_DOCS_URL).catch((e) => console.warn("openUrl failed", e))}>docs.claude.com/en/docs/claude-code</button>.</span></p>
    </div>
  {:else if !connected}
    <div class="ob-input-row">
      <button class="ob-btn primary" type="button" onclick={() => void signIn(false)} disabled={assistant.loginInProgress}>
        {#if assistant.loginInProgress}<Loader2 size={15} class="spin" /> Waiting for sign-in…{:else}<LogIn size={15} /> Sign in with Claude{/if}
      </button>
    </div>
    {#if assistant.loginInProgress}
      <p class="ob-hint"><span>A console window has opened with your sign-in link — it may be behind this window, so check your taskbar. Complete the sign-in there and Rift will detect it automatically.</span></p>
    {:else}
      <p class="ob-hint"><span>This signs you in with a Claude subscription (Pro or Max). If you use the Anthropic API instead, <button type="button" class="ob-link" onclick={() => void signIn(true)}>sign in with Console</button> or <button type="button" class="ob-link" onclick={() => (showApiKey = !showApiKey)}>paste an API key</button>.</span></p>
    {/if}
    {#if showApiKey}
      <div class="ob-input-row">
        <input
          class="ob-input"
          type="password"
          placeholder="sk-ant-…"
          bind:value={apiKeyDraft}
          onkeydown={(e) => { if (e.key === "Enter") void saveKey(); }}
        />
        <button class="ob-btn" type="button" onclick={() => void saveKey()} disabled={savingKey || apiKeyDraft.trim().length === 0}>
          {#if savingKey}<Loader2 size={14} class="spin" />{:else}<KeyRound size={14} />{/if} Save key
        </button>
      </div>
      <p class="ob-hint"><span>API keys bill per use through your Anthropic Console account. Everything in Rift works the same — your personal Claude CLI settings simply aren't applied when a key is used.</span></p>
    {/if}
  {:else}
    <div class="ob-connbar"><span class="dot"></span> Claude is connected and ready to work.</div>
  {/if}
{/if}

{#if error}
  <span class="ob-error"><AlertCircle size={14} /> {error}</span>
{/if}

{#if status && !connected}
  <div class="ob-input-row">
    <button class="ob-btn sm" type="button" onclick={() => void probe()} disabled={probing}>
      <RefreshCw size={13} /> Re-check
    </button>
  </div>
{/if}

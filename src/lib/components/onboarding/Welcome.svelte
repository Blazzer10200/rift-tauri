<!--
  First-run welcome screen.

  NOT YET WIRED INTO AppShell. The wiring contract for a future session:
    1. Add a first-run detection getter in src/lib/state/connection.svelte.ts
       at the bottom of the ConnectionStore class — combines `this.servers.length === 0`,
       `assistant.auth?.loggedIn === false`, and `defaultSshKeyExists === false` into
       one boolean. Call it `isFirstRun`.
    2. In src/lib/components/AppShell.svelte, before mounting the activity bar
       (search for `<aside class="rail"` — around line 80-120) gate the main
       content area on `!connection.isFirstRun` and render <OnboardingFlow /> when
       isFirstRun is true. OnboardingFlow is a thin router that walks Welcome →
       SSHKeySetup → ProfileSetup → ServerAdd → ClaudeAuth → FirstSync.
    3. Make the flow dismissible (top-right "Skip onboarding" button) so power
       users can configure manually.

  Until that wiring lands, this component is reachable only via direct import
  from a route or dev tool. It still renders + functions correctly in isolation.
-->
<script lang="ts">
  import { Sparkles, ChevronRight } from "lucide-svelte";

  type Props = {
    onNext: () => void;
    onSkip?: () => void;
  };
  const { onNext, onSkip }: Props = $props();
</script>

<div class="onboarding-step">
  <header class="onboarding-head">
    <div class="onboarding-icon">
      <Sparkles size={32} />
    </div>
    <h1 class="onboarding-title">Welcome to Rift</h1>
    <p class="onboarding-sub">
      A FiveM / RedM dev workspace launcher with SFTP, auto-sync, drift
      reconciliation, and an embedded Claude assistant. Let's walk through
      the five-minute setup.
    </p>
  </header>

  <ul class="onboarding-bullets">
    <li>
      <strong>Generate or import an SSH key</strong> — used to connect to your
      game server's SFTP endpoint without typing a password.
    </li>
    <li>
      <strong>Configure your local workspace</strong> — pick a folder on disk
      where Rift will mirror remote resources.
    </li>
    <li>
      <strong>Add your first server</strong> — host, port, user, key, remote
      root.
    </li>
    <li>
      <strong>Connect Claude (optional)</strong> — the embedded assistant uses
      your existing <code>claude</code> CLI auth. Skip this step if you don't
      have the CLI installed.
    </li>
    <li>
      <strong>Run your first sync</strong> — confirm the path before any files
      move.
    </li>
  </ul>

  <footer class="onboarding-foot">
    <button class="btn ghost sm" type="button" onclick={() => onSkip?.()}>
      Skip — I'll configure manually
    </button>
    <button class="btn primary" type="button" onclick={onNext}>
      Get started
      <ChevronRight size={14} />
    </button>
  </footer>
</div>

<style>
  .onboarding-step {
    display: flex; flex-direction: column;
    gap: 20px;
    max-width: 560px;
    margin: 56px auto;
    padding: 28px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }
  .onboarding-head { display: flex; flex-direction: column; gap: 8px; }
  .onboarding-icon {
    width: 56px; height: 56px;
    display: flex; align-items: center; justify-content: center;
    border-radius: var(--radius-md);
    background: color-mix(in oklch, var(--accent) 14%, transparent);
    color: var(--accent);
    margin-bottom: 4px;
  }
  .onboarding-title { font-size: 22px; font-weight: 600; color: var(--fg); margin: 0; }
  .onboarding-sub { font-size: 13.5px; color: var(--fg-muted); line-height: 1.5; margin: 0; }
  .onboarding-bullets {
    list-style: none; padding: 0; margin: 0;
    display: flex; flex-direction: column; gap: 10px;
  }
  .onboarding-bullets li {
    font-size: 13px; color: var(--fg-2); line-height: 1.5;
    padding-left: 16px; position: relative;
  }
  .onboarding-bullets li::before {
    content: "→";
    position: absolute; left: 0; top: 0;
    color: var(--accent);
  }
  .onboarding-bullets code {
    font-family: var(--font-mono); font-size: 12px;
    padding: 1px 5px;
    background: var(--bg-elev-2);
    border-radius: var(--radius-xs);
    color: var(--fg);
  }
  .onboarding-foot {
    display: flex; justify-content: space-between; align-items: center;
    margin-top: 8px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }
</style>

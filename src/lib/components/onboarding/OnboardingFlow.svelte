<script lang="ts">
  // #7 first-run flow router. Walks the six step components in order, threading
  // the two pieces of data that flow forward (local workspace root → ServerAdd,
  // saved server key → FirstSync). Mounted by AppShell when the first-run gate
  // is open; `onDone` fires on completion OR skip (AppShell persists dismissal).
  import Welcome from "./Welcome.svelte";
  import SSHKeySetup from "./SSHKeySetup.svelte";
  import ProfileSetup from "./ProfileSetup.svelte";
  import ServerAdd from "./ServerAdd.svelte";
  import ClaudeAuth from "./ClaudeAuth.svelte";
  import FirstSync from "./FirstSync.svelte";
  import { connection } from "../../state/connection.svelte";
  import { X } from "lucide-svelte";

  type Props = { onDone: () => void };
  const { onDone }: Props = $props();

  type Step = "welcome" | "ssh" | "profile" | "server" | "auth" | "sync";
  const ORDER: Step[] = ["welcome", "ssh", "profile", "server", "auth", "sync"];
  let step = $state<Step>("welcome");
  let localRoot = $state<string>("");
  let serverKey = $state<string | null>(null);

  const stepIdx = $derived(ORDER.indexOf(step));

  function go(to: Step) {
    step = to;
  }
  function back() {
    if (stepIdx > 0) step = ORDER[stepIdx - 1];
  }

  async function finish() {
    // Reload servers so the first-run gate flips off, then select the one the
    // user just created so they land connected rather than on an empty Sync page.
    try {
      await connection.loadServers();
    } catch (e) {
      console.error("onboarding: loadServers failed", e);
    }
    if (serverKey) {
      try {
        await connection.select(serverKey);
      } catch (e) {
        console.error("onboarding: select failed", e);
      }
    }
    onDone();
  }
</script>

<div class="ob-shell">
  <button class="ob-skip" type="button" onclick={onDone} aria-label="Skip onboarding">
    <X size={14} /> Skip
  </button>

  <div class="ob-progress" role="progressbar" aria-valuemin={1} aria-valuemax={ORDER.length} aria-valuenow={stepIdx + 1}>
    {#each ORDER as _s, i (i)}
      <span class="ob-dot" data-active={stepIdx >= i}></span>
    {/each}
  </div>

  {#if step === "welcome"}
    <Welcome onNext={() => go("ssh")} onSkip={onDone} />
  {:else if step === "ssh"}
    <SSHKeySetup onNext={() => go("profile")} onBack={back} />
  {:else if step === "profile"}
    <ProfileSetup onNext={(root) => { localRoot = root; go("server"); }} onBack={back} />
  {:else if step === "server"}
    <ServerAdd {localRoot} onNext={(key) => { serverKey = key; go("auth"); }} onBack={back} />
  {:else if step === "auth"}
    <ClaudeAuth onNext={() => go("sync")} onBack={back} onSkip={() => go("sync")} />
  {:else if step === "sync"}
    <FirstSync {serverKey} onDone={finish} onBack={back} />
  {/if}
</div>

<style>
  .ob-shell {
    position: relative;
    height: 100%;
    overflow-y: auto;
    background: var(--bg);
  }
  .ob-skip {
    position: absolute;
    top: 14px;
    right: 16px;
    z-index: 2;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 10px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg-elev-1);
    color: var(--fg-2);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }
  .ob-skip:hover {
    background: var(--surface-hover);
    color: var(--fg);
  }
  .ob-progress {
    display: flex;
    gap: 6px;
    justify-content: center;
    padding-top: 20px;
  }
  .ob-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--border);
    transition: background 160ms ease;
  }
  .ob-dot[data-active="true"] {
    background: var(--accent);
  }
</style>

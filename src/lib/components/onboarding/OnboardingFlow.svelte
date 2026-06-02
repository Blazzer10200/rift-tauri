<script lang="ts">
  // #7 first-run flow router. Graphite-Ink redesign: full-pane takeover with a
  // persistent left-rail vertical stepper + a right pane (active step body) +
  // pinned footer (progress dots · Back · primary advance). Five steps — the old
  // ProfileSetup (local-root picker) is folded into ServerAdd. Mounted by
  // AppShell when the first-run gate is open; onDone fires on completion OR skip.
  import "$lib/styles/onboarding.css";
  import Welcome from "./Welcome.svelte";
  import SSHKeySetup from "./SSHKeySetup.svelte";
  import ServerAdd from "./ServerAdd.svelte";
  import ClaudeAuth from "./ClaudeAuth.svelte";
  import FirstSync from "./FirstSync.svelte";
  import { connection } from "../../state/connection.svelte";
  import { Layers, ChevronLeft, ChevronRight, Check, Clock } from "lucide-svelte";

  type Props = { onDone: () => void };
  const { onDone }: Props = $props();

  type Step = "welcome" | "ssh" | "server" | "claude" | "sync";
  type StepDef = { id: Step; t: string; s: string; primary: string };
  const STEPS: StepDef[] = [
    { id: "welcome", t: "Welcome", s: "What Rift does", primary: "Get started" },
    { id: "ssh", t: "SSH key", s: "Auth without passwords", primary: "Next" },
    { id: "server", t: "Connect server", s: "Host, key & paths", primary: "Next" },
    { id: "claude", t: "Claude", s: "Optional assistant", primary: "Next" },
    { id: "sync", t: "First sync", s: "Preview & finish", primary: "Finish setup" },
  ];

  let step = $state<Step>("welcome");
  let serverKey = $state<string | null>(null);
  let canAdvance = $state(true);

  const stepIdx = $derived(STEPS.findIndex((s) => s.id === step));
  const current = $derived(STEPS[stepIdx]);
  const isLast = $derived(stepIdx === STEPS.length - 1);

  function defaultAdvance(s: Step): boolean {
    // SSH + Server gate the primary until their async work completes.
    return !(s === "ssh" || s === "server");
  }

  function go(to: Step) {
    canAdvance = defaultAdvance(to);
    step = to;
  }
  function back() {
    if (stepIdx > 0) go(STEPS[stepIdx - 1].id);
  }
  function railJump(target: Step, idx: number) {
    if (idx < stepIdx) go(target); // only jump back to a completed step
  }

  async function next() {
    if (!canAdvance) return;
    if (isLast) { await finish(); return; }
    go(STEPS[stepIdx + 1].id);
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

<div class="ob-overlay">
  <!-- ── left rail ──────────────────────────────────────────────────── -->
  <aside class="ob-rail">
    <div class="ob-brand">
      <span class="ob-brand-mark"><Layers size={17} /></span>
      <span class="ob-brand-name">Rift</span>
      <span class="ob-brand-tag">Setup</span>
    </div>

    <div class="ob-steps">
      {#each STEPS as s, i (s.id)}
        <button
          type="button"
          class="ob-step"
          class:active={i === stepIdx}
          class:done={i < stepIdx}
          disabled={i >= stepIdx}
          onclick={() => railJump(s.id, i)}
        >
          <span class="ob-node">
            {#if i < stepIdx}<Check size={13} />{:else}{i + 1}{/if}
          </span>
          <span class="ob-step-body">
            <span class="ob-step-t">{s.t}</span>
            <span class="ob-step-s">{s.s}</span>
          </span>
        </button>
      {/each}
    </div>

    <div class="ob-rail-foot">
      <span class="ob-time"><Clock size={12} /> About 3 minutes</span>
      <button type="button" class="ob-skip-link" onclick={onDone}>Skip setup — configure manually</button>
    </div>
  </aside>

  <!-- ── right pane ─────────────────────────────────────────────────── -->
  <div class="ob-main ob-enter">
    <div class="ob-body">
      {#key step}
        <div class="ob-step-wrap">
          {#if step === "welcome"}
            <Welcome />
          {:else if step === "ssh"}
            <SSHKeySetup onReady={(b) => (canAdvance = b)} />
          {:else if step === "server"}
            <ServerAdd onSaved={(key) => { serverKey = key; canAdvance = true; }} />
          {:else if step === "claude"}
            <ClaudeAuth />
          {:else if step === "sync"}
            <FirstSync />
          {/if}
        </div>
      {/key}
    </div>

    <footer class="ob-foot">
      <div class="ob-foot-left">
        <div class="ob-foot-dots">
          {#each STEPS as _s, i (i)}
            <span class="ob-fdot" class:on={i < stepIdx} class:cur={i === stepIdx}></span>
          {/each}
        </div>
      </div>
      <div class="ob-foot-right">
        <button type="button" class="ob-btn ghost" onclick={back} disabled={stepIdx === 0}>
          <ChevronLeft size={15} /> Back
        </button>
        <button type="button" class="ob-btn primary" onclick={next} disabled={!canAdvance}>
          {current.primary}
          {#if !isLast}<ChevronRight size={15} />{/if}
        </button>
      </div>
    </footer>
  </div>
</div>

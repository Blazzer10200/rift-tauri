<script lang="ts">
  // First-run flow. Pure-assistant build: a 3-step welcome — what Rift is,
  // personalize (accent), connect Claude (optional). The old SSH/server/sync
  // walkthrough was removed when Rift dropped its SFTP/sync half. Mounted by
  // AppShell when the first-run gate is open; onDone fires on finish OR skip
  // and persists the dismissal.
  import "$lib/styles/onboarding.css";
  import ObStage from "./ObStage.svelte";
  import ClaudeAuth from "./ClaudeAuth.svelte";
  import RiftLogo from "$lib/components/shell/RiftLogo.svelte";
  import { uiPrefs, ACCENTS } from "$lib/state/ui-prefs.svelte";
  import {
    Check, ChevronLeft, ChevronRight, FolderGit2, Terminal, Zap,
    Sparkles, TriangleAlert,
  } from "lucide-svelte";

  type Props = { onDone: () => void };
  const { onDone }: Props = $props();

  const steps = [
    { t: "Welcome", s: "What Rift is" },
    { t: "Personalize", s: "Make it yours" },
    { t: "Connect Claude", s: "Optional" },
    { t: "Before you start", s: "A quick heads-up" },
  ];

  let step = $state(1);
  const last = steps.length;

  function goto(n: number) {
    if (n < step) step = n; // rail nodes only navigate backward (completed)
  }
  function next() {
    if (step < last) step += 1;
    else onDone();
  }
  function back() {
    if (step > 1) step -= 1;
  }

  function onEscape(e: KeyboardEvent) {
    if (e.key === "Escape") { e.preventDefault(); onDone(); }
  }
</script>

<svelte:window onkeydown={onEscape} />

<div class="ob-overlay" role="dialog" aria-modal="true" aria-label="Rift first-run setup">
  <!-- ── Left rail: brand + vertical stepper ── -->
  <aside class="ob-rail">
    <div class="ob-brand">
      <span class="ob-brand-mark"><RiftLogo size={30} /></span>
      <span class="ob-brand-name">Rift</span>
      <span class="ob-brand-tag">Setup</span>
    </div>

    <div class="ob-steps">
      {#each steps as s, i (s.t)}
        {@const n = i + 1}
        <button
          type="button"
          class="ob-step"
          class:active={step === n}
          class:done={step > n}
          onclick={() => goto(n)}
        >
          <span class="ob-node">{#if step > n}<Check size={13} strokeWidth={3} />{:else}{n}{/if}</span>
          <span class="ob-step-body">
            <span class="ob-step-t">{s.t}</span>
            <span class="ob-step-s">{s.s}</span>
          </span>
        </button>
      {/each}
    </div>

    <div class="ob-rail-foot">
      <button type="button" class="ob-skip-link" onclick={onDone}>Skip setup</button>
    </div>
  </aside>

  <!-- ── Right pane: active step ── -->
  <main class="ob-main">
    <div class="ob-body">
      {#key step}
        <div class="ob-step-wrap ob-enter">
          {#if step === 1}
            <ObStage kind="welcome" caption="local assistant" />
            <header class="ob-head">
              <span class="ob-eyebrow">Step 1 · Welcome</span>
              <h1 class="ob-title">Meet Rift</h1>
              <p class="ob-sub">A local coding assistant powered by the <code>claude</code> CLI. It runs entirely on your machine — no remote connections, no sync.</p>
            </header>
            <div class="ob-vlist">
              <div class="ob-vrow">
                <span class="ob-vic"><FolderGit2 size={18} /></span>
                <span class="ob-vbody">
                  <span class="ob-vt">Workspace-scoped</span>
                  <span class="ob-vp">Reads, searches, and edits your local codebase. Nothing leaves your machine.</span>
                </span>
              </div>
              <div class="ob-vrow">
                <span class="ob-vic"><Terminal size={18} /></span>
                <span class="ob-vbody">
                  <span class="ob-vt">Powered by the claude CLI</span>
                  <span class="ob-vp">Spawns <code>claude</code> as a subprocess — your auth, your billing, your data.</span>
                </span>
              </div>
              <div class="ob-vrow">
                <span class="ob-vic"><Zap size={18} /></span>
                <span class="ob-vbody">
                  <span class="ob-vt">MCP tools built in</span>
                  <span class="ob-vp"><code>read_file</code>, <code>list_dir</code>, <code>grep</code>, and local-git — exposed over stdio, no config.</span>
                </span>
              </div>
            </div>
          {:else if step === 2}
            <ObStage kind="personalize" caption="personalize" />
            <header class="ob-head">
              <span class="ob-eyebrow">Step 2 · Personalize</span>
              <h1 class="ob-title">Make it yours</h1>
              <p class="ob-sub">Pick an accent color — it retunes the whole interface live. Everything else lives in Settings.</p>
            </header>
            <div class="ob-accent">
              <span class="ob-accent-label">Accent color</span>
              <div class="ob-accent-row">
                {#each ACCENTS as a (a.id)}
                  <button
                    type="button"
                    class="ob-accent-dot"
                    class:on={uiPrefs.accentHue === a.hue}
                    style="--sw: oklch(0.78 0.15 {a.hue})"
                    aria-label={a.label}
                    aria-pressed={uiPrefs.accentHue === a.hue}
                    onclick={() => uiPrefs.setAccentHue(a.hue)}
                  >
                    {#if uiPrefs.accentHue === a.hue}<Check size={14} strokeWidth={3} />{/if}
                  </button>
                {/each}
              </div>
            </div>
          {:else if step === 3}
            <ObStage kind="claude" caption="embedded assistant" />
            <ClaudeAuth />
          {:else}
            <ObStage kind="beta" caption="beta program" />
            <header class="ob-head">
              <span class="ob-eyebrow">Step 4 · Before you start</span>
              <h1 class="ob-title">You're testing the beta</h1>
              <p class="ob-sub">Thanks for trying Rift early. Two things to keep in mind while you use it.</p>
            </header>
            <div class="ob-vlist">
              <div class="ob-vrow">
                <span class="ob-vic"><Sparkles size={18} /></span>
                <span class="ob-vbody">
                  <span class="ob-vt">This is pre-release software</span>
                  <span class="ob-vp">Rift is still being fine-tuned — expect rough edges, occasional bugs, and features that may change or break between updates.</span>
                </span>
              </div>
              <div class="ob-vrow">
                <span class="ob-vic warn"><TriangleAlert size={18} /></span>
                <span class="ob-vbody">
                  <span class="ob-vt">AI can make mistakes</span>
                  <span class="ob-vp">Responses and code edits are AI-generated and may be wrong, incomplete, or unsafe. Review every change before relying on it — and keep your work backed up.</span>
                </span>
              </div>
            </div>
          {/if}
        </div>
      {/key}
    </div>

    <!-- ── Footer: progress dots + nav ── -->
    <footer class="ob-foot">
      <div class="ob-foot-left">
        <div class="ob-foot-dots">
          {#each steps as s, i (s.t)}
            {@const n = i + 1}
            <span class="ob-fdot" class:on={step >= n} class:cur={step === n}></span>
          {/each}
        </div>
      </div>
      <div class="ob-foot-right">
        {#if step > 1}
          <button type="button" class="ob-btn ghost" onclick={back}>
            <ChevronLeft size={15} /> Back
          </button>
        {/if}
        <button type="button" class="ob-btn primary" onclick={next}>
          {#if step === last}
            <Check size={15} /> Start working
          {:else}
            Next <ChevronRight size={15} />
          {/if}
        </button>
      </div>
    </footer>
  </main>
</div>

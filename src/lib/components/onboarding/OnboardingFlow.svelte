<script lang="ts">
  // First-run flow. Pure-assistant build: 4 actionable steps — welcome (w/ beta
  // notice + accent picker folded in), guided Claude connect, open a project,
  // and defaults (model / effort / git tools). Mounted by AppShell when the
  // first-run gate is open; onDone fires on finish OR skip and persists the
  // dismissal + beta acknowledgment.
  import "$lib/styles/onboarding.css";
  import ObStage from "./ObStage.svelte";
  import ClaudeConnect from "./ClaudeConnect.svelte";
  import RiftLogo from "$lib/components/shell/RiftLogo.svelte";
  import { uiPrefs, ACCENTS } from "$lib/state/ui-prefs.svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import { fableAvailable, haikuAvailable } from "$lib/state/assistant/helpers";
  import { MODE_OPTIONS } from "$lib/components/assistant/composer/modelMatrix";
  import type { ModelSel, ThinkingEffort } from "$lib/state/assistant/types";
  import {
    Check, ChevronLeft, ChevronRight, FolderGit2, FolderOpen,
    History, Loader2, TriangleAlert,
  } from "lucide-svelte";

  type Props = { onDone: () => void };
  const { onDone }: Props = $props();

  const steps = [
    { t: "Welcome", s: "What Rift is" },
    { t: "Connect Claude", s: "CLI & sign-in" },
    { t: "Open a project", s: "Pick a folder" },
    { t: "Defaults", s: "Model & tools" },
  ];

  let step = $state(1);
  const last = steps.length;

  // Soft-block: if the user tries to leave the Connect step (2) without Claude
  // connected, warn ONCE — a model pick is inert without auth — then let them
  // proceed on the next press (we don't hard-block; some users connect later).
  let connectConnected = $state(false);
  let warnSkipConnect = $state(false);

  function goto(n: number) {
    if (n < step) {
      if (n <= 2) warnSkipConnect = false;
      step = n; // rail nodes only navigate backward (completed)
    }
  }
  function next() {
    if (step === 2 && !connectConnected && !warnSkipConnect) {
      warnSkipConnect = true;
      return;
    }
    if (step < last) step += 1;
    else onDone();
  }
  function back() {
    warnSkipConnect = false;
    if (step > 1) step -= 1;
  }

  function onEscape(e: KeyboardEvent) {
    if (e.key !== "Escape") return;
    e.preventDefault();
    // Esc on step 1 dismisses the flow; on later steps it walks back a step so a
    // reflexive Escape (cancel a field edit, dismiss a tooltip) can't blow away
    // the whole setup. Skip is still one click via the rail-footer link.
    if (step === 1) onDone();
    else back();
  }

  // ── Open-project step ──
  let picking = $state(false);
  async function chooseFolder() {
    if (picking) return;
    picking = true;
    try { await assistant.pickFolder(); } finally { picking = false; }
  }
  const recentRoots = $derived(
    assistant.workspace.recent.filter((r) => r !== assistant.workspace.current).slice(0, 3),
  );
  // strip the Windows long-path prefix (\\?\C:\…) the backend stores
  const displayPath = (p: string) => p.replace(/^\\\\\?\\/, "");

  // ── Defaults step — same matrix shape as Composer's picker, current models only ──
  type ModelOpt = { id: ModelSel; label: string; version: string; effort: boolean; maxEffort: ThinkingEffort };
  const MODEL_OPTIONS: ModelOpt[] = [
    ...(fableAvailable() ? [{ id: "claude-fable-5" as ModelSel, label: "Fable", version: "5", effort: true, maxEffort: "ultra" as ThinkingEffort }] : []),
    { id: "opus",   label: "Opus",   version: "4.8", effort: true,  maxEffort: "ultra" },
    { id: "sonnet", label: "Sonnet", version: "5",   effort: true,  maxEffort: "ultra" },
    ...(haikuAvailable() ? [{ id: "haiku" as ModelSel, label: "Haiku", version: "4.5", effort: false, maxEffort: "none" as ThinkingEffort }] : []),
  ];
  function pickModel(m: ModelOpt) {
    assistant.setModel(m.id);
    // setModel already clamps the stored effort to the new model's ceiling.
  }

  // ── Presets — collapse permission + thinking + git-trust into ONE first-run
  // choice. A first-runner shouldn't have to reason about three orthogonal
  // controls; power users retune any of them later from the composer/Settings.
  // Each preset fans out to the same setters the granular controls used, so the
  // stored state is identical to having picked them by hand.
  type Preset = {
    id: "cautious" | "balanced" | "fast";
    label: string;
    blurb: string;
    perm: (typeof MODE_OPTIONS)[number]["id"];
    thinking: { on: boolean; effort: ThinkingEffort };
    trust: "readonly" | "standard";
  };
  const PRESETS: Preset[] = [
    { id: "cautious", label: "Cautious", blurb: "Asks before every edit. Read-only git. Safest while you learn what Rift does.", perm: "default",           thinking: { on: true,  effort: "smart" }, trust: "readonly" },
    { id: "balanced", label: "Balanced", blurb: "Edits files automatically, asks for anything riskier. Git commit/pull/push on. Recommended.", perm: "acceptEdits",       thinking: { on: true,  effort: "smart" }, trust: "standard" },
    { id: "fast",     label: "Fast",     blurb: "Runs everything without prompts and replies instantly (no thinking step). Most autonomous.", perm: "bypassPermissions", thinking: { on: false, effort: "smart" }, trust: "standard" },
  ];
  // Reflect the live state back to a preset chip so the selection survives a
  // back-nav (or a granular change made elsewhere) without a separate $state.
  const activePreset = $derived.by(() => {
    const p = PRESETS.find(
      (x) =>
        x.perm === assistant.permissionMode &&
        x.thinking.on === assistant.thinkingEnabled &&
        (x.trust === "readonly") === (assistant.trustLevel === "readonly"),
    );
    return p?.id ?? null;
  });
  function pickPreset(p: Preset) {
    assistant.setPermissionMode(p.perm);
    assistant.setThinkingDial(p.thinking.on, p.thinking.effort);
    void assistant.setTrustLevel(p.trust);
  }
</script>

<svelte:window onkeydown={onEscape} />

<div class="ob-overlay" role="dialog" aria-modal="true" aria-label="Rift first-run setup">
 <div class="ob-card">
  <!-- ── Left rail: brand + vertical stepper ── -->
  <aside class="ob-rail">
    <div class="ob-brand">
      <span class="ob-brand-mark"><RiftLogo size={30} /></span>
      <span class="ob-brand-name">Rift</span>
      <span class="ob-brand-tag">Setup</span>
    </div>

    <div class="ob-steps">
      <span class="ob-spine"><i style="height:{((step - 1) / (last - 1)) * 100}%"></i></span>
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
              <p class="ob-sub">A local coding assistant powered by the <code>claude</code> CLI — it reads, searches, and edits your codebase entirely on your machine. No remote connections, no sync; your auth, your billing.</p>
            </header>
            <div class="ob-vlist">
              <div class="ob-vrow">
                <span class="ob-vic"><FolderGit2 size={18} /></span>
                <span class="ob-vbody">
                  <span class="ob-vt">Local &amp; private</span>
                  <span class="ob-vp">Works in the folder you point it at — nothing leaves your machine. MCP tools (read, grep, local-git) built in, no config.</span>
                </span>
              </div>
              <div class="ob-vrow">
                <span class="ob-vic warn"><TriangleAlert size={18} /></span>
                <span class="ob-vbody">
                  <span class="ob-vt">You're testing the beta</span>
                  <span class="ob-vp">Pre-release, and replies are AI-generated — review changes before relying on them and keep your work backed up.</span>
                </span>
              </div>
            </div>
            <div class="ob-accent ob-accent-inline">
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
                    {#if uiPrefs.accentHue === a.hue}<Check size={12} strokeWidth={3} />{/if}
                  </button>
                {/each}
              </div>
            </div>
          {:else if step === 2}
            <ObStage kind="claude" caption="embedded assistant" />
            <ClaudeConnect onConnectedChange={(c) => { connectConnected = c; if (c) warnSkipConnect = false; }} />
            {#if warnSkipConnect}
              <p class="ob-hint ob-hint--warn"><TriangleAlert size={14} /><span>Claude isn't connected yet — your model and tool picks won't work until it is. Press Next again to continue anyway, or connect above first.</span></p>
            {/if}
          {:else if step === 3}
            <ObStage kind="project" caption="your workspace" />
            <header class="ob-head">
              <span class="ob-eyebrow">Step 3 · Open a project</span>
              <h1 class="ob-title">Open a project</h1>
              <p class="ob-sub">Point Rift at a code folder so the assistant has something to work on. You can switch projects anytime from the title bar.</p>
            </header>
            {#if assistant.workspace.current}
              <div class="ob-statcard">
                <div class="ob-statrow">
                  <span class="k"><FolderGit2 size={15} /> Workspace</span>
                  <span class="v ok"><Check size={14} /> {displayPath(assistant.workspace.current)}</span>
                </div>
              </div>
              <div class="ob-input-row">
                <button class="ob-btn sm" type="button" onclick={() => void chooseFolder()} disabled={picking}>
                  {#if picking}<Loader2 size={13} class="spin" />{:else}<FolderOpen size={13} />{/if} Change folder
                </button>
              </div>
            {:else}
              <div class="ob-input-row">
                <button class="ob-btn primary" type="button" onclick={() => void chooseFolder()} disabled={picking}>
                  {#if picking}<Loader2 size={15} class="spin" />{:else}<FolderOpen size={15} />{/if} Choose a folder
                </button>
              </div>
              {#if recentRoots.length > 0}
                <div class="ob-field">
                  <span class="ob-flabel">Recent</span>
                  <div class="ob-recent">
                    {#each recentRoots as r (r)}
                      <button type="button" class="ob-recent-btn" onclick={() => void assistant.setRoot(r)}>
                        <History size={14} />
                        <span class="ob-recent-path">{displayPath(r)}</span>
                      </button>
                    {/each}
                  </div>
                </div>
              {/if}
              <p class="ob-hint"><span>Or skip — Rift works in a private scratch space until you pick a folder, and you can open one anytime from the Workspace page.</span></p>
            {/if}
          {:else}
            <ObStage kind="defaults" caption="tuned to you" />
            <header class="ob-head">
              <span class="ob-eyebrow">Step 4 · Defaults</span>
              <h1 class="ob-title">Pick your defaults</h1>
              <p class="ob-sub">How the assistant runs by default. Each project remembers its own model and effort — change these anytime from the composer.</p>
            </header>
            <div class="ob-fields">
              <div class="ob-field">
                <span class="ob-flabel">Model</span>
                <div class="ob-seg" role="radiogroup" aria-label="Default model">
                  {#each MODEL_OPTIONS as m (m.id)}
                    <button
                      type="button"
                      class="ob-seg-btn"
                      class:on={assistant.model === m.id}
                      role="radio"
                      aria-checked={assistant.model === m.id}
                      onclick={() => pickModel(m)}
                    >
                      {m.label} <span class="ob-seg-ver">{m.version}</span>
                    </button>
                  {/each}
                </div>
              </div>
              <div class="ob-field">
                <span class="ob-flabel">How should Rift work?</span>
                <div class="ob-presets" role="radiogroup" aria-label="Working style preset">
                  {#each PRESETS as p (p.id)}
                    <button
                      type="button"
                      class="ob-preset"
                      class:on={activePreset === p.id}
                      role="radio"
                      aria-checked={activePreset === p.id}
                      onclick={() => pickPreset(p)}
                    >
                      <span class="ob-preset-head">
                        <span class="ob-preset-name">{p.label}</span>
                        {#if p.id === "balanced"}<span class="ob-preset-rec">Recommended</span>{/if}
                        {#if activePreset === p.id}<span class="ob-preset-check"><Check size={13} strokeWidth={3} /></span>{/if}
                      </span>
                      <span class="ob-preset-blurb">{p.blurb}</span>
                    </button>
                  {/each}
                </div>
                <span class="ob-field-hint">Fine-tune permissions, thinking, and git access anytime from the composer and Settings.</span>
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
</div>

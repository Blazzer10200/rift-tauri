<script lang="ts">
  // First-run flow: 4 actionable steps — welcome (w/ beta notice + accent
  // picker folded in), guided provider connect, open a project,
  // and defaults (model / effort / git tools). Mounted by AppShell when the
  // first-run gate is open; onDone fires on finish OR skip and persists the
  // dismissal + beta acknowledgment.
  import "$lib/styles/onboarding.css";
  import ObStage from "./ObStage.svelte";
  import ClaudeConnect from "./ClaudeConnect.svelte";
  import OpenAiConnect from "./OpenAiConnect.svelte";
  import RiftLogo from "$lib/components/shell/RiftLogo.svelte";
  import { uiPrefs, ACCENTS } from "$lib/state/ui-prefs.svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import { MODE_OPTIONS, currentModels, isFreeClaudeModel, type ModelOpt } from "$lib/components/assistant/composer/modelMatrix";
  import type { ThinkingEffort } from "$lib/state/assistant/types";
  import { isOpenAIModel } from "$lib/state/assistant/helpers";
  import { CHATGPT } from "$lib/state/assistant/providerDisplay";
  import {
    Check, ChevronLeft, ChevronRight, FolderGit2, FolderOpen,
    History, Loader2, Orbit, Terminal, TriangleAlert,
  } from "lucide-svelte";

  type Props = { onDone: () => void };
  const { onDone }: Props = $props();

  const steps = [
    { t: "Welcome", s: "What Rift is" },
    { t: "Connect AI", s: "Claude or ChatGPT" },
    { t: "Open a project", s: "Choose a folder" },
    { t: "Defaults", s: "Model & working style" },
  ];

  let step = $state(1);
  const last = steps.length;

  // Soft-block: if the user tries to leave the Connect step (2) without the
  // selected provider connected, warn ONCE, then let them
  // proceed on the next press (we don't hard-block; some users connect later).
  let connectConnected = $state(false);
  let warnSkipConnect = $state(false);
  let provider = $state<"claude" | "openai">(isOpenAIModel(assistant.model) ? "openai" : "claude");

  function selectProvider(next: "claude" | "openai") {
    provider = next;
    connectConnected = next === "openai"
      ? assistant.codexStatus?.ready === true
      : assistant.auth?.pill === "green" || assistant.auth?.pill === "yellow";
    if (next === "openai" && !isOpenAIModel(assistant.model)) {
      assistant.setModel(assistant.codexModels?.find((model) => model.isDefault)?.id ?? "gpt-5.6-sol");
    }
    if (next === "claude" && isOpenAIModel(assistant.model)) assistant.setModel("sonnet");
    warnSkipConnect = false;
  }

  const defaultModels = $derived(currentModels.filter((model) =>
    model.provider === (provider === "openai" ? "openai" : "claude")
      && (model.provider !== "claude" || assistant.plan !== "free" || isFreeClaudeModel(model.id))
  ));
  const selectedModelLabel = $derived.by(() => {
    const live = assistant.codexModels?.find((model) => model.id === assistant.model);
    if (live) return live.label;
    const known = currentModels.find((model) => model.id === assistant.model);
    return known ? `${known.label} ${known.version}` : assistant.model;
  });

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
    else {
      seedFirstPrompt();
      onDone();
    }
  }

  // Finish (not Skip) lands with a ghost first-prompt in the composer — the
  // existing promptSuggestion affordance (#87): Tab to accept, dismissible,
  // never blocks typing. Carries setup momentum straight into a first turn.
  function seedFirstPrompt() {
    const tab = assistant.activeTab;
    if (!tab || (tab.draft ?? "").trim() || tab.promptSuggestion) return;
    tab.promptSuggestion = assistant.workspace.current
      ? "Give me a tour of this project — what it does, how it's organized, and where to start."
      : "What can you do? Show me how to work with you.";
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

  // ── Defaults step — Composer's own current-model rows, so labels can't drift ──
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
    { id: "cautious", label: "Cautious", blurb: "Rift asks for your approval before every change, and version control stays read-only. The safest way to learn what Rift can do.", perm: "default",           thinking: { on: true,  effort: "smart" }, trust: "readonly" },
    { id: "balanced", label: "Balanced", blurb: "Rift edits files on its own and asks before anything riskier. Git commits, pulls, and pushes are enabled. The best fit for most people.", perm: "acceptEdits",       thinking: { on: true,  effort: "smart" }, trust: "standard" },
    { id: "fast",     label: "Fast",     blurb: "Rift works without asking for approval and replies immediately, skipping the extended-thinking step. The most autonomous option.", perm: "bypassPermissions", thinking: { on: false, effort: "smart" }, trust: "standard" },
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
    // setTrustLevel rethrows after its own danger toast; swallow the rejection
    // so a failed backend invoke doesn't surface as an unhandled promise (the
    // user already saw the toast, and activePreset reflects the partial apply).
    assistant.setTrustLevel(p.trust).catch(() => {});
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
              <p class="ob-sub">Rift is a local coding workspace for Claude and ChatGPT models. It reads, searches, and edits only inside the folder you choose; prompts and relevant context go directly to the AI service you connect.</p>
            </header>
            <div class="ob-vlist">
              <div class="ob-vrow">
                <span class="ob-vic"><FolderGit2 size={18} /></span>
                <span class="ob-vbody">
                  <span class="ob-vt">Private by design</span>
                  <span class="ob-vp">Rift only works inside the project folder you choose. Reading, searching, and version control are built in — no setup required.</span>
                </span>
              </div>
              <div class="ob-vrow">
                <span class="ob-vic warn"><TriangleAlert size={18} /></span>
                <span class="ob-vbody">
                  <span class="ob-vt">This is a beta release</span>
                  <span class="ob-vp">Rift is still in active development, and its replies are AI-generated. Review changes before you rely on them, and keep backups of important work.</span>
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
            <ObStage kind={provider} caption={provider === "claude" ? "local CLI connection" : "ChatGPT account connection"} />
            <div class="ob-seg ob-provider-seg" role="radiogroup" aria-label="AI provider">
              <button type="button" class="ob-seg-btn ob-provider-btn" class:on={provider === "openai"} role="radio" aria-checked={provider === "openai"} onclick={() => selectProvider("openai")}>
                <Orbit size={15} /><span><b>ChatGPT</b><small>account via Codex App Server</small></span>
              </button>
              <button type="button" class="ob-seg-btn ob-provider-btn" class:on={provider === "claude"} role="radio" aria-checked={provider === "claude"} onclick={() => selectProvider("claude")}>
                <Terminal size={15} /><span><b>Claude</b><small>CLI or API key</small></span>
              </button>
            </div>
            {#if provider === "claude"}
              <ClaudeConnect onConnectedChange={(c) => { connectConnected = c; if (c) warnSkipConnect = false; }} />
            {:else}
              <OpenAiConnect onConnectedChange={(c) => { connectConnected = c; if (c) warnSkipConnect = false; }} />
            {/if}
            {#if warnSkipConnect}
              <p class="ob-hint ob-hint--warn"><TriangleAlert size={14} /><span>{provider === "claude" ? "Claude" : CHATGPT.label} isn't connected yet, so that service can't reply. Press Next again to continue anyway, or finish connecting first.</span></p>
            {/if}
          {:else if step === 3}
            <ObStage kind="project" caption="your workspace" />
            <header class="ob-head">
              <span class="ob-eyebrow">Step 3 · Open a project</span>
              <h1 class="ob-title">Open a project</h1>
              <p class="ob-sub">Choose the folder that holds your code. Rift reads and works only inside that folder, and you can switch projects at any time from the title bar.</p>
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
              <p class="ob-hint"><span>You can also skip this for now. Rift will use a private scratch space until you choose a folder — open one at any time from the Workspace page.</span></p>
            {/if}
          {:else}
            <ObStage kind="defaults" caption="tuned to you" modelLabel={selectedModelLabel} />
            <header class="ob-head">
              <span class="ob-eyebrow">Step 4 · Defaults</span>
              <h1 class="ob-title">Pick your defaults</h1>
              <p class="ob-sub">Choose how Rift works by default. Every project remembers its own settings, and you can change any of this later from the chat controls.</p>
            </header>
            <div class="ob-fields">
              <div class="ob-field">
                <span class="ob-flabel">Model</span>
                <div class="ob-seg" role="radiogroup" aria-label="Default model">
                  {#each defaultModels as m (m.id)}
                    {@const available = m.provider === "openai" ? assistant.chatGptModelAvailable(m.id) : assistant.authReadyForModel(m.id)}
                    <button
                      type="button"
                      class="ob-seg-btn"
                      class:on={assistant.model === m.id}
                      role="radio"
                      aria-checked={assistant.model === m.id}
                      disabled={!available}
                      onclick={() => pickModel(m)}
                    >
                      {m.label} <span class="ob-seg-ver">{available ? m.version : "unavailable"}</span>
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
                <span class="ob-field-hint">You can fine-tune permissions, thinking, and version-control access at any time in Settings.</span>
              </div>
            </div>
          {/if}
        </div>
      {/key}
    </div>

    <!-- ── Footer: progress dots + nav ── -->
    <footer class="ob-foot">
      <div class="ob-foot-left">
        <button type="button" class="ob-skip-link ob-skip-mobile" onclick={onDone}>Skip setup</button>
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

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
  import { fableAvailable } from "$lib/state/assistant/helpers";
  import { MODE_OPTIONS } from "$lib/components/assistant/composer/modelMatrix";
  import type { ModelSel, ThinkingEffort } from "$lib/state/assistant/types";
  import {
    Check, ChevronLeft, ChevronRight, FolderGit2, FolderOpen, Terminal, Zap,
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
    { id: "sonnet", label: "Sonnet", version: "4.6", effort: true,  maxEffort: "smart" },
    { id: "haiku",  label: "Haiku",  version: "4.5", effort: false, maxEffort: "none" },
  ];
  type EffortOpt = { id: ThinkingEffort; label: string; hint: string };
  const EFFORT_OPTIONS: EffortOpt[] = [
    { id: "none",  label: "Instant",   hint: "Minimal reasoning — fastest answers" },
    { id: "quick", label: "Quick",     hint: "Light reasoning, leaner tool use" },
    { id: "smart", label: "Smart",     hint: "Standard depth — the recommended default" },
    { id: "deep",  label: "Deep",      hint: "Extra depth for hard agentic coding" },
    { id: "ultra", label: "Ultracode", hint: "Deep reasoning + multi-agent workflows" },
  ];
  const currentModel = $derived(MODEL_OPTIONS.find((m) => m.id === assistant.model));
  const effortStops = $derived.by(() => {
    if (!currentModel?.effort) return [] as EffortOpt[];
    const cap = EFFORT_OPTIONS.findIndex((e) => e.id === currentModel.maxEffort);
    return EFFORT_OPTIONS.slice(0, cap >= 0 ? cap + 1 : EFFORT_OPTIONS.length);
  });
  function pickModel(m: ModelOpt) {
    assistant.setModel(m.id);
    // clamp effort to the new model's ceiling so the saved pref never exceeds it
    const idx = EFFORT_OPTIONS.findIndex((e) => e.id === assistant.thinkingEffort);
    const cap = EFFORT_OPTIONS.findIndex((e) => e.id === m.maxEffort);
    if (m.effort && cap >= 0 && idx > cap) assistant.setThinkingEffort(EFFORT_OPTIONS[cap].id);
  }
  // The three modes a first-run user can reason about — plan/auto stay in the
  // composer's full picker. Same source as the composer so labels never drift.
  const PERM_OPTIONS = MODE_OPTIONS.filter((m) =>
    m.id === "default" || m.id === "acceptEdits" || m.id === "bypassPermissions",
  );
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
              <div class="ob-vrow">
                <span class="ob-vic warn"><TriangleAlert size={18} /></span>
                <span class="ob-vbody">
                  <span class="ob-vt">You're testing the beta</span>
                  <span class="ob-vp">Pre-release software, and responses are AI-generated — they can be wrong or unsafe. Review changes before relying on them; keep your work backed up.</span>
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
            <ClaudeConnect />
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
              <p class="ob-hint"><span>Or skip — you can open a folder anytime from the title bar.</span></p>
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
              {#if effortStops.length > 0}
                <div class="ob-field">
                  <span class="ob-flabel">Thinking effort</span>
                  <div class="ob-seg" role="radiogroup" aria-label="Thinking effort">
                    {#each effortStops as e (e.id)}
                      <button
                        type="button"
                        class="ob-seg-btn"
                        class:on={assistant.thinkingEffort === e.id}
                        role="radio"
                        aria-checked={assistant.thinkingEffort === e.id}
                        title={e.hint}
                        onclick={() => assistant.setThinkingEffort(e.id)}
                      >{e.label}</button>
                    {/each}
                  </div>
                </div>
              {/if}
              <div class="ob-field">
                <span class="ob-flabel">Permissions</span>
                <div class="ob-seg" role="radiogroup" aria-label="Permission mode">
                  {#each PERM_OPTIONS as m (m.id)}
                    <button
                      type="button"
                      class="ob-seg-btn"
                      class:on={assistant.permissionMode === m.id}
                      role="radio"
                      aria-checked={assistant.permissionMode === m.id}
                      title={m.hint}
                      onclick={() => assistant.setPermissionMode(m.id)}
                    >{m.label}</button>
                  {/each}
                </div>
                <span class="ob-field-hint">Bypass runs tools and edits files without asking — pick "Ask before edits" to approve each change. Change anytime from the composer.</span>
              </div>
              <div class="ob-field">
                <span class="ob-flabel">Git tools</span>
                <div class="ob-seg" role="radiogroup" aria-label="Git tools trust level">
                  <button type="button" class="ob-seg-btn" class:on={assistant.trustLevel === "readonly"} role="radio" aria-checked={assistant.trustLevel === "readonly"} onclick={() => void assistant.setTrustLevel("readonly")}>Read-only</button>
                  <button type="button" class="ob-seg-btn" class:on={assistant.trustLevel !== "readonly"} role="radio" aria-checked={assistant.trustLevel !== "readonly"} onclick={() => void assistant.setTrustLevel("standard")}>Standard</button>
                </div>
                <span class="ob-field-hint">Read-only = status, diff, log. Standard adds commit, pull, and push.</span>
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

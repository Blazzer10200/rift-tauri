<script lang="ts">
  import "$lib/styles/settings-controls.css";
  import { onMount } from "svelte";
  import { FlaskConical, Loader2, Eye, EyeOff, RefreshCw, Check, Zap, Plus, Trash2, Power, Gauge, Sparkles, AlertTriangle } from "lucide-svelte";
  import PageHero from "../shared/PageHero.svelte";
  import { providers, PRESETS, draftIdFor, type PresetDef, type ProviderDto } from "../../state/providers.svelte";
  import { localLlm } from "../../state/localLlm.svelte";

  // Which profile the cockpit below edits. null = the Claude panel.
  let selectedId = $state<string | null>(null);
  const selected = $derived(providers.list.find((p) => p.id === selectedId) ?? null);

  // Draft fields — copied on select, persisted on blur via upsert.
  let dName = $state("");
  let dBase = $state("");
  let dModel = $state("");
  let dEffort = $state(false);
  let saveError = $state("");

  let keyInput = $state("");
  let keyVisible = $state(false);
  let keySaving = $state(false);

  let testing = $state(false);
  let testResult = $state<{ ok: boolean; msg: string } | null>(null);
  let testMs = $state<number | null>(null);
  let testTokens = $state<number | null>(null);
  let modelHint = $state("");
  let confirmDelete = $state(false);
  let switching = $state(false);

  let optimizeResult = $state<{ ok: boolean; msg: string } | null>(null);
  let optimizeTarget = $state(32768);

  const CTX_TARGETS = [
    { label: "16K", value: 16384 },
    { label: "32K", value: 32768 },
    { label: "64K", value: 65536 },
    { label: "128K", value: 131072 },
  ] as const;

  const canTest = $derived(!!selected && dBase.trim().length > 0 && dModel.trim().length > 0);

  // Ollama ctx guidance — meaningful only for the LIVE provider (the probe
  // reads the active wire fields backend-side).
  const ctx = $derived(localLlm.ctxInfo);
  const showCtx = $derived(!!selected?.active && !!ctx && ctx.is_ollama);
  const fmt = (n: number) => n.toLocaleString();
  const cardBits = $derived([ctx?.family, ctx?.params, ctx?.quant].filter((s): s is string => !!s));

  const tokPerSec = $derived(
    testTokens != null && testMs != null && testMs > 0 ? Math.round(testTokens / (testMs / 1000)) : null,
  );

  // Ctx probe is Ollama-only; probing a cloud or dead base just logs errors.
  function probeCtxIfLocal() {
    const base = providers.active?.base_url ?? "";
    if (!/\/\/(localhost|127\.0\.0\.1)(:|\/|$)/.test(base)) return;
    void localLlm.refresh().then(() => localLlm.refreshCtx());
  }

  onMount(() => {
    void providers.refresh().then(() => {
      select(providers.active ?? null);
      probeCtxIfLocal();
    });
  });

  function resetTransient() {
    testResult = null; testMs = null; testTokens = null;
    modelHint = ""; saveError = ""; confirmDelete = false;
    optimizeResult = null; keyInput = ""; keyVisible = false;
  }

  function select(p: ProviderDto | null) {
    selectedId = p?.id ?? null;
    resetTransient();
    if (p) { dName = p.name; dBase = p.base_url; dModel = p.model ?? ""; dEffort = p.effort; }
  }

  async function saveDraft() {
    if (!selected) return;
    saveError = "";
    testResult = null;
    try {
      await providers.upsert({
        id: selected.id,
        name: dName.trim() || selected.name,
        base_url: dBase.trim(),
        model: dModel.trim() || null,
        models: selected.models,
        preset: selected.preset,
        max_output_tokens: selected.max_output_tokens,
        effort: dEffort,
      });
      const fresh = providers.list.find((p) => p.id === selected.id);
      if (fresh) { dName = fresh.name; dBase = fresh.base_url; dModel = fresh.model ?? ""; dEffort = fresh.effort; }
    } catch (e) {
      console.error("save provider failed", e);
      saveError = String(e);
    }
  }

  async function addProvider(def: PresetDef | null) {
    const taken = new Set(providers.list.map((p) => p.id));
    const id = draftIdFor(def?.preset ?? null, taken);
    try {
      await providers.upsert({
        id,
        name: def?.name ?? "Custom endpoint",
        base_url: def?.base_url ?? "http://localhost:8000",
        model: def?.models[0] ?? null,
        models: def?.models ?? [],
        preset: def?.preset ?? null,
        max_output_tokens: def?.max_output_tokens ?? null,
        effort: def?.effort ?? false,
      });
      const p = providers.list.find((x) => x.id === id);
      if (p) select(p);
    } catch (e) {
      console.error("add provider failed", e);
      saveError = String(e);
    }
  }

  async function selectModel(m: string) {
    dModel = m;
    await saveDraft();
  }

  async function toggleEffort() {
    dEffort = !dEffort;
    await saveDraft();
  }

  async function detectModels() {
    if (!selected) return;
    modelHint = "";
    await saveDraft();
    const n = await providers.detectModels(selected.id);
    modelHint = n === 0
      ? "Endpoint lists no models — type one (normal for Kimi/DeepSeek/GLM)."
      : `${n} model${n === 1 ? "" : "s"} found.`;
  }

  async function saveKey() {
    if (!selected) return;
    keySaving = true;
    try { await providers.setKey(selected.id, keyInput); keyInput = ""; }
    catch (e) { saveError = String(e); }
    finally { keySaving = false; }
  }

  async function clearKey() {
    if (!selected) return;
    keySaving = true;
    try { await providers.setKey(selected.id, null); }
    catch (e) { saveError = String(e); }
    finally { keySaving = false; }
  }

  async function testConnection() {
    if (!selected) return;
    await saveDraft();
    testing = true;
    testResult = null; testMs = null; testTokens = null;
    const t0 = performance.now();
    const r = await providers.test(selected.id);
    testMs = Math.round(performance.now() - t0);
    testTokens = r.tokens ?? null;
    testResult = { ok: r.ok, msg: r.msg };
    testing = false;
  }

  async function activateSelected() {
    if (!selected) return;
    await saveDraft();
    switching = true;
    try {
      await providers.activate(selected.id);
      probeCtxIfLocal();
    } catch (e) { saveError = String(e); }
    finally { switching = false; }
  }

  async function backToClaude() {
    switching = true;
    try { await providers.activate(null); }
    catch (e) { saveError = String(e); }
    finally { switching = false; }
  }

  async function deleteSelected() {
    if (!selected) return;
    if (!confirmDelete) { confirmDelete = true; return; }
    try {
      await providers.remove(selected.id);
      select(null);
    } catch (e) { saveError = String(e); }
  }

  async function optimizeCtx() {
    if (!selected) return;
    optimizeResult = null;
    optimizeResult = await localLlm.optimize(optimizeTarget);
    // The optimize repoints the wire model directly — mirror it into the
    // profile so the next activate doesn't resurrect the un-optimized tag.
    if (optimizeResult.ok) {
      dModel = optimizeResult.msg;
      await saveDraft();
    }
  }
</script>

<div class="sb-main">
  <PageHero
    eyebrow="Experimental"
    title="Models"
    desc="Bring other frontier models into Rift — Kimi, DeepSeek, GLM, OpenRouter, or any Anthropic-compatible endpoint. Claude stays the default; switch back any time."
  >
    {#snippet chip()}
      <span class="sb-chip {providers.active ? 'verified' : ''}">
        <span class="dot"></span>{providers.active ? providers.active.name : "Claude"}
      </span>
    {/snippet}
  </PageHero>

  <div class="sb-scroll">
    <div class="sb-wrap">

      <!-- ── Chat brain rail — who answers in Chat ── -->
      <div class="st-block">
        <div class="st-block-label">Chat brain</div>
        <div class="prow">
          <button class="pcard" type="button" class:live={!providers.active} class:sel={selectedId === null} onclick={() => select(null)}>
            <span class="pcard-name">Claude</span>
            <span class="pcard-sub">Claude Code sign-in</span>
            {#if !providers.active}<span class="st-pill ok"><span class="dot"></span>live</span>{/if}
          </button>
          {#each providers.list as p (p.id)}
            <button class="pcard" type="button" class:live={p.active} class:sel={selectedId === p.id} onclick={() => select(p)}>
              <span class="pcard-name">{p.name}</span>
              <span class="pcard-sub mono">{p.model || p.base_url}</span>
              {#if p.active}<span class="st-pill ok"><span class="dot"></span>live</span>{/if}
            </button>
          {/each}
        </div>
        <div class="prow-add">
          <span class="preset-lead"><Plus size={12} strokeWidth={2.5} /> Add:</span>
          {#each PRESETS as def (def.preset)}
            <button class="chip-btn" type="button" onclick={() => addProvider(def)}>{def.name}</button>
          {/each}
          <button class="chip-btn" type="button" onclick={() => addProvider(null)}>Custom URL</button>
        </div>
      </div>

      {#if !selected}
        <!-- ── Claude panel ── -->
        <div class="st-block">
          <div class="st-row">
            <div class="st-row-body">
              <span class="st-row-label">Claude — via your Claude Code sign-in</span>
              <div class="st-row-desc">The default brain. You pick the model and reasoning effort in the chat composer, and usage counts against your Claude plan. Choosing a provider above sends chats to that endpoint instead — Rift's effort tiers and usage limits don't apply there.</div>
            </div>
            <div class="st-row-ctl">
              {#if providers.active}
                <button class="st-btn primary" type="button" disabled={switching} onclick={backToClaude}>
                  {#if switching}<Loader2 size={14} class="st-spin" /> Switching…{:else}<Power size={14} /> Switch to Claude{/if}
                </button>
              {:else}
                <span class="st-pill ok"><Check size={12} strokeWidth={3} />Active</span>
              {/if}
            </div>
          </div>
        </div>
      {:else}
        <!-- ── Selected provider cockpit ── -->
        <div class="st-block">
          <div class="st-block-label">
            {selected.name}{#if selected.preset}<span class="lbl-tag mono">{selected.preset}</span>{/if}
          </div>

          <div class="st-row">
            <div class="st-row-body">
              <label class="st-row-label" for="pv-name">Name</label>
              <div class="st-row-desc">Shown on the card above and on the model pill in chat.</div>
            </div>
            <div class="st-row-ctl">
              <input id="pv-name" class="st-input" type="text" bind:value={dName} onblur={saveDraft}
                style="width:100%; max-width:220px;" spellcheck="false" autocomplete="off" />
            </div>
          </div>

          <div class="st-row">
            <div class="st-row-body">
              <label class="st-row-label" for="pv-base">Base URL</label>
              <div class="st-row-desc">The endpoint must support the Anthropic <code>/v1/messages</code> API. Rift sets it as <code>ANTHROPIC_BASE_URL</code> for every chat turn while this provider is live.</div>
            </div>
            <div class="st-row-ctl">
              <input id="pv-base" class="st-input mono" type="text" bind:value={dBase} onblur={saveDraft}
                placeholder="https://api.example.com/anthropic" style="width:100%; max-width:260px;"
                spellcheck="false" autocapitalize="off" autocomplete="off" />
            </div>
          </div>

          <div class="st-row">
            <div class="st-row-body">
              <label class="st-row-label" for="pv-model">Model</label>
              <div class="st-row-desc">
                Type a model name, or click <strong>Detect</strong> to list what the endpoint serves.
                {#if modelHint}<span class="st-detect-hint">{modelHint}</span>{/if}
              </div>
            </div>
            <div class="st-row-ctl">
              <input id="pv-model" class="st-input mono" type="text" list="pv-models" bind:value={dModel} onblur={saveDraft}
                placeholder="model id" style="width:200px;" spellcheck="false" autocapitalize="off" autocomplete="off" />
              <datalist id="pv-models">
                {#each selected.models as m (m)}<option value={m}></option>{/each}
              </datalist>
              <button class="st-btn" type="button" disabled={providers.list.length === 0} onclick={detectModels}>
                <RefreshCw size={14} /> Detect
              </button>
            </div>
            {#if selected.models.length > 0}
              <div class="model-list">
                {#each selected.models as m (m)}
                  <button class="chip-btn" type="button" onclick={() => selectModel(m)} class:active={dModel.trim() === m}>
                    {#if dModel.trim() === m}<Check size={12} strokeWidth={3} />{/if}{m}
                  </button>
                {/each}
              </div>
            {/if}
          </div>

          <div class="st-row">
            <div class="st-row-body">
              <span class="st-row-label">Reasoning effort</span>
              <div class="st-row-desc">
                The endpoint honors Anthropic extended-thinking — Rift shows the effort ladder in
                the composer and sends <code>--effort</code> per turn. On for the reasoning clouds
                (Kimi / DeepSeek / GLM); keep it off for Ollama and proxies that reject a
                <code>thinking</code> block.
              </div>
            </div>
            <div class="st-row-ctl">
              <button class="rift-toggle" class:on={dEffort} role="switch" aria-checked={dEffort}
                aria-label="Reasoning effort" type="button" onclick={() => void toggleEffort()}><span class="rift-toggle-knob"></span></button>
            </div>
          </div>

          <div class="st-row">
            <div class="st-row-body">
              <span class="st-row-label">API key</span>
              <div class="st-row-desc">Stored in your OS keychain, never in a config file. {#if selected.preset}Needs: {PRESETS.find((d) => d.preset === selected?.preset)?.keyHint ?? ""}.{/if}</div>
            </div>
            <div class="st-row-ctl">
              {#if selected.has_key}
                <span class="st-pill ok"><span class="dot"></span>Set</span>
                <button class="st-btn" type="button" disabled={keySaving} onclick={clearKey}>Clear</button>
              {:else}
                <span class="st-secret">
                  <input class="st-input mono" type={keyVisible ? "text" : "password"} bind:value={keyInput}
                    placeholder="paste key" style="width:100%; max-width:170px;"
                    spellcheck="false" autocapitalize="off" autocomplete="off" />
                  <button class="st-eye" type="button" onclick={() => (keyVisible = !keyVisible)} aria-label={keyVisible ? "Hide key" : "Show key"}>
                    {#if keyVisible}<EyeOff size={14} />{:else}<Eye size={14} />{/if}
                  </button>
                </span>
                <button class="st-btn primary" type="button" disabled={keySaving || !keyInput.trim()} onclick={saveKey}>
                  {keySaving ? "Saving…" : "Save"}
                </button>
              {/if}
            </div>
          </div>

          <div class="st-row">
            <div class="st-row-body">
              <span class="st-row-label">Verify connection</span>
              <div class="st-row-desc">Sends a one-line prompt and shows the reply — run this before switching your chats over.</div>
              {#if testResult}
                <div class="ll-result mono" class:err={!testResult.ok}>{testResult.msg}</div>
              {/if}
            </div>
            <div class="st-row-ctl">
              {#if testResult}
                <span class="st-pill {testResult.ok ? 'ok' : 'warn'}"><span class="dot"></span>{testResult.ok ? "OK" : "Failed"}</span>
              {/if}
              {#if tokPerSec != null}
                <span class="rv-latency" title="Approximate — output tokens over total round-trip"><Gauge size={11} strokeWidth={2.5} />~{tokPerSec} tok/s</span>
              {/if}
              {#if testMs != null}
                <span class="rv-latency"><Zap size={11} strokeWidth={2.5} />{testMs} ms</span>
              {/if}
              <button class="st-btn primary" type="button" disabled={!canTest || testing} onclick={testConnection}>
                {#if testing}<Loader2 size={14} class="st-spin" /> Testing…{:else}<FlaskConical size={14} /> Test{/if}
              </button>
            </div>
          </div>

          {#if showCtx && ctx}
            <div class="st-row ctx-row" class:bad={localLlm.ctxUndersized}>
              <div class="st-row-body">
                <span class="st-row-label"><Gauge size={14} strokeWidth={2} /> Context window</span>
                <div class="st-row-desc">
                  {#if localLlm.ctxUndersized}
                    Ollama is serving this model at
                    <strong>{ctx.num_ctx == null ? "the 4096-token default" : `only ${fmt(ctx.num_ctx)} tokens`}</strong>
                    — too small for Rift's system prompt, tools, and files.
                    {#if ctx.max_ctx}This model supports up to {fmt(ctx.max_ctx)}.{/if}
                  {:else}
                    Serving <strong>{fmt(ctx.num_ctx ?? 0)} tokens</strong> — enough headroom for
                    agentic edits.{#if ctx.max_ctx} Model ceiling {fmt(ctx.max_ctx)}.{/if}
                  {/if}
                </div>
                {#if cardBits.length > 0}
                  <div class="model-card">
                    {#each cardBits as b (b)}<span class="mc-tag mono">{b}</span>{/each}
                  </div>
                {/if}
                {#if localLlm.ctxUndersized}
                  <div class="ctx-targets">
                    <span class="preset-lead">Target:</span>
                    {#each CTX_TARGETS as t (t.value)}
                      <button class="chip-btn" type="button"
                        disabled={localLlm.optimizing || (!!ctx.max_ctx && t.value > ctx.max_ctx)}
                        class:active={optimizeTarget === t.value}
                        onclick={() => (optimizeTarget = t.value)}>{t.label}</button>
                    {/each}
                  </div>
                {/if}
                {#if optimizeResult}
                  <div class="ctx-result mono" class:err={!optimizeResult.ok}>
                    {optimizeResult.ok ? `Created ${optimizeResult.msg} (${fmt(optimizeTarget)} tokens) — now active.` : optimizeResult.msg}
                  </div>
                {/if}
              </div>
              <div class="st-row-ctl">
                {#if localLlm.ctxUndersized}
                  <span class="st-pill warn"><AlertTriangle size={12} strokeWidth={2.5} />Too small</span>
                  <button class="st-btn primary" type="button" disabled={localLlm.optimizing} onclick={optimizeCtx}>
                    {#if localLlm.optimizing}<Loader2 size={14} class="st-spin" /> Optimizing…{:else}<Sparkles size={14} /> Optimize for Rift{/if}
                  </button>
                {:else}
                  <span class="st-pill ok"><Check size={12} strokeWidth={3} />{fmt(ctx.num_ctx ?? 0)}</span>
                {/if}
              </div>
            </div>
          {/if}

          <!-- Footer actions -->
          <div class="st-row card-foot">
            <div class="st-row-body">
              {#if saveError}<div class="ll-result mono err">{saveError}</div>{/if}
            </div>
            <div class="st-row-ctl">
              <button class="st-btn danger" type="button" onclick={deleteSelected}>
                <Trash2 size={14} /> {confirmDelete ? "Click again to confirm" : "Remove"}
              </button>
              {#if selected.active}
                <button class="st-btn" type="button" disabled={switching} onclick={backToClaude}>
                  {#if switching}<Loader2 size={14} class="st-spin" /> Switching…{:else}<Power size={14} /> Back to Claude{/if}
                </button>
              {:else}
                <button class="st-btn primary" type="button" disabled={switching} onclick={activateSelected}>
                  {#if switching}<Loader2 size={14} class="st-spin" /> Switching…{:else}<Power size={14} /> Use for chats{/if}
                </button>
              {/if}
            </div>
          </div>
        </div>
      {/if}

      <div class="st-warn">
        Heads up: Rift still runs everything through the Claude CLI — a provider only swaps which model answers. How well a model handles edits and tool calls varies; if one struggles, that's the model, not Rift. Switching the chat brain mid-conversation starts a fresh chat (the old one is saved in History).
      </div>

    </div>
  </div>
</div>

<style>
  .sb-main { position: relative; overflow: hidden; display: flex; flex-direction: column; flex: 1; min-height: 0; min-width: 0; background: transparent; color: var(--fg); }

  .sb-chip { display: inline-flex; align-items: center; gap: 7px; height: 30px; padding: 0 12px; border-radius: 999px; background: var(--surface); border: 1px solid var(--border); color: var(--fg-2); font: inherit; font-size: var(--fs-xs); font-weight: 600; }
  .sb-chip .dot { width: 7px; height: 7px; border-radius: 999px; background: var(--fg-faint); }
  .sb-chip.verified { background: var(--ok-soft); border-color: color-mix(in oklch, var(--ok) 28%, transparent); color: var(--ok); }
  .sb-chip.verified .dot { background: var(--ok); }

  .sb-scroll { flex: 1; min-width: 0; min-height: 0; overflow-y: auto; overflow-x: hidden; scroll-behavior: smooth; }
  .sb-wrap { max-width: 820px; margin: 0 auto; padding: 18px 40px 22px; display: flex; flex-direction: column; gap: 12px; }

  /* ── Chat-brain rail ── */
  .prow { display: flex; flex-wrap: wrap; gap: 9px; padding: 13px 17px; }
  .pcard { display: flex; flex-direction: column; align-items: flex-start; gap: 3px; min-width: 150px; max-width: 230px; padding: 10px 13px; border-radius: var(--radius); border: 1px solid var(--border); background: var(--field); font: inherit; text-align: left; cursor: pointer; transition: background var(--dur-fast), border-color var(--dur-fast); }
  .pcard:hover { background: var(--surface-hover); border-color: var(--border-strong); }
  .pcard.sel { border-color: color-mix(in oklch, var(--accent) 45%, var(--border)); background: color-mix(in oklab, var(--accent) 6%, var(--field)); }
  .pcard.live { border-color: color-mix(in oklch, var(--ok) 38%, var(--border)); }
  .pcard-name { font-size: var(--fs-sm); font-weight: 650; color: var(--fg); }
  .pcard-sub { font-size: 10.5px; color: var(--fg-muted); max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pcard .st-pill { margin-top: 3px; }

  .prow-add { display: flex; align-items: center; flex-wrap: wrap; gap: 7px; padding: 11px 17px 13px; border-top: 1px solid var(--border); }
  .prow-add .preset-lead { display: inline-flex; align-items: center; gap: 4px; }

  /* ── Blocks & rows (shared look with Settings) ── */
  .st-block { display: flex; flex-direction: column; background: var(--surface); border: 1px solid var(--border); border-radius: var(--r-card); box-shadow: inset 0 1px 0 color-mix(in oklab, white 4%, transparent), var(--shadow-sm); }
  .st-block-label { display: flex; align-items: center; gap: 8px; font-size: 10px; font-weight: 700; letter-spacing: 0.09em; text-transform: uppercase; color: var(--fg-faint); padding: 11px 17px; border-bottom: 1px solid var(--border); }
  .lbl-tag { font-size: 9.5px; font-weight: 600; letter-spacing: 0; text-transform: none; color: var(--accent); background: var(--accent-soft); border-radius: 4px; padding: 1px 6px; }

  .st-row { display: flex; flex-wrap: wrap; align-items: center; gap: 12px 16px; padding: 14px 17px; }
  .st-row + .st-row { border-top: 1px solid var(--border); }
  .st-row-body { flex: 1 1 300px; min-width: 0; }
  .st-row-label { font-size: var(--fs-md); font-weight: 600; color: var(--fg); display: block; }
  .st-row-desc { font-size: var(--fs-xs); color: var(--fg-muted); margin-top: 3px; line-height: 1.5; max-width: 60ch; }
  .st-row-desc code { font-family: var(--font-mono); color: var(--code-fg); background: var(--code-bg); border: 1px solid var(--code-border); padding: 1px 5px; border-radius: 4px; font-size: 0.9em; }
  .st-row-ctl { flex: 0 1 auto; margin-left: auto; display: flex; align-items: center; gap: 10px; flex-wrap: wrap; justify-content: flex-end; }
  .card-foot { background: color-mix(in oklab, var(--fg) 2.5%, transparent); }

  .preset-lead { font-size: var(--fs-xs); color: var(--fg-subtle); margin-right: 2px; }
  .model-list { flex-basis: 100%; display: flex; align-items: center; flex-wrap: wrap; gap: 7px; }
  .chip-btn { display: inline-flex; align-items: center; gap: 5px; height: 26px; padding: 0 10px; border-radius: 999px; font: inherit; font-size: var(--fs-xs); font-weight: 600; cursor: pointer; border: 1px solid var(--border); background: var(--field); color: var(--fg-2); font-family: var(--font-mono); transition: background var(--dur-fast), border-color var(--dur-fast), color var(--dur-fast); }
  .chip-btn:hover:not(:disabled) { background: var(--surface-hover); border-color: var(--border-strong); color: var(--fg); }
  .chip-btn.active { background: var(--ok-soft); border-color: color-mix(in oklch, var(--ok) 32%, transparent); color: var(--ok); }
  .chip-btn:disabled { opacity: 0.45; cursor: not-allowed; }

  .rv-latency { display: inline-flex; align-items: center; gap: 3px; font-size: var(--fs-xs); font-weight: 650; color: var(--accent); }
  .ll-result { white-space: pre-wrap; word-break: break-word; color: var(--ok); font-size: var(--fs-xs); line-height: 1.5; background: var(--bg-inset, color-mix(in oklab, var(--fg) 5%, transparent)); border: 1px solid var(--border); border-radius: var(--radius); padding: 9px 11px; max-height: 160px; overflow: auto; margin-top: 8px; }
  .ll-result.err { color: var(--danger); }

  /* ── Context-window guidance row ── */
  .ctx-row .st-row-label { display: inline-flex; align-items: center; gap: 6px; }
  .ctx-row.bad { background: var(--warn-soft); }
  .ctx-row.bad .st-row-label :global(svg) { color: var(--warn); }
  .ctx-result { flex-basis: 100%; margin-top: 8px; white-space: pre-wrap; word-break: break-word; color: var(--ok); font-size: var(--fs-xs); line-height: 1.5; background: var(--bg-inset, color-mix(in oklab, var(--fg) 5%, transparent)); border: 1px solid var(--border); border-radius: var(--radius); padding: 8px 11px; }
  .ctx-result.err { color: var(--danger); }
  .model-card { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 8px; }
  .mc-tag { font-size: 10.5px; font-weight: 600; color: var(--code-fg); background: var(--code-bg); border: 1px solid var(--code-border); border-radius: 4px; padding: 2px 7px; }
  .ctx-targets { display: flex; align-items: center; flex-wrap: wrap; gap: 7px; margin-top: 10px; }

  .st-detect-hint { color: var(--accent); font-weight: 600; margin-left: 2px; }
  .st-warn { display: block; font-size: var(--fs-xs); color: var(--warn); line-height: 1.5; padding: 10px 13px; background: var(--warn-soft); border: 1px solid color-mix(in oklab, var(--warn) 32%, transparent); border-radius: var(--r-card); }

  /* Input · button · status-pill kit → $lib/styles/settings-controls.css */
</style>

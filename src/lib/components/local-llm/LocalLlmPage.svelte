<script lang="ts">
  import { onMount } from "svelte";
  import { Cpu, FlaskConical, Loader2, Eye, EyeOff, RefreshCw, Check, Zap, ArrowRight, Gauge, Sparkles, AlertTriangle } from "lucide-svelte";
  import PageHero from "../shared/PageHero.svelte";
  import { localLlm } from "../../state/localLlm.svelte";

  let keyInput = $state("");
  let keyVisible = $state(false);

  let keySaving = $state(false);
  let testing = $state(false);
  let testResult = $state<{ ok: boolean; msg: string } | null>(null);
  let testMs = $state<number | null>(null);
  let testTokens = $state<number | null>(null);
  let lastChecked = $state<Date | null>(null);
  let modelHint = $state("");
  let optimizeResult = $state<{ ok: boolean; msg: string } | null>(null);
  let optimizeTarget = $state(32768);

  // Both speak the Anthropic /v1/messages API: current Ollama serves it natively
  // at :11434, and a LiteLLM proxy fronts it at :4000.
  const PRESETS = [
    { label: "Ollama", url: "http://localhost:11434" },
    { label: "LiteLLM", url: "http://localhost:4000" },
  ] as const;

  // Curated Ollama tags that hold up under Rift's MCP tool surface + agentic
  // edits. Free-text field stays authoritative — these are one-click conveniences.
  const RECOMMENDED = [
    "qwen3-coder:30b",
    "qwen2.5-coder:14b",
    "qwen2.5-coder:7b",
    "devstral:24b",
  ] as const;

  // Optimize targets. Disabled above the model's advertised ceiling.
  const CTX_TARGETS = [
    { label: "16K", value: 16384 },
    { label: "32K", value: 32768 },
    { label: "64K", value: 65536 },
    { label: "128K", value: 131072 },
  ] as const;

  // Test needs a target — gate the button so an empty probe can't fire.
  const canTest = $derived(
    localLlm.enabled && localLlm.baseUrl.trim().length > 0 && localLlm.model.trim().length > 0,
  );

  const hasEndpoint = $derived(localLlm.baseUrl.trim().length > 0);
  const hasModel = $derived(localLlm.model.trim().length > 0);
  const verified = $derived(testResult?.ok === true);

  // Readiness state machine — drives the rail badge + hero tone.
  const readiness = $derived.by(() => {
    if (!localLlm.enabled) return { key: "off", label: "Local mode off", sub: "Turn it on to configure an endpoint." };
    if (!hasEndpoint || !hasModel) return { key: "incomplete", label: "Setup incomplete", sub: "Endpoint and model are required." };
    if (!verified) return { key: "ready", label: "Ready to verify", sub: "Config looks complete — run a test." };
    return { key: "verified", label: "Verified & live", sub: "Turns route through your endpoint." };
  });

  const steps = $derived([
    { label: "Endpoint", done: hasEndpoint, hint: hasEndpoint ? localLlm.baseUrl : "Not set", optional: false },
    { label: "Model", done: hasModel, hint: hasModel ? localLlm.model : "Not set", optional: false },
    { label: "API key", done: localLlm.hasKey, hint: localLlm.hasKey ? "Configured" : "Optional", optional: true },
    { label: "Verified", done: verified, hint: verified && testMs != null ? `${testMs} ms round-trip` : "Not tested", optional: false },
  ]);

  // Context guidance — only meaningful for an Ollama endpoint with a model set.
  const ctx = $derived(localLlm.ctxInfo);
  const showCtx = $derived(localLlm.enabled && !!ctx && ctx.is_ollama);
  const fmt = (n: number) => n.toLocaleString();

  // Model card line — any subset of family / params / quant the probe surfaced.
  const cardBits = $derived(
    [ctx?.family, ctx?.params, ctx?.quant].filter((s): s is string => !!s),
  );

  // Approximate throughput: output tokens over the measured round-trip. Includes
  // connect + first-token latency so it under-reads true generation speed — a
  // floor, not a benchmark. Hidden when the endpoint reports no usage.
  const tokPerSec = $derived(
    testTokens != null && testMs != null && testMs > 0
      ? Math.round(testTokens / (testMs / 1000))
      : null,
  );

  onMount(() => { void localLlm.refresh().then(() => localLlm.refreshCtx()); });

  // A config change invalidates a prior pass — drop the verified state so the
  // rail never claims "live" against settings that were never tested.
  function invalidateTest() { testResult = null; testMs = null; testTokens = null; lastChecked = null; }

  async function toggleEnabled() {
    try { await localLlm.setEnabled(!localLlm.enabled); } catch { /* reverted in store */ }
  }

  async function applyPreset(url: string) {
    localLlm.baseUrl = url;
    invalidateTest();
    await localLlm.saveBaseUrl();
  }

  async function saveBaseUrl() { invalidateTest(); await localLlm.saveBaseUrl(); void localLlm.refreshCtx(); }

  async function saveModel() {
    invalidateTest();
    optimizeResult = null;
    try { await localLlm.saveModel(); void localLlm.refreshCtx(); }
    catch (e) { testResult = { ok: false, msg: String(e) }; }
  }

  async function optimizeCtx() {
    optimizeResult = null;
    invalidateTest();
    optimizeResult = await localLlm.optimize(optimizeTarget);
  }

  async function selectModel(m: string) {
    localLlm.model = m;
    await saveModel();
  }

  async function saveKey() {
    keySaving = true;
    try { await localLlm.saveKey(keyInput); keyInput = ""; }
    catch (e) { console.error("set key failed", e); }
    finally { keySaving = false; }
  }

  async function clearKey() {
    keySaving = true;
    try { await localLlm.saveKey(null); keyInput = ""; }
    catch (e) { console.error("clear key failed", e); }
    finally { keySaving = false; }
  }

  async function detectModels() {
    modelHint = "";
    const n = await localLlm.listModels();
    modelHint = n === 0 ? "No models found at this endpoint." : `${n} model${n === 1 ? "" : "s"} found.`;
  }

  async function testConnection() {
    testing = true;
    testResult = null;
    testMs = null;
    testTokens = null;
    const t0 = performance.now();
    const r = await localLlm.test();
    testMs = Math.round(performance.now() - t0);
    testTokens = r.tokens ?? null;
    testResult = { ok: r.ok, msg: r.msg };
    lastChecked = new Date();
    testing = false;
  }
</script>

<div class="sb-main">
  <PageHero
    eyebrow="Experimental"
    title="Local LLM"
    desc="Route turns through a local Anthropic-compatible endpoint (LiteLLM / Ollama) instead of your Claude session. Toggle off any time to return to normal Claude."
  >
    {#snippet icon()}<Cpu size={22} strokeWidth={1.75} />{/snippet}
    {#snippet chip()}
      <span class="sb-chip {readiness.key}">
        <span class="dot"></span>{localLlm.enabled ? readiness.label : "Off"}
      </span>
    {/snippet}
  </PageHero>

  <div class="sb-scroll">
    <div class="sb-wrap">

      <!-- ── Mode master strip ── -->
      <div class="st-block mode-bar">
        <div class="mode-main">
          <div class="mode-body">
            <div class="mode-title">Local LLM mode</div>
            <div class="mode-desc">When on, every turn spawns the CLI against your local endpoint with <code>--bare</code>. Bypasses cloud auth, effort tiers, and per-conversation model pinning.</div>
          </div>
          <button
            class="st-switch" class:on={localLlm.enabled}
            role="switch" aria-checked={localLlm.enabled} aria-label="Toggle local LLM mode"
            disabled={!localLlm.loaded} type="button" onclick={toggleEnabled}
          ></button>
        </div>
      </div>

      <!-- ── Cockpit: status rail + config ── -->
      <div class="cockpit">

        <!-- Status rail -->
        <aside class="st-block rail rail-{readiness.key}">
          <div class="rail-state">
            <span class="rail-dot"></span>
            <div>
              <div class="rail-label">{readiness.label}</div>
              <div class="rail-sub">{readiness.sub}</div>
            </div>
          </div>

          {#if localLlm.enabled}
            <ol class="rail-steps">
              {#each steps as s (s.label)}
                <li class:done={s.done} class:opt={s.optional}>
                  <span class="step-mark">{#if s.done}<Check size={12} strokeWidth={3} />{/if}</span>
                  <span class="step-label">{s.label}{#if s.optional && !s.done}<span class="step-opt"> · optional</span>{/if}</span>
                  <span class="step-hint mono">{s.hint}</span>
                </li>
              {/each}
            </ol>

            <div class="rail-verify">
              <div class="rv-head">
                <span class="rv-title">Verify connection</span>
                <span class="rv-metrics">
                  {#if tokPerSec != null}
                    <span class="rv-latency" title="Approximate — output tokens over total round-trip"><Gauge size={11} strokeWidth={2.5} />~{tokPerSec} tok/s</span>
                  {/if}
                  {#if testMs != null}
                    <span class="rv-latency"><Zap size={11} strokeWidth={2.5} />{testMs} ms</span>
                  {/if}
                </span>
              </div>
              <div class="rv-desc">Round-trips a one-line prompt and reports the reply.</div>
              <div class="rv-actions">
                {#if testResult}
                  <span class="st-pill {testResult.ok ? 'ok' : 'warn'}"><span class="dot"></span>{testResult.ok ? "OK" : "Failed"}</span>
                {/if}
                {#if lastChecked}
                  <span class="rv-stamp">checked {lastChecked.toLocaleTimeString()}</span>
                {/if}
                <button class="st-btn primary" type="button" disabled={!canTest || testing} onclick={testConnection}>
                  {#if testing}<Loader2 size={14} class="st-spin" /> Testing…{:else}<FlaskConical size={14} /> Test{/if}
                </button>
              </div>
              {#if testResult}
                <div class="ll-result mono" class:err={!testResult.ok}>{testResult.msg}</div>
              {/if}
            </div>

            <!-- API key — lives in the rail so the config column stays balanced -->
            <div class="rail-key">
              <div class="rk-head">
                <span class="rv-title">API key</span>
                {#if localLlm.hasKey}<span class="st-pill ok"><span class="dot"></span>Set</span>{/if}
              </div>
              <div class="rv-desc">OS keychain, never in config. Most local proxies accept any non-empty value. Sets <code>ANTHROPIC_API_KEY</code>.</div>
              <div class="rk-actions">
                {#if localLlm.hasKey}
                  <button class="st-btn" type="button" disabled={!localLlm.enabled || keySaving} onclick={clearKey}>Clear key</button>
                {:else}
                  <span class="st-secret">
                    <input
                      id="ll-key" class="st-input mono" type={keyVisible ? "text" : "password"}
                      bind:value={keyInput} disabled={!localLlm.enabled}
                      placeholder="any non-empty value" style="width:158px;"
                      spellcheck="false" autocapitalize="off" autocomplete="off"
                    />
                    <button class="st-eye" type="button" onclick={() => (keyVisible = !keyVisible)} aria-label={keyVisible ? "Hide key" : "Show key"}>
                      {#if keyVisible}<EyeOff size={14} />{:else}<Eye size={14} />{/if}
                    </button>
                  </span>
                  <button class="st-btn primary" type="button" disabled={!localLlm.enabled || keySaving || !keyInput.trim()} onclick={saveKey}>
                    {keySaving ? "Saving…" : "Save"}
                  </button>
                {/if}
              </div>
            </div>
          {:else}
            <!-- Off-state flow explainer -->
            <div class="rail-flow">
              <span class="flow-node">Rift</span>
              <ArrowRight size={14} class="flow-arrow" />
              <span class="flow-node">Endpoint</span>
              <ArrowRight size={14} class="flow-arrow" />
              <span class="flow-node">Local model</span>
            </div>
            <div class="rail-flow-note">Enable Local mode to point Rift at your own endpoint instead of the Claude cloud.</div>
          {/if}
        </aside>

        <!-- Config -->
        <div class="st-block config" data-disabled={!localLlm.enabled}>
          <div class="st-block-label">Endpoint &amp; model</div>

          <!-- Base URL + presets -->
          <div class="st-row">
            <div class="st-row-body">
              <label class="st-row-label" for="ll-base">Base URL</label>
              <div class="st-row-desc">Must speak the Anthropic <code>/v1/messages</code> API — <strong>Ollama</strong> serves it natively (<code>:11434</code>), or a <strong>LiteLLM proxy</strong> (<code>:4000</code>). Sets <code>ANTHROPIC_BASE_URL</code>.</div>
            </div>
            <div class="st-row-ctl">
              <input
                id="ll-base" class="st-input mono" type="text"
                bind:value={localLlm.baseUrl} onblur={saveBaseUrl} disabled={!localLlm.enabled}
                placeholder="http://localhost:4000" style="width:220px;"
                spellcheck="false" autocapitalize="off" autocomplete="off"
              />
            </div>
            <div class="preset-row">
              <span class="preset-lead">Quick start:</span>
              {#each PRESETS as p (p.url)}
                <button class="chip-btn" type="button" disabled={!localLlm.enabled} onclick={() => applyPreset(p.url)} class:active={localLlm.baseUrl.trim() === p.url}>{p.label}</button>
              {/each}
            </div>
          </div>

          <!-- Model + detect + detected list -->
          <div class="st-row">
            <div class="st-row-body">
              <label class="st-row-label" for="ll-model">Model</label>
              <div class="st-row-desc">
                Passed as <code>--model</code>. Type one or <strong>Detect</strong> what the endpoint serves.
                {#if modelHint}<span class="st-detect-hint">{modelHint}</span>{/if}
              </div>
            </div>
            <div class="st-row-ctl">
              <input
                id="ll-model" class="st-input mono" type="text" list="ll-models"
                bind:value={localLlm.model} onblur={saveModel} disabled={!localLlm.enabled}
                placeholder="ollama/llama3" style="width:200px;"
                spellcheck="false" autocapitalize="off" autocomplete="off"
              />
              <datalist id="ll-models">
                {#each localLlm.models as m (m)}<option value={m}></option>{/each}
              </datalist>
              <button class="st-btn" type="button" disabled={!localLlm.enabled || localLlm.detecting} onclick={detectModels}>
                {#if localLlm.detecting}<Loader2 size={14} class="st-spin" /> …{:else}<RefreshCw size={14} /> Detect{/if}
              </button>
            </div>
            {#if localLlm.models.length > 0}
              <div class="model-list">
                {#each localLlm.models as m (m)}
                  <button class="chip-btn" type="button" disabled={!localLlm.enabled} onclick={() => selectModel(m)} class:active={localLlm.model.trim() === m}>
                    {#if localLlm.model.trim() === m}<Check size={12} strokeWidth={3} />{/if}{m}
                  </button>
                {/each}
              </div>
            {:else}
              <div class="model-list">
                <span class="preset-lead">Good for tools:</span>
                {#each RECOMMENDED as m (m)}
                  <button class="chip-btn" type="button" disabled={!localLlm.enabled} onclick={() => selectModel(m)} class:active={localLlm.model.trim() === m}>
                    {#if localLlm.model.trim() === m}<Check size={12} strokeWidth={3} />{/if}{m}
                  </button>
                {/each}
              </div>
            {/if}
          </div>

          <!-- Context window — Ollama's 4096 default silently truncates Rift's
               prompt + tools + files → stalls / refused edits. Detect + one-click fix. -->
          {#if showCtx && ctx}
            <div class="st-row ctx-row" class:bad={localLlm.ctxUndersized}>
              <div class="st-row-body">
                <label class="st-row-label">
                  <Gauge size={14} strokeWidth={2} />
                  Context window
                </label>
                <div class="st-row-desc">
                  {#if localLlm.ctxUndersized}
                    Ollama is serving this model at
                    <strong>{ctx.num_ctx == null ? "the 4096-token default" : `only ${fmt(ctx.num_ctx)} tokens`}</strong>
                    — too small for Rift's system prompt, tools, and files. This is the
                    <strong>#1 cause of stalls and refused edits</strong> in local mode.
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
                      <button
                        class="chip-btn" type="button"
                        disabled={localLlm.optimizing || (!!ctx.max_ctx && t.value > ctx.max_ctx)}
                        class:active={optimizeTarget === t.value}
                        title={!!ctx.max_ctx && t.value > ctx.max_ctx ? "Exceeds this model's ceiling" : ""}
                        onclick={() => (optimizeTarget = t.value)}
                      >{t.label}</button>
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

        </div>

        <!-- Full-width footer — spans both cockpit columns -->
        <div class="st-warn cockpit-foot">
          Tool-calling fidelity depends on the local model — small models may struggle with Rift's MCP tools.
        </div>

      </div>
    </div>
  </div>
</div>

<style>
  .sb-main { position: relative; overflow: hidden; display: flex; flex-direction: column; flex: 1; min-height: 0; min-width: 0; background: transparent; color: var(--fg); }

  /* Hero status chip — tinted per readiness state (snippet defined in-file → scoped here). */
  .sb-chip { display: inline-flex; align-items: center; gap: 7px; height: 30px; padding: 0 12px; border-radius: 999px; background: var(--surface); border: 1px solid var(--border); color: var(--fg-2); font: inherit; font-size: var(--fs-xs); font-weight: 600; }
  .sb-chip .dot { width: 7px; height: 7px; border-radius: 999px; background: var(--fg-faint); }
  .sb-chip.ready { background: var(--accent-soft, color-mix(in oklab, var(--accent) 14%, transparent)); border-color: color-mix(in oklch, var(--accent) 30%, transparent); color: var(--accent); }
  .sb-chip.ready .dot { background: var(--accent); }
  .sb-chip.incomplete { background: var(--warn-soft); border-color: color-mix(in oklab, var(--warn) 28%, transparent); color: var(--warn); }
  .sb-chip.incomplete .dot { background: var(--warn); }
  .sb-chip.verified { background: var(--ok-soft); border-color: color-mix(in oklch, var(--ok) 28%, transparent); color: var(--ok); }
  .sb-chip.verified .dot { background: var(--ok); }

  .sb-scroll { flex: 1; min-width: 0; min-height: 0; overflow-y: auto; overflow-x: hidden; scroll-behavior: smooth; }
  .sb-wrap { max-width: 820px; margin: 0 auto; padding: 18px 40px 22px; display: flex; flex-direction: column; gap: 12px; }

  /* ── Mode master strip ── */
  .mode-bar { transition: border-color 200ms var(--ease-soft), background 200ms var(--ease-soft); }
  .mode-bar:has(.st-switch.on) { border-color: color-mix(in oklch, var(--accent) 30%, var(--border)); background: color-mix(in oklab, var(--accent) 5%, var(--surface)); }
  .mode-bar .mode-main { display: flex; align-items: center; gap: 16px; padding: 15px 18px; }
  .mode-body { flex: 1 1 auto; min-width: 0; }
  .mode-title { font-size: var(--fs-md); font-weight: 650; color: var(--fg); }
  .mode-desc { font-size: var(--fs-xs); color: var(--fg-muted); margin-top: 3px; line-height: 1.5; max-width: 72ch; }
  .mode-desc code { font-family: var(--font-mono); color: var(--fg-2); background: color-mix(in oklab, var(--fg) 6%, transparent); padding: 0 4px; border-radius: 4px; font-size: 0.9em; }

  /* ── Cockpit grid ── */
  .cockpit { display: grid; grid-template-columns: minmax(0, 286px) minmax(0, 1fr); gap: 12px; align-items: start; }
  .cockpit-foot { grid-column: 1 / -1; }
  @media (max-width: 760px) { .cockpit { grid-template-columns: 1fr; } }

  /* ── Status rail ── */
  .rail { position: relative; padding: 16px 17px; gap: 16px; overflow: hidden; }
  .rail::before { content: ""; position: absolute; inset: 0 0 auto 0; height: 2px; background: var(--fg-faint); opacity: 0.7; }
  .rail-incomplete::before { background: var(--warn); }
  .rail-ready::before { background: var(--accent); }
  .rail-verified::before { background: var(--ok); }
  .rail-state { display: flex; align-items: flex-start; gap: 11px; }
  .rail-dot { width: 10px; height: 10px; border-radius: 999px; margin-top: 4px; flex: none; background: var(--fg-faint); box-shadow: 0 0 0 4px color-mix(in oklab, var(--fg-faint) 18%, transparent); }
  .rail-off .rail-dot { background: var(--fg-faint); box-shadow: 0 0 0 4px color-mix(in oklab, var(--fg-faint) 16%, transparent); }
  .rail-incomplete .rail-dot { background: var(--warn); box-shadow: 0 0 0 4px var(--warn-soft); }
  .rail-ready .rail-dot { background: var(--accent); box-shadow: 0 0 0 4px color-mix(in oklab, var(--accent) 20%, transparent); }
  .rail-verified .rail-dot { background: var(--ok); box-shadow: 0 0 0 4px var(--ok-soft); }
  .rail-label { font-size: var(--fs-md); font-weight: 650; color: var(--fg); }
  .rail-sub { font-size: var(--fs-xs); color: var(--fg-muted); margin-top: 2px; line-height: 1.45; }

  .rail-steps { list-style: none; margin: 0; padding: 14px 0 0; border-top: 1px solid var(--border); display: flex; flex-direction: column; gap: 11px; }
  .rail-steps li { display: grid; grid-template-columns: 18px 1fr auto; align-items: center; gap: 9px; }
  .step-mark { width: 18px; height: 18px; border-radius: 999px; display: grid; place-items: center; border: 1.5px solid var(--border-strong); color: var(--accent-fg); background: transparent; }
  .rail-steps li.done .step-mark { background: var(--ok); border-color: transparent; color: var(--accent-fg); }
  .step-label { font-size: var(--fs-sm); font-weight: 600; color: var(--fg-2); }
  .rail-steps li.done .step-label { color: var(--fg); }
  .step-opt { font-weight: 500; color: var(--fg-subtle); }
  .step-hint { font-size: var(--fs-xs); color: var(--fg-muted); max-width: 130px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; text-align: right; }

  .rail-verify { border-top: 1px solid var(--border); padding-top: 14px; display: flex; flex-direction: column; gap: 8px; }
  .rv-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  .rv-title { font-size: var(--fs-sm); font-weight: 650; color: var(--fg); }
  .rv-latency { display: inline-flex; align-items: center; gap: 3px; font-size: var(--fs-xs); font-weight: 650; color: var(--accent); }
  .rv-desc { font-size: var(--fs-xs); color: var(--fg-muted); line-height: 1.45; }
  .rv-actions { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; margin-top: 2px; }
  .rv-stamp { font-size: var(--fs-2xs, 10.5px); color: var(--fg-subtle); }
  .rail-verify .st-btn.primary { margin-left: auto; }
  .ll-result { white-space: pre-wrap; word-break: break-word; color: var(--ok); font-size: var(--fs-xs); line-height: 1.5; background: var(--bg-inset, color-mix(in oklab, var(--fg) 5%, transparent)); border: 1px solid var(--border); border-radius: var(--radius); padding: 9px 11px; max-height: 160px; overflow: auto; }
  .ll-result.err { color: var(--danger); }

  /* ── API key (rail-resident) ── */
  .rail-key { border-top: 1px solid var(--border); padding-top: 14px; display: flex; flex-direction: column; gap: 8px; }
  .rk-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  .rk-actions { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; margin-top: 2px; }
  .rk-actions > .st-btn:only-child { margin-left: auto; }
  .rail-key .st-btn.primary { margin-left: auto; }
  .rk-actions .st-secret .st-input { width: 158px; }

  .rail-flow { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; padding-top: 14px; border-top: 1px solid var(--border); }
  .flow-node { font-size: var(--fs-xs); font-weight: 600; color: var(--fg-2); padding: 5px 11px; border-radius: 999px; background: var(--field); border: 1px solid var(--border); }
  .rail-flow :global(.flow-arrow) { color: var(--fg-faint); }
  .rail-flow-note { font-size: var(--fs-xs); color: var(--fg-muted); line-height: 1.5; }

  /* ── Config blocks ── */
  .st-block { display: flex; flex-direction: column; background: var(--surface); border: 1px solid var(--border); border-radius: var(--r-card); box-shadow: inset 0 1px 0 color-mix(in oklab, white 4%, transparent), var(--shadow-sm); }
  .st-block-label { font-size: var(--fs-sm); font-weight: 650; color: var(--fg); padding: 13px 17px; border-bottom: 1px solid var(--border); }
  .config[data-disabled="true"] { opacity: 0.55; }

  .st-row { display: flex; flex-wrap: wrap; align-items: center; gap: 12px 16px; padding: 14px 17px; }
  .st-row + .st-row { border-top: 1px solid var(--border); }
  .st-row-body { flex: 1 1 300px; min-width: 0; }
  .st-row-label { font-size: var(--fs-md); font-weight: 600; color: var(--fg); display: block; }
  .st-row-desc { font-size: var(--fs-xs); color: var(--fg-muted); margin-top: 3px; line-height: 1.5; max-width: 60ch; }
  .st-row-desc code { font-family: var(--font-mono); color: var(--fg-2); background: color-mix(in oklab, var(--fg) 6%, transparent); padding: 0 4px; border-radius: 4px; font-size: 0.9em; }
  .st-row-ctl { flex: 0 1 auto; margin-left: auto; display: flex; align-items: center; gap: 10px; flex-wrap: wrap; justify-content: flex-end; }

  /* presets + detected-model chips share the chip-btn look */
  .preset-row, .model-list { flex-basis: 100%; display: flex; align-items: center; flex-wrap: wrap; gap: 7px; }
  .preset-lead { font-size: var(--fs-xs); color: var(--fg-subtle); margin-right: 2px; }
  .chip-btn { display: inline-flex; align-items: center; gap: 5px; height: 26px; padding: 0 10px; border-radius: 999px; font: inherit; font-size: var(--fs-xs); font-weight: 600; cursor: pointer; border: 1px solid var(--border); background: var(--field); color: var(--fg-2); font-family: var(--font-mono); transition: background 120ms, border-color 120ms, color 120ms; }
  .chip-btn:hover:not(:disabled) { background: var(--surface-hover); border-color: var(--border-strong); color: var(--fg); }
  .chip-btn.active { background: var(--ok-soft); border-color: color-mix(in oklch, var(--ok) 32%, transparent); color: var(--ok); }
  .chip-btn:disabled { opacity: 0.45; cursor: not-allowed; }

  /* ── Context-window guidance row ── */
  .ctx-row .st-row-label { display: inline-flex; align-items: center; gap: 6px; }
  .ctx-row.bad { background: var(--warn-soft); }
  .ctx-row.bad .st-row-label :global(svg) { color: var(--warn); }
  .ctx-result { flex-basis: 100%; margin-top: 8px; white-space: pre-wrap; word-break: break-word; color: var(--ok); font-size: var(--fs-xs); line-height: 1.5; background: var(--bg-inset, color-mix(in oklab, var(--fg) 5%, transparent)); border: 1px solid var(--border); border-radius: var(--radius); padding: 8px 11px; }
  .ctx-result.err { color: var(--danger); }

  /* Model card — family / params / quant tags from /api/show */
  .model-card { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 8px; }
  .mc-tag { font-size: var(--fs-2xs, 10.5px); font-weight: 600; color: var(--fg-2); background: var(--field); border: 1px solid var(--border); border-radius: 6px; padding: 2px 7px; }

  /* Optimize target selector */
  .ctx-targets { display: flex; align-items: center; flex-wrap: wrap; gap: 7px; margin-top: 10px; }

  /* Verify-head metrics cluster (tok/s + latency) */
  .rv-metrics { display: inline-flex; align-items: center; gap: 10px; }

  .st-note { padding: 10px 17px; font-size: var(--fs-xs); color: var(--fg-muted); }
  .st-detect-hint { color: var(--accent); font-weight: 600; margin-left: 2px; }

  .st-warn { display: block; font-size: var(--fs-xs); color: var(--warn); line-height: 1.5; padding: 10px 13px; background: var(--warn-soft); border: 1px solid color-mix(in oklab, var(--warn) 32%, transparent); border-radius: var(--r-card); }

  .st-switch { position: relative; width: 40px; height: 23px; border-radius: 999px; border: 0; padding: 0; background: var(--bg-elev-3); cursor: pointer; transition: background 160ms var(--ease-soft); flex: none; }
  .st-switch::after { content: ""; position: absolute; top: 3px; left: 3px; width: 17px; height: 17px; border-radius: 999px; background: var(--fg-muted); transition: transform 180ms var(--ease-page), background 160ms; }
  .st-switch.on { background: var(--accent); }
  .st-switch.on::after { transform: translateX(17px); background: var(--accent-fg); }
  .st-switch:disabled { opacity: 0.5; cursor: not-allowed; }
  .st-switch:focus-visible { outline: 0; box-shadow: 0 0 0 3px var(--ring); }

  .st-input { height: 32px; padding: 0 12px; border-radius: var(--radius); background: var(--field); border: 1px solid var(--field-border); color: var(--fg); font: inherit; font-size: var(--fs-sm); }
  .st-input:focus { outline: 0; border-color: var(--border-focus); box-shadow: 0 0 0 3px var(--ring); }
  .st-input.mono { font-family: var(--font-mono); }
  .st-secret { position: relative; display: inline-flex; align-items: center; }
  .st-secret .st-input { padding-right: 34px; }
  .st-eye { position: absolute; right: 5px; display: grid; place-items: center; width: 24px; height: 24px; border: 0; background: none; color: var(--fg-subtle); cursor: pointer; border-radius: 6px; }
  .st-eye:hover { color: var(--fg); background: var(--surface-hover); }

  .st-btn { display: inline-flex; align-items: center; gap: 7px; height: 32px; padding: 0 13px; border-radius: var(--radius); font: inherit; font-size: var(--fs-sm); font-weight: 600; cursor: pointer; border: 1px solid var(--border); background: var(--surface); color: var(--fg-2); transition: background 120ms, border-color 120ms, color 120ms; white-space: nowrap; }
  .st-btn:hover:not(:disabled) { background: var(--surface-hover); border-color: var(--border-strong); color: var(--fg); }
  .st-btn:disabled { opacity: 0.45; cursor: not-allowed; }
  .st-btn.primary { background: var(--accent); border-color: transparent; color: var(--accent-fg); box-shadow: 0 4px 14px -5px color-mix(in srgb, var(--accent) 60%, transparent); }
  .st-btn.primary:hover:not(:disabled) { background: var(--accent-hover); }
  .st-btn :global(svg) { color: currentColor; }
  .st-btn :global(.st-spin) { animation: st-spin 0.8s linear infinite; }
  @keyframes st-spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .st-btn :global(.st-spin) { animation: none; } }

  .st-pill { display: inline-flex; align-items: center; gap: 7px; height: 24px; padding: 0 10px; border-radius: 999px; font-size: var(--fs-xs); font-weight: 650; border: 1px solid var(--border); background: var(--surface); color: var(--fg-muted); }
  .st-pill .dot { width: 7px; height: 7px; border-radius: 999px; background: var(--fg-faint); }
  .st-pill.ok { background: var(--ok-soft); border-color: color-mix(in oklch, var(--ok) 28%, transparent); color: var(--ok); }
  .st-pill.ok .dot { background: var(--ok); }
  .st-pill.warn { background: var(--warn-soft); border-color: color-mix(in oklab, var(--warn) 28%, transparent); color: var(--warn); }
  .st-pill.warn .dot { background: var(--warn); }
</style>

<script lang="ts">
  import { onMount } from "svelte";
  import { Cpu, FlaskConical, Loader2, Eye, EyeOff, RefreshCw } from "lucide-svelte";
  import { localLlm } from "../../state/localLlm.svelte";

  let keyInput = $state("");
  let keyVisible = $state(false);

  let keySaving = $state(false);
  let testing = $state(false);
  let testResult = $state<{ ok: boolean; msg: string } | null>(null);
  let modelHint = $state("");
  // Test needs a target — gate the button so an empty probe can't fire.
  const canTest = $derived(
    localLlm.enabled && localLlm.baseUrl.trim().length > 0 && localLlm.model.trim().length > 0,
  );

  onMount(() => { void localLlm.refresh(); });

  async function toggleEnabled() {
    // Store owns the optimistic flip + mid-conversation reset; swallow the
    // revert-throw here so the switch just snaps back on failure.
    try { await localLlm.setEnabled(!localLlm.enabled); } catch { /* reverted in store */ }
  }

  async function saveModel() {
    try { await localLlm.saveModel(); }
    catch (e) { testResult = { ok: false, msg: String(e) }; }
  }

  async function saveKey() {
    keySaving = true;
    try {
      await localLlm.saveKey(keyInput);
      keyInput = "";
    } catch (e) {
      console.error("set key failed", e);
    } finally {
      keySaving = false;
    }
  }

  async function clearKey() {
    keySaving = true;
    try {
      await localLlm.saveKey(null);
      keyInput = "";
    } catch (e) {
      console.error("clear key failed", e);
    } finally {
      keySaving = false;
    }
  }

  async function detectModels() {
    modelHint = "";
    const n = await localLlm.listModels();
    modelHint = n === 0 ? "No models found at this endpoint." : `${n} model${n === 1 ? "" : "s"} found.`;
  }

  async function testConnection() {
    testing = true;
    testResult = null;
    testResult = await localLlm.test();
    testing = false;
  }
</script>

<div class="sb-main">
  <!-- ── Hero ── -->
  <div class="sb-topbar">
    <div class="sb-hero">
      <div class="sb-hero-l">
        <div class="sb-hero-ic"><Cpu size={22} strokeWidth={1.75} /></div>
        <div class="sb-hero-tx">
          <div class="sb-eyebrow">Experimental</div>
          <div class="sb-hero-tt">Local LLM</div>
          <div class="sb-hero-sub">
            Route turns through a local Anthropic-compatible endpoint (LiteLLM / Ollama)
            instead of your Claude session. Toggle off any time to return to normal Claude.
          </div>
        </div>
      </div>
      <div class="sb-hero-r">
        <span class="sb-chip {localLlm.enabled ? 'ok' : ''}">
          <span class="dot"></span>{localLlm.enabled ? "Local mode on" : "Off"}
        </span>
      </div>
    </div>
  </div>

  <!-- ── Scrolling canvas ── -->
  <div class="sb-scroll">
    <div class="sb-wrap">
      <div class="sb-bento">

        <!-- Mode -->
        <div class="st-block blk-mode">
          <div class="st-block-label">Mode</div>
          <div class="st-card">
            <div class="st-row">
              <div class="st-row-body">
                <div class="st-row-label">Local LLM mode</div>
                <div class="st-row-desc">When on, every turn spawns the CLI against your local endpoint with <code>--bare</code>.</div>
              </div>
              <div class="st-row-ctl">
                <button
                  class="st-switch" class:on={localLlm.enabled}
                  role="switch" aria-checked={localLlm.enabled} aria-label="Toggle local LLM mode"
                  disabled={!localLlm.loaded} type="button" onclick={toggleEnabled}
                ></button>
              </div>
            </div>
            <div class="st-warn">
              Local mode bypasses cloud auth, effort tiers, and per-conversation model pinning.
              Tool-calling fidelity depends on the local model — small models may struggle with
              Rift's MCP tools.
            </div>
          </div>
        </div>

        <!-- Endpoint -->
        <div class="st-block blk-endpoint">
          <div class="st-block-label">Endpoint</div>
          <div class="st-card">
            <div class="st-row" data-disabled={!localLlm.enabled}>
              <div class="st-row-body">
                <label class="st-row-label" for="ll-base">Base URL</label>
                <div class="st-row-desc">Must speak the Anthropic <code>/v1/messages</code> API — a <strong>LiteLLM proxy</strong> (default <code>:4000</code>), <em>not</em> raw Ollama (<code>:11434</code>). Sets <code>ANTHROPIC_BASE_URL</code>.</div>
              </div>
              <div class="st-row-ctl">
                <input
                  id="ll-base" class="st-input mono" type="text"
                  bind:value={localLlm.baseUrl} onblur={() => localLlm.saveBaseUrl()} disabled={!localLlm.enabled}
                  placeholder="http://localhost:4000" style="width:220px;"
                  spellcheck="false" autocapitalize="off" autocomplete="off"
                />
              </div>
            </div>

            <div class="st-row" data-disabled={!localLlm.enabled}>
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
            </div>

            <div class="st-row" data-disabled={!localLlm.enabled}>
              <div class="st-row-body">
                <label class="st-row-label" for="ll-key">API key</label>
                <div class="st-row-desc">Stored in the OS keychain, never in config. Most local proxies accept any non-empty value. Sets <code>ANTHROPIC_API_KEY</code>.</div>
              </div>
              <div class="st-row-ctl">
                {#if localLlm.hasKey}
                  <span class="st-pill ok"><span class="dot"></span>Configured</span>
                  <button class="st-btn" type="button" disabled={!localLlm.enabled || keySaving} onclick={clearKey}>Clear</button>
                {:else}
                  <span class="st-secret">
                    <input
                      id="ll-key" class="st-input mono" type={keyVisible ? "text" : "password"}
                      bind:value={keyInput} disabled={!localLlm.enabled}
                      placeholder="any non-empty value" style="width:188px;"
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
          </div>
        </div>

        <!-- Connection -->
        <div class="st-block blk-conn">
          <div class="st-block-label">Connection</div>
          <div class="st-card">
            <div class="st-row" data-disabled={!localLlm.enabled}>
              <div class="st-row-body">
                <div class="st-row-label">Test connection</div>
                <div class="st-row-desc">Round-trips a one-line prompt through your endpoint and reports the reply.</div>
              </div>
              <div class="st-row-ctl">
                {#if testResult}
                  <span class="st-pill {testResult.ok ? 'ok' : 'warn'}"><span class="dot"></span>{testResult.ok ? "OK" : "Failed"}</span>
                {/if}
                <button class="st-btn primary" type="button" disabled={!canTest || testing} onclick={testConnection}>
                  {#if testing}<Loader2 size={14} class="st-spin" /> Testing…{:else}<FlaskConical size={14} /> Test{/if}
                </button>
              </div>
            </div>
            {#if testResult}
              <div class="st-note ll-result" class:err={!testResult.ok}>{testResult.msg}</div>
            {/if}
          </div>
        </div>

      </div>
    </div>
  </div>
</div>

<style>
  /* Chrome replicated from SettingsPage (Svelte-scoped there) so this page
     mocks the same surface — all values are global CSS tokens. */
  .sb-main { position: relative; overflow: hidden; display: flex; flex-direction: column; flex: 1; min-height: 0; min-width: 0; background: var(--bg); color: var(--fg); }

  .sb-topbar { flex: none; padding: 26px 40px 0; background: linear-gradient(180deg, color-mix(in oklab, var(--accent) 3%, var(--bg)), var(--bg) 120px); border-bottom: 1px solid var(--border); }
  .sb-hero { display: flex; align-items: flex-start; justify-content: space-between; gap: 24px; max-width: 820px; margin: 0 auto; padding-bottom: 26px; }
  .sb-hero-l { display: flex; align-items: center; gap: 14px; min-width: 0; }
  .sb-hero-ic { width: 44px; height: 44px; border-radius: 12px; flex: none; display: grid; place-items: center; background: var(--accent-soft); color: var(--accent); box-shadow: inset 0 0 0 1px var(--ghost-border); }
  .sb-hero-ic :global(svg) { color: var(--accent); }
  .sb-eyebrow { font-size: 10px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--fg-subtle); }
  .sb-hero-tt { font-size: 24px; font-weight: 760; letter-spacing: -0.025em; line-height: 1.1; margin-top: 1px; }
  .sb-hero-sub { font-size: var(--fs-sm); color: var(--fg-muted); margin-top: 3px; line-height: 1.45; max-width: 64ch; }
  .sb-hero-r { display: flex; align-items: center; gap: 8px; flex: none; }
  .sb-chip { display: inline-flex; align-items: center; gap: 7px; height: 30px; padding: 0 12px; border-radius: 999px; background: var(--surface); border: 1px solid var(--border); color: var(--fg-2); font: inherit; font-size: var(--fs-xs); font-weight: 600; }
  .sb-chip .dot { width: 7px; height: 7px; border-radius: 999px; background: var(--fg-faint); }
  .sb-chip.ok { background: var(--ok-soft); border-color: color-mix(in oklch, var(--ok) 28%, transparent); color: var(--ok); }
  .sb-chip.ok .dot { background: var(--ok); }

  /* overflow:auto is a safety net only — the bento is sized to fit the app's
     800px min window height with no scroll, so the bar never actually shows. */
  .sb-scroll { flex: 1; min-width: 0; min-height: 0; overflow-y: auto; overflow-x: hidden; scroll-behavior: smooth; }
  .sb-wrap { max-width: 880px; margin: 0 auto; padding: 22px 40px 28px; }
  /* Bento: Endpoint (the input-heavy card) takes the taller right column;
     Mode + Connection stack on the left so the two columns balance in height. */
  .sb-bento {
    display: grid;
    grid-template-columns: minmax(0, 0.9fr) minmax(0, 1.1fr);
    grid-template-areas: "mode endpoint" "conn endpoint";
    gap: 16px;
    align-items: start;
  }
  .sb-bento > .st-block { min-width: 0; }
  .blk-mode { grid-area: mode; }
  .blk-endpoint { grid-area: endpoint; }
  .blk-conn { grid-area: conn; }
  /* Mode + Connection have small controls (toggle, Test) — let the body shrink
     so they sit inline in the narrow left column instead of wrapping below. The
     Endpoint card keeps the default 300px basis so its inputs wrap cleanly. */
  .blk-mode .st-row-body, .blk-conn .st-row-body { flex: 1 1 0; }
  /* Narrow fallback (below the app min-width, e.g. detached/resized): single
     column, natural order. The safety-net scrollbar covers any overflow here. */
  @media (max-width: 760px) {
    .sb-bento { grid-template-columns: 1fr; grid-template-areas: "mode" "endpoint" "conn"; }
  }

  .st-block { display: flex; flex-direction: column; background: var(--surface); border: 1px solid var(--border); border-radius: var(--r-card); box-shadow: inset 0 1px 0 color-mix(in oklab, white 4%, transparent), var(--shadow-sm); }
  .st-block-label { font-size: var(--fs-sm); font-weight: 650; color: var(--fg); padding: 13px 17px; border-bottom: 1px solid var(--border); }
  .st-card { background: transparent; border: 0; border-radius: 0; }
  .st-card > .st-warn { margin: 4px 17px 14px; }

  .st-row { display: flex; flex-wrap: wrap; align-items: center; gap: 12px 16px; padding: 14px 17px; }
  .st-row + .st-row, .st-row + .st-note { border-top: 1px solid var(--border); }
  .st-row-body { flex: 1 1 300px; min-width: 0; }
  .st-row-label { font-size: var(--fs-md); font-weight: 600; color: var(--fg); display: block; }
  .st-row-desc { font-size: var(--fs-xs); color: var(--fg-muted); margin-top: 3px; line-height: 1.5; max-width: 60ch; }
  .st-row-desc code { font-family: var(--font-mono); color: var(--fg-2); background: color-mix(in oklab, var(--fg) 6%, transparent); padding: 0 4px; border-radius: 4px; font-size: 0.9em; }
  .st-row-ctl { flex: 0 1 auto; margin-left: auto; display: flex; align-items: center; gap: 10px; flex-wrap: wrap; justify-content: flex-end; }
  .st-row[data-disabled="true"] { opacity: 0.55; }

  .st-note { padding: 10px 17px; font-size: var(--fs-xs); color: var(--fg-muted); border-top: 1px solid var(--border); }
  .ll-result { font-family: var(--font-mono); white-space: pre-wrap; word-break: break-word; color: var(--ok); }
  .ll-result.err { color: var(--danger, #e66); }
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

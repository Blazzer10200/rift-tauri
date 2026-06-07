<script lang="ts">
  // Edit-swarm panel (idea-phase-plan §2 Phase 3b) — Harness sub-tab. Feeds a
  // confirmed-findings list into worktree-isolated, gate-verified,
  // diff-reviewed edits that cherry-pick to main only if they pass. The harness
  // + safety model live in docs/design/edit-swarm-safety-layer.md. Reuses the
  // Harness bento language; one viewport, no scroll-cram.
  import { onMount, onDestroy } from "svelte";
  import { Boxes, GitBranch, CheckCircle2, XCircle, MinusCircle, Play, RotateCw, ChevronRight } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { swarm, type Finding } from "../../state/swarm.svelte";

  const DEMO = `[
  {
    "file": "src/lib/example.ts",
    "line": 42,
    "evidence": "unused import 'foo' flagged by the audit swarm",
    "suggestedFix": "remove the unused 'foo' import"
  }
]`;

  let findingsText = $state("");
  let parseError = $state<string | null>(null);
  let expanded = $state<Record<string, boolean>>({});

  const root = $derived(assistant.workspace.current);
  const env = $derived(swarm.env);
  const ready = $derived(
    !!root && !!env && env.isGitRepo && env.cleanTree && env.claudePresent && !swarm.running,
  );

  function parseFindings(): Finding[] | null {
    parseError = null;
    const raw = findingsText.trim();
    if (!raw) { parseError = "paste a findings list first"; return null; }
    let v: unknown;
    try { v = JSON.parse(raw); } catch (e) { parseError = "invalid JSON: " + String(e); return null; }
    if (!Array.isArray(v) || v.length === 0) { parseError = "expected a non-empty JSON array"; return null; }
    for (const f of v as Record<string, unknown>[]) {
      if (typeof f?.file !== "string" || typeof f?.evidence !== "string" || typeof f?.suggestedFix !== "string") {
        parseError = "each finding needs string file, evidence, suggestedFix";
        return null;
      }
    }
    return v as Finding[];
  }

  async function refreshEnv() {
    if (root) await swarm.checkEnv(root);
  }

  async function runSwarm() {
    const findings = parseFindings();
    if (!findings || !root) return;
    await swarm.run(root, findings);
    await refreshEnv(); // reflect the post-run tree state
  }

  function gateIcon(v: string) { return v === "pass" ? CheckCircle2 : v === "fail" ? XCircle : MinusCircle; }
  function verdictClass(v: string) {
    return v === "pass" || v === "accept" ? "ok" : v === "fail" || v === "reject" ? "bad" : "muted";
  }

  onMount(() => { swarm.attach(); refreshEnv(); });
  onDestroy(() => swarm.detach());
</script>

<div class="swarm">
  <header class="shead">
    <div class="shead-l">
      <div class="shead-title">Edit swarm <span class="shead-spark"><Boxes size={15} /></span></div>
      <div class="shead-sub">Worktree-isolated fixes · verify-gated · diff-reviewed before merge to main</div>
    </div>
    <button class="icon-btn" type="button" onclick={refreshEnv} title="Re-check environment" aria-label="Re-check environment">
      <RotateCw size={15} />
    </button>
  </header>

  <!-- preflight chips -->
  <div class="chips">
    {#if !root}
      <span class="chip bad">No workspace open</span>
    {:else}
      <span class="chip" class:ok={env?.isGitRepo} class:bad={env && !env.isGitRepo}><GitBranch size={12} /> {env?.isGitRepo ? "git repo" : "not a repo"}</span>
      <span class="chip" class:ok={env?.cleanTree} class:bad={env && !env.cleanTree}>{env?.cleanTree ? "clean tree" : "dirty tree"}</span>
      <span class="chip" class:ok={env?.claudePresent} class:bad={env && !env.claudePresent}>{env?.claudePresent ? "claude CLI" : "no CLI"}</span>
      <span class="chip" class:ok={env?.nodeModulesPresent} class:muted={env && !env.nodeModulesPresent}>{env?.nodeModulesPresent ? "node_modules" : "no node_modules"}</span>
      {#if env?.headShort}<span class="chip muted">HEAD {env.headShort}</span>{/if}
    {/if}
  </div>

  <div class="bento">
    <!-- findings input -->
    <section class="card input-card">
      <div class="card-head">
        <span class="card-title">Confirmed findings</span>
        <button class="link-btn" type="button" onclick={() => (findingsText = DEMO)}>load template</button>
      </div>
      <textarea
        class="findings"
        bind:value={findingsText}
        spellcheck="false"
        placeholder={'[{"file":"src/x.ts","line":12,"evidence":"…","suggestedFix":"…"}]'}
      ></textarea>
      {#if parseError}<div class="parse-err">{parseError}</div>{/if}
      <div class="run-row">
        <button class="run-btn" type="button" disabled={!ready} onclick={runSwarm}>
          {#if swarm.running}<RotateCw size={14} class="spin" /> running…{:else}<Play size={14} /> Run swarm{/if}
        </button>
        {#if !ready && root && env && !swarm.running}
          <span class="hint">{!env.isGitRepo ? "needs a git repo" : !env.cleanTree ? "commit/stash first" : !env.claudePresent ? "claude CLI not found" : ""}</span>
        {/if}
      </div>
    </section>

    <!-- results -->
    <section class="card results-card">
      <div class="card-head">
        <span class="card-title">Agents</span>
        {#if swarm.report}
          <span class="summary" class:ok={swarm.report.mainTreeIntact} class:bad={!swarm.report.mainTreeIntact}>
            {swarm.report.mergedCount}/{swarm.report.agents.length} merged · main tree {swarm.report.mainTreeIntact ? "intact" : "CHECK"}
          </span>
        {/if}
      </div>

      {#if swarm.error}
        <div class="parse-err">{swarm.error}</div>
      {/if}

      {#if !swarm.report && !swarm.running && Object.keys(swarm.live).length === 0}
        <div class="empty">Paste a findings list and run — one agent per file, isolated in its own worktree.</div>
      {/if}

      <!-- live stages while running -->
      {#if swarm.running}
        {#each Object.entries(swarm.live) as [agent, stage] (agent)}
          <div class="agent live">
            <div class="agent-top"><span class="agent-name">{agent}</span><span class="stage">{stage}</span></div>
          </div>
        {/each}
        {#if Object.keys(swarm.live).length === 0}
          <div class="empty">spinning up worktrees…</div>
        {/if}
      {/if}

      <!-- final per-agent cards -->
      {#if swarm.report}
        {#each swarm.report.agents as a (a.agent)}
          <div class="agent">
            <button class="agent-top" type="button" onclick={() => (expanded = { ...expanded, [a.agent]: !expanded[a.agent] })}>
              <span class="caret" class:open={expanded[a.agent]}><ChevronRight size={13} /></span>
              <span class="agent-name">{a.agent}</span>
              <span class="agent-file">{a.file}</span>
              <span class="badges">
                {#if a.gate !== "n/a"}
                  {@const GI = gateIcon(a.gate)}
                  <span class="badge {verdictClass(a.gate)}"><GI size={12} /> gate {a.gate}</span>
                {/if}
                {#if a.review !== "n/a"}<span class="badge {verdictClass(a.review)}">review {a.review}</span>{/if}
                {#if a.merged}<span class="badge ok">merged</span>{:else}<span class="badge muted">discarded</span>{/if}
              </span>
            </button>
            {#if expanded[a.agent]}
              <div class="agent-body">
                <div class="kv"><span>findings</span><span>{a.findings}</span></div>
                <div class="kv"><span>last stage</span><span>{a.stage}</span></div>
                {#if a.gateDetail}<div class="kv"><span>gate</span><span class="mono">{a.gateDetail}</span></div>{/if}
                {#if a.reviewDetail}<div class="kv"><span>review</span><span>{a.reviewDetail}</span></div>{/if}
                {#if a.error}<div class="kv"><span>note</span><span class="bad">{a.error}</span></div>{/if}
                {#if a.diff}<pre class="diff">{a.diff}</pre>{/if}
              </div>
            {/if}
          </div>
        {/each}
      {/if}
    </section>
  </div>
</div>

<style>
  .swarm { display: flex; flex-direction: column; gap: 12px; padding: 14px 22px 18px; height: 100%; min-height: 0; overflow: hidden; }
  .shead { display: flex; align-items: flex-start; justify-content: space-between; flex: none; }
  .shead-title { display: inline-flex; align-items: center; gap: 8px; font-size: var(--fs-lg); font-weight: 750; color: var(--fg); }
  .shead-spark :global(svg) { color: var(--accent); }
  .shead-sub { margin-top: 2px; font-size: var(--fs-xs); color: var(--fg-muted); }
  .icon-btn { display: inline-flex; align-items: center; justify-content: center; width: 32px; height: 32px; border-radius: 999px; border: 1px solid var(--border); background: var(--surface); color: var(--fg-muted); cursor: pointer; }
  .icon-btn:hover { color: var(--fg); border-color: var(--border-strong); }

  .chips { display: flex; flex-wrap: wrap; gap: 6px; flex: none; }
  .chip { display: inline-flex; align-items: center; gap: 5px; height: 24px; padding: 0 10px; border-radius: 999px; border: 1px solid var(--border); background: var(--bg-inset); font-size: var(--fs-2xs); font-weight: 600; color: var(--fg-muted); }
  .chip.ok { border-color: color-mix(in oklab, var(--ok) 45%, transparent); color: var(--ok); }
  .chip.bad { border-color: color-mix(in oklab, var(--danger) 45%, transparent); color: var(--danger); }
  .chip.muted { opacity: 0.7; }

  .bento { display: grid; grid-template-columns: minmax(280px, 0.85fr) 1.15fr; gap: 12px; flex: 1; min-height: 0; }
  .card { display: flex; flex-direction: column; min-height: 0; gap: 8px; padding: 14px; border-radius: 14px; border: 1px solid var(--border); background: var(--surface); }
  .card-head { display: flex; align-items: center; justify-content: space-between; flex: none; }
  .card-title { font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.02em; text-transform: uppercase; color: var(--fg-muted); }
  .link-btn, .summary { font-size: var(--fs-2xs); }
  .link-btn { background: none; border: none; color: var(--accent); cursor: pointer; font-weight: 600; }
  .summary.ok { color: var(--ok); }
  .summary.bad { color: var(--danger); }

  .input-card .findings { flex: 1; min-height: 160px; resize: none; border-radius: 10px; border: 1px solid var(--border); background: var(--bg-inset); color: var(--fg); padding: 10px; font-family: var(--font-mono); font-size: var(--fs-2xs); line-height: 1.5; }
  .findings::placeholder { color: var(--fg-faint); }
  .parse-err { font-size: var(--fs-2xs); color: var(--danger); }
  .run-row { display: flex; align-items: center; gap: 10px; flex: none; }
  .run-btn { display: inline-flex; align-items: center; gap: 7px; height: 34px; padding: 0 16px; border-radius: 999px; border: 1px solid var(--ghost-border); background: var(--accent-soft); color: var(--accent); font-weight: 700; font-size: var(--fs-xs); cursor: pointer; }
  .run-btn:disabled { opacity: 0.45; cursor: not-allowed; }
  .hint { font-size: var(--fs-2xs); color: var(--fg-muted); }

  .results-card { overflow-y: auto; }
  .empty { font-size: var(--fs-xs); color: var(--fg-muted); padding: 16px 4px; }
  .agent { border: 1px solid var(--border); border-radius: 10px; background: var(--bg-inset); overflow: hidden; }
  .agent.live { padding: 8px 12px; }
  .agent-top { display: flex; align-items: center; gap: 8px; width: 100%; padding: 9px 12px; background: none; border: none; color: var(--fg); cursor: pointer; text-align: left; font-size: var(--fs-xs); }
  .caret { display: inline-flex; transition: transform var(--dur-fast) var(--ease-soft); color: var(--fg-muted); }
  .caret.open { transform: rotate(90deg); }
  .agent-name { font-weight: 700; }
  .agent-file { color: var(--fg-muted); font-family: var(--font-mono); font-size: var(--fs-2xs); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; }
  .stage { color: var(--accent); font-size: var(--fs-2xs); }
  .badges { display: inline-flex; gap: 5px; flex: none; }
  .badge { display: inline-flex; align-items: center; gap: 4px; height: 20px; padding: 0 8px; border-radius: 999px; font-size: var(--fs-2xs); font-weight: 650; border: 1px solid var(--border); color: var(--fg-muted); }
  .badge.ok { color: var(--ok); border-color: color-mix(in oklab, var(--ok) 40%, transparent); }
  .badge.bad { color: var(--danger); border-color: color-mix(in oklab, var(--danger) 40%, transparent); }
  .badge.muted { opacity: 0.7; }
  .agent-body { padding: 4px 12px 12px 12px; display: flex; flex-direction: column; gap: 5px; }
  .kv { display: flex; gap: 10px; font-size: var(--fs-2xs); }
  .kv > span:first-child { color: var(--fg-muted); min-width: 72px; }
  .kv .bad { color: var(--danger); }
  .mono { font-family: var(--font-mono); }
  .diff { margin: 4px 0 0; max-height: 240px; overflow: auto; padding: 8px; border-radius: 8px; background: var(--bg); border: 1px solid var(--border); font-family: var(--font-mono); font-size: var(--fs-2xs); line-height: 1.45; white-space: pre; color: var(--fg-muted); }
  :global(.spin) { animation: spin 0.9s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>

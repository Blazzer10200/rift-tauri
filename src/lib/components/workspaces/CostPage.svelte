<script lang="ts">
  // Cost cockpit (idea-phase-plan §1e) — Harness sub-tab. Cross-session credit
  // pool: a fuel gauge whose arc depletes as the current reset window burns
  // budget, plus daily / per-model / per-workspace rollups read straight off the
  // SQLite usage store. Reuses the Harness bento/KPI visual language.
  import { onMount } from "svelte";
  import { RotateCw, Gauge as GaugeIcon } from "lucide-svelte";
  import { usage, type BudgetPlan, type BudgetCadence } from "../../state/usage.svelte";
  import { tooltip } from "$lib/actions/tooltip";

  // ── formatters (local — mirrors HarnessPage's own set) ──
  function fmtUsd(n: number | null | undefined): string {
    if (n == null) return "—";
    return "$" + (n < 1 ? n.toFixed(3) : n < 100 ? n.toFixed(2) : n.toFixed(0));
  }
  function fmtTok(n: number | null | undefined): string {
    if (n == null) return "—";
    if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + "M";
    if (n >= 1_000) return (n / 1_000).toFixed(1) + "K";
    return String(n);
  }
  function fmtDate(ts: number | null | undefined): string {
    if (ts == null) return "—";
    const d = new Date(ts);
    if (isNaN(d.getTime())) return "—";
    return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  }
  function fmtDay(iso: string): string {
    // "2026-06-07" → "Jun 7"
    const d = new Date(iso + "T00:00:00");
    return isNaN(d.getTime()) ? iso : d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  }
  function fmtDays(n: number | null | undefined): string {
    if (n == null) return "—";
    if (n >= 100) return Math.round(n) + "d";
    if (n >= 10) return n.toFixed(0) + "d";
    return n.toFixed(1) + "d";
  }
  function cleanPath(p: string | null | undefined): string {
    return p ? p.replace(/^\\\\\?\\/, "").split(/[\\/]/).pop() || p : "—";
  }
  function modelHue(name: string): number {
    if (name.includes("haiku")) return 175;
    if (name.includes("opus")) return 280;
    if (name.includes("sonnet")) return 225;
    return 163;
  }
  function shortModel(id: string): string {
    return id.replace(/^claude-/, "").replace(/-(\d)-(\d)$/, " $1.$2");
  }

  const PLANS: { id: BudgetPlan; label: string; sub: string }[] = [
    { id: "pro", label: "Pro", sub: "$20" },
    { id: "max5x", label: "Max 5×", sub: "$100" },
    { id: "max20x", label: "Max 20×", sub: "$200" },
    { id: "custom", label: "Custom", sub: "set" },
  ];
  const CADENCES: { id: BudgetCadence; label: string }[] = [
    { id: "monthly", label: "Monthly" },
    { id: "weekly", label: "Weekly" },
    { id: "daily", label: "Daily" },
  ];

  const b = $derived(usage.budget);
  // Fuel = % of pool remaining. Zone thresholds mirror the context ring's feel.
  const pct = $derived(b ? Math.max(0, Math.min(100, b.pctRemaining)) : 100);
  const zone = $derived(pct > 50 ? "ok" : pct > 20 ? "warn" : "hot");
  const dryText = $derived.by(() => {
    if (!b) return "—";
    if (b.projectedExhaustionDate != null) return `dry-out ~ ${fmtDate(b.projectedExhaustionDate)}`;
    if (b.limit > 0 && b.spent >= b.limit) return "over budget";
    return "on track";
  });

  // Gauge geometry (matches HarnessPage's ring).
  const R = 82, C = 2 * Math.PI * 82;
  const fillDash = $derived(C * (1 - pct / 100));

  // ── daily / model / workspace folds for the bars ──
  const dailyMax = $derived(Math.max(0.0001, ...usage.daily.map((d) => d.cost)));
  const modelMax = $derived(Math.max(0.0001, ...usage.byModel.map((m) => m.cost)));
  const wsMax = $derived(Math.max(0.0001, ...usage.byWorkspace.map((w) => w.cost)));
  const hasData = $derived(usage.daily.length > 0 || usage.byModel.length > 0);

  function setPlan(plan: BudgetPlan) {
    void usage.setBudget({ ...usage.config, plan });
  }
  function setCadence(cadence: BudgetCadence) {
    void usage.setBudget({ ...usage.config, cadence });
  }
  let customLimit = $state("");
  function commitCustom() {
    const v = parseFloat(customLimit);
    if (!isNaN(v) && v > 0) void usage.setBudget({ ...usage.config, plan: "custom", customLimitUsd: v });
  }

  onMount(() => {
    void usage.refresh();
    customLimit = usage.config.customLimitUsd != null ? String(usage.config.customLimitUsd) : "";
  });
</script>

<div class="cost">
  <header class="chead">
    <div class="chead-l">
      <div class="chead-title">Cost cockpit <span class="chead-spark"><GaugeIcon size={15} /></span></div>
      <div class="chead-sub">
        Credit pool across all sessions · {usage.allTimeTurns} turns logged · {fmtUsd(usage.allTimeCost)} all-time
      </div>
    </div>
    <button class="crefresh" type="button" onclick={() => usage.refresh()} use:tooltip={"Refresh"}>
      <RotateCw size={13} class={usage.loading ? "spin" : ""} />
    </button>
  </header>

  {#if usage.loaded && !hasData}
    <div class="empty-hero">
      <GaugeIcon size={30} strokeWidth={1.6} />
      <div class="empty-hero-t">No usage logged yet</div>
      <div class="empty-hero-s">Send a few turns — each one lands in the durable store and the cockpit fills in here.</div>
    </div>
  {:else}
    <div class="bento">
      <!-- HERO: fuel gauge -->
      <section class="cell hero" data-zone={zone}>
        <div class="hero-glow"></div>
        <div class="hero-tag">credit pool · {b?.cadence ?? "—"}</div>
        <div class="gauge">
          <svg viewBox="0 0 200 200" class="gauge-svg">
            <defs>
              <linearGradient id="fuelgrad" x1="0" y1="0" x2="1" y2="1">
                <stop offset="0%" class="g0" />
                <stop offset="100%" class="g1" />
              </linearGradient>
            </defs>
            <circle cx="100" cy="100" r={R} class="gauge-track" />
            <circle cx="100" cy="100" r={R} class="gauge-fill" data-zone={zone}
              stroke="url(#fuelgrad)"
              stroke-dasharray={C} stroke-dashoffset={fillDash}
              transform="rotate(-90 100 100)" />
          </svg>
          <div class="gauge-mid">
            <div class="gauge-pct" data-zone={zone}>{pct.toFixed(0)}<span class="gauge-pct-u">%</span></div>
            <div class="gauge-cap">pool left</div>
          </div>
        </div>
        <div class="hero-foot">
          <div class="hero-spent">{fmtUsd(b?.spent)} <span class="dim">/ {fmtUsd(b?.limit)} this {b?.cadence === "monthly" ? "month" : b?.cadence === "weekly" ? "week" : "day"}</span></div>
          <div class="hero-zone" data-zone={zone}>{dryText}</div>
        </div>
      </section>

      <!-- Budget plan picker -->
      <section class="cell wide">
        <div class="cell-head"><span class="cell-title">Budget</span><span class="cell-meta">pick your plan</span></div>
        <div class="plan-grid">
          {#each PLANS as p (p.id)}
            <button class="plan" class:on={usage.config.plan === p.id} type="button" onclick={() => setPlan(p.id)}>
              <span class="plan-l">{p.label}</span>
              <span class="plan-sub">{p.sub}</span>
            </button>
          {/each}
        </div>
        {#if usage.config.plan === "custom"}
          <div class="custom-row">
            <span class="custom-lbl">$ limit</span>
            <input class="custom-in mono" type="number" min="1" step="1" bind:value={customLimit}
              placeholder="e.g. 150" onchange={commitCustom} onblur={commitCustom} />
          </div>
        {/if}
        <div class="cad">
          {#each CADENCES as c (c.id)}
            <button class="cad-btn" class:on={usage.config.cadence === c.id} type="button" onclick={() => setCadence(c.id)}>{c.label}</button>
          {/each}
        </div>
      </section>

      <!-- KPI rail -->
      <section class="cell full kpi-rail">
        <div class="kpi"><span class="kpi-v">{fmtUsd(b?.spent)}</span><span class="kpi-k">spent this {b?.cadence === "monthly" ? "mo" : b?.cadence === "weekly" ? "wk" : "day"}</span></div>
        <div class="kpi"><span class="kpi-v">{fmtUsd(b?.burnPerDay)}<span class="kpi-u"> /day</span></span><span class="kpi-k">burn rate</span></div>
        <div class="kpi"><span class="kpi-v" class:nodata={b?.daysRemaining == null}>{fmtDays(b?.daysRemaining)}</span><span class="kpi-k">runway left</span></div>
        <div class="kpi"><span class="kpi-v">{fmtUsd(usage.allTimeCost)}</span><span class="kpi-k">all-time cost</span></div>
        <div class="kpi"><span class="kpi-v">{usage.allTimeTurns}</span><span class="kpi-k">all-time turns</span></div>
        <div class="kpi"><span class="kpi-v" class:nodata={b?.projectedExhaustionDate == null}>{b?.projectedExhaustionDate != null ? fmtDate(b.projectedExhaustionDate) : "—"}</span><span class="kpi-k">projected dry-out</span></div>
      </section>

      <!-- Insights — "Rift noticed…" (2b, observational only) -->
      {#if usage.insights.length > 0}
        <section class="cell full insights">
          <div class="cell-head">
            <span class="cell-title">Rift noticed…</span>
            <span class="cell-meta">{usage.insights.length} pattern{usage.insights.length === 1 ? "" : "s"}</span>
          </div>
          <div class="ins-grid">
            {#each usage.insights as ins (ins.id)}
              <div class="ins" data-sev={ins.severity}>
                <span class="ins-dot"></span>
                <div class="ins-body">
                  <div class="ins-title">{ins.title}</div>
                  <div class="ins-detail">{ins.detail}</div>
                </div>
              </div>
            {/each}
          </div>
        </section>
      {/if}

      <!-- Daily spend (last 30d) -->
      <section class="cell full tl">
        <div class="cell-head">
          <span class="cell-title">Daily spend</span>
          <span class="cell-meta">{usage.daily.length} days · bar = cost</span>
        </div>
        {#if usage.daily.length === 0}
          <div class="empty tl-empty">No daily activity yet.</div>
        {:else}
          <div class="tl-track">
            <span class="tl-grid"></span>
            {#each usage.daily as d (d.date)}
              <span class="tl-bar"
                style="height:{Math.max(6, (d.cost / dailyMax) * 100)}%"
                use:tooltip={`${fmtDay(d.date)} · ${fmtUsd(d.cost)}\n${d.turns} turns · ${fmtTok(d.input + d.output + d.cacheRead + d.cacheWrite)} tok`}></span>
            {/each}
          </div>
          <div class="tl-axis"><span>{usage.daily.length ? fmtDay(usage.daily[0].date) : ""}</span><span>today →</span></div>
        {/if}
      </section>

      <!-- By model -->
      <section class="cell wide">
        <div class="cell-head"><span class="cell-title">By model</span><span class="cell-meta">cost share</span></div>
        {#if usage.byModel.length === 0}
          <div class="empty">No turns yet.</div>
        {:else}
          <div class="bars">
            {#each usage.byModel.slice(0, 6) as m (m.modelId)}
              <div class="bar-row">
                <span class="bar-k mono" style="--mH:{modelHue(m.modelId)}"><span class="mdl-dot"></span>{shortModel(m.modelId)}{#if !m.priced}<span class="est">est</span>{/if}</span>
                <span class="bar-track"><span class="bar-fill" style="width:{Math.max(2, (m.cost / modelMax) * 100)}%; --barH:{modelHue(m.modelId)}"></span></span>
                <span class="bar-v mono">{fmtUsd(m.cost)}</span>
              </div>
            {/each}
          </div>
        {/if}
      </section>

      <!-- By workspace -->
      <section class="cell wide">
        <div class="cell-head"><span class="cell-title">By workspace</span><span class="cell-meta">cost share</span></div>
        {#if usage.byWorkspace.length === 0}
          <div class="empty">No workspaces yet.</div>
        {:else}
          <div class="bars">
            {#each usage.byWorkspace.slice(0, 6) as w (w.workspace ?? "—")}
              <div class="bar-row">
                <span class="bar-k">{cleanPath(w.workspace)}</span>
                <span class="bar-track"><span class="bar-fill" style="width:{Math.max(2, (w.cost / wsMax) * 100)}%; --barH:200"></span></span>
                <span class="bar-v mono">{fmtUsd(w.cost)}</span>
              </div>
            {/each}
          </div>
        {/if}
      </section>
    </div>
  {/if}
</div>

<style>
  .cost { position: relative; flex: 1; min-height: 0; min-width: 0; overflow-y: auto; overflow-x: hidden; color: var(--fg); padding: 8px 22px 8px; background:
      radial-gradient(circle, color-mix(in oklab, var(--fg) 3%, transparent) 1px, transparent 1px) 0 0 / 26px 26px,
      radial-gradient(130% 90% at 50% -25%, color-mix(in oklab, var(--accent) 5%, transparent), transparent 52%),
      var(--bg); }

  /* ── Header ── */
  .chead { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; margin-bottom: 14px; }
  .chead-title { font-size: 23px; font-weight: 750; letter-spacing: -0.02em; display: flex; align-items: center; gap: 9px; }
  .chead-spark { display: inline-flex; color: var(--accent); }
  .chead-sub { font-size: var(--fs-xs); color: var(--fg-subtle); margin-top: 3px; }
  .crefresh { display: inline-grid; place-items: center; width: 30px; height: 30px; border-radius: 999px; border: 1px solid var(--border); background: var(--surface); color: var(--fg-muted); cursor: pointer; flex: none; }
  .crefresh:hover { color: var(--fg); border-color: var(--border-strong); }
  .crefresh :global(.spin) { animation: cspin 0.8s linear infinite; }

  /* ── Bento (mirrors HarnessPage) ── */
  .bento { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; grid-auto-flow: dense; }
  .cell { position: relative; background: var(--surface); border: 1px solid var(--border); border-radius: 16px; padding: 11px 14px; overflow: hidden; min-width: 0; animation: cellrise var(--dur-rise) var(--ease-page) both; }
  .bento > :nth-child(1) { animation-delay: 0ms; }
  .bento > :nth-child(2) { animation-delay: var(--stagger); }
  .bento > :nth-child(3) { animation-delay: calc(var(--stagger) * 2); }
  .bento > :nth-child(4) { animation-delay: calc(var(--stagger) * 3); }
  .bento > :nth-child(n + 5) { animation-delay: calc(var(--stagger) * 4); }
  .wide { grid-column: span 2; }
  .full { grid-column: 1 / -1; }
  .cell-head { display: flex; align-items: center; justify-content: space-between; gap: 10px; margin-bottom: 9px; }
  .cell-title { font-size: var(--fs-sm); font-weight: 650; color: var(--fg); }
  .cell-meta { font-size: 10.5px; color: var(--fg-subtle); font-family: var(--font-mono); }
  .empty { font-size: var(--fs-xs); color: var(--fg-subtle); padding: 10px 0; }
  .dim { color: var(--fg-subtle); }
  .mono { font-family: var(--font-mono); }

  /* ── Insights ("Rift noticed…") ── */
  .ins-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 8px; }
  @media (max-width: 760px) { .ins-grid { grid-template-columns: 1fr; } }
  .ins { display: flex; gap: 10px; padding: 10px 12px; border-radius: 11px; background: var(--bg-inset); border: 1px solid var(--border); }
  .ins-dot { flex-shrink: 0; width: 8px; height: 8px; margin-top: 5px; border-radius: 999px; background: var(--info, var(--accent)); }
  .ins[data-sev="good"] .ins-dot { background: var(--ok); }
  .ins[data-sev="warn"] .ins-dot { background: var(--warn); }
  .ins[data-sev="warn"] { border-color: color-mix(in oklab, var(--warn) 26%, var(--border)); }
  .ins-body { min-width: 0; }
  .ins-title { font-size: var(--fs-sm); font-weight: 650; color: var(--fg); margin-bottom: 2px; }
  .ins-detail { font-size: var(--fs-xs); color: var(--fg-subtle); line-height: 1.45; }

  /* ── Empty hero ── */
  .empty-hero { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; padding: 70px 0; color: var(--fg-subtle); text-align: center; }
  .empty-hero :global(svg) { color: var(--accent); opacity: 0.8; }
  .empty-hero-t { font-size: var(--fs-md); font-weight: 650; color: var(--fg-2); }
  .empty-hero-s { font-size: var(--fs-xs); max-width: 320px; line-height: 1.5; }

  /* ── Fuel-gauge hero ── */
  .hero { grid-column: span 2; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px; padding: 14px 16px; }
  .hero-tag { position: absolute; top: 14px; left: 16px; font-size: 9.5px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.09em; color: var(--fg-faint); z-index: 1; }
  .hero-glow { position: absolute; top: 44%; left: 50%; transform: translate(-50%, -50%); width: 320px; height: 320px; border-radius: 50%; z-index: 0; filter: blur(32px); opacity: 0.55; pointer-events: none;
    background: radial-gradient(circle, color-mix(in oklab, var(--accent) 16%, transparent), transparent 66%); }
  .hero[data-zone="warn"] .hero-glow { background: radial-gradient(circle, color-mix(in oklab, var(--warn) 16%, transparent), transparent 66%); }
  .hero[data-zone="hot"] .hero-glow { background: radial-gradient(circle, color-mix(in oklab, var(--danger) 18%, transparent), transparent 66%); }
  .gauge { position: relative; width: 152px; height: 152px; z-index: 1; }
  .gauge-svg { width: 100%; height: 100%; }
  .gauge-track { fill: none; stroke: var(--bg-inset); stroke-width: 13; }
  .g0 { stop-color: oklch(0.66 0.16 var(--accent-h)); }
  .g1 { stop-color: oklch(0.85 0.13 var(--accent-h)); }
  .gauge-fill { fill: none; stroke-width: 13; stroke-linecap: round; transition: stroke-dashoffset var(--dur-slow) var(--ease-page); filter: drop-shadow(0 0 6px color-mix(in oklab, var(--accent) 45%, transparent)); }
  .gauge-fill[data-zone="warn"] { stroke: var(--warn) !important; filter: drop-shadow(0 0 6px color-mix(in oklab, var(--warn) 45%, transparent)); }
  .gauge-fill[data-zone="hot"] { stroke: var(--danger) !important; filter: drop-shadow(0 0 6px color-mix(in oklab, var(--danger) 50%, transparent)); }
  .gauge-mid { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; }
  .gauge-pct { font-size: 50px; font-weight: 760; letter-spacing: -0.03em; line-height: 1; color: var(--fg); font-variant-numeric: tabular-nums; }
  .gauge-pct[data-zone="warn"] { color: var(--warn); }
  .gauge-pct[data-zone="hot"] { color: var(--danger); }
  .gauge-pct-u { font-size: 21px; font-weight: 600; color: var(--fg-subtle); margin-left: 2px; }
  .gauge-cap { font-size: var(--fs-xs); color: var(--fg-muted); margin-top: 5px; text-transform: uppercase; letter-spacing: 0.06em; }
  .hero-foot { display: flex; flex-direction: column; align-items: center; gap: 6px; z-index: 1; }
  .hero-spent { font-size: var(--fs-sm); color: var(--fg-2); font-family: var(--font-mono); }
  .hero-zone { font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.08em; padding: 2px 10px; border-radius: 999px; }
  .hero-zone[data-zone="ok"] { color: var(--ok); background: var(--ok-soft); }
  .hero-zone[data-zone="warn"] { color: var(--warn); background: var(--warn-soft); }
  .hero-zone[data-zone="hot"] { color: var(--danger); background: color-mix(in oklab, var(--danger) 15%, transparent); }

  /* ── Plan picker ── */
  .plan-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 6px; }
  .plan { display: flex; flex-direction: column; align-items: center; gap: 2px; padding: 9px 4px; border-radius: 10px; border: 1px solid var(--border); background: var(--bg-inset); color: var(--fg-muted); cursor: pointer; transition: border-color var(--dur-fast) var(--ease-soft), color var(--dur-fast) var(--ease-soft), background var(--dur-fast) var(--ease-soft); }
  .plan:hover { border-color: var(--border-strong); color: var(--fg); }
  .plan.on { border-color: var(--ghost-border); background: var(--accent-soft); color: var(--accent); }
  .plan-l { font-size: var(--fs-xs); font-weight: 650; }
  .plan-sub { font-size: 10px; font-family: var(--font-mono); color: var(--fg-subtle); }
  .plan.on .plan-sub { color: color-mix(in oklab, var(--accent) 80%, var(--fg)); }
  .custom-row { display: flex; align-items: center; gap: 9px; margin-top: 9px; }
  .custom-lbl { font-size: var(--fs-xs); color: var(--fg-muted); }
  .custom-in { height: 28px; padding: 0 10px; border-radius: 7px; background: var(--field); border: 1px solid var(--field-border); color: var(--fg); font-size: var(--fs-xs); width: 110px; }
  .custom-in:focus { outline: 0; border-color: var(--border-focus); box-shadow: 0 0 0 3px var(--ring); }
  .cad { display: inline-flex; margin-top: 10px; background: var(--track); border: 1px solid var(--border); border-radius: 7px; padding: 2px; gap: 2px; }
  .cad-btn { height: 24px; padding: 0 12px; border: 0; border-radius: 5px; background: none; color: var(--fg-muted); font: inherit; font-size: 11px; font-weight: 600; cursor: pointer; }
  .cad-btn.on { background: var(--surface-hover); color: var(--fg); }

  /* ── KPI rail ── */
  .kpi-rail { display: flex; flex-wrap: wrap; align-items: stretch; gap: 0; padding: 0; }
  .kpi { flex: 1 1 120px; display: flex; flex-direction: column; justify-content: center; gap: 3px; padding: 10px 18px; }
  .kpi + .kpi { border-left: 1px solid color-mix(in oklab, var(--border) 70%, transparent); }
  .kpi-v { font-size: 22px; font-weight: 720; letter-spacing: -0.02em; color: var(--fg); font-variant-numeric: tabular-nums; line-height: 1; }
  .kpi-v.nodata { color: var(--fg-faint); }
  .kpi-u { font-size: 12px; color: var(--fg-subtle); font-weight: 500; }
  .kpi-k { font-size: var(--fs-xs); color: var(--fg-muted); text-transform: uppercase; letter-spacing: 0.05em; }

  /* ── Bars ── */
  .bars { display: flex; flex-direction: column; gap: 8px; }
  .bar-row { display: grid; grid-template-columns: 152px 1fr 54px; align-items: center; gap: 11px; }
  .bar-k { font-size: var(--fs-xs); color: var(--fg-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; display: inline-flex; align-items: center; gap: 6px; }
  .mdl-dot { width: 8px; height: 8px; border-radius: 50%; flex: none; background: oklch(0.74 0.16 var(--mH)); box-shadow: 0 0 6px oklch(0.74 0.16 var(--mH) / 0.55); }
  .est { font-size: 8.5px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; color: var(--warn); background: var(--warn-soft); padding: 1px 5px; border-radius: 5px; flex: none; }
  .bar-track { height: 9px; border-radius: 999px; background: var(--bg-inset); overflow: hidden; position: relative; }
  .bar-fill { position: absolute; inset: 0 auto 0 0; height: 100%; border-radius: 999px; transition: width var(--dur-slow) var(--ease-page);
    background: linear-gradient(90deg, oklch(0.62 0.15 var(--barH)), oklch(0.78 0.16 var(--barH))); }
  .bar-v { font-size: var(--fs-xs); color: var(--fg-2); text-align: right; }

  /* ── Daily timeline ── */
  .tl-track { position: relative; display: flex; align-items: flex-end; gap: 3px; height: 78px; padding-top: 10px; overflow-x: auto; overflow-y: hidden; border-bottom: 1px solid color-mix(in oklab, var(--border) 85%, transparent); }
  .tl-grid { position: absolute; inset: 10px 0 0 0; z-index: 0; pointer-events: none;
    background: repeating-linear-gradient(to top, transparent 0, transparent 32px, color-mix(in oklab, var(--border) 45%, transparent) 32px, color-mix(in oklab, var(--border) 45%, transparent) 33px); }
  .tl-bar { flex: 1 1 8px; min-width: 6px; max-width: 28px; border-radius: 3px 3px 2px 2px; position: relative; z-index: 1; align-self: flex-end;
    background: linear-gradient(180deg, oklch(0.82 0.13 var(--accent-h)), oklch(0.58 0.15 var(--accent-h)));
    transition: height var(--dur-slow) var(--ease-page); cursor: default; }
  .tl-bar:hover { filter: brightness(1.18); }
  .tl-axis { display: flex; justify-content: space-between; font-size: 10px; color: var(--fg-faint); margin-top: 6px; letter-spacing: 0.04em; }
  .tl-empty { padding: 30px 0; text-align: center; }

  @keyframes cellrise { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: translateY(0); } }
  @keyframes cspin { to { transform: rotate(360deg); } }

  @media (max-width: 1080px) {
    .bento { grid-template-columns: repeat(2, 1fr); }
    .hero, .wide { grid-column: span 2; }
    .plan-grid { grid-template-columns: repeat(4, 1fr); }
  }
  @media (prefers-reduced-motion: reduce) {
    .cell, .crefresh :global(.spin) { animation: none; }
  }
</style>

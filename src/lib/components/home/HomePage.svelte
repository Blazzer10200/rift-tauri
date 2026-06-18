<script lang="ts">
  import {
    Sparkles, Send, GitBranch, MessageSquare, FolderOpen, ChevronRight,
    X, ArrowUpCircle, Copy, Check, Loader2,
  } from "lucide-svelte";
  import { onMount } from "svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import { cliUpdate } from "$lib/state/cliUpdate.svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { usage, type LimitWindow } from "$lib/state/usage.svelte";
  import { localLlm } from "$lib/state/localLlm.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { modelLabel } from "./statsHelpers";
  import { MODEL_OPTIONS } from "$lib/components/assistant/composer/modelMatrix";
  import type { ModelSel } from "$lib/state/assistant/types";
  import { leafName } from "$lib/components/shell/tabsbar/helpers";

  // Claude Code CLI update — the dashboard is the launch surface, so kick the
  // (throttled) npm check here and surface a dismissible banner if one's live.
  const cliInstalled = $derived(assistant.auth?.cliVersion ?? null);
  const cliUpdAvail = $derived(cliUpdate.availableAny(assistant.auth?.installs, cliInstalled));
  const cliSummary = $derived(cliUpdate.summary(assistant.auth?.installs));
  onMount(() => {
    void cliUpdate.maybeCheck();
    void usage.refreshRateLimits(assistant.auth?.cliVersion ?? null);
    void localLlm.refresh();
  });
  $effect(() => { cliUpdate.setMethod(assistant.auth?.installMethod ?? null); });

  async function runCliUpdate() {
    const ok = await cliUpdate.runUpdate();
    if (ok) await assistant.refreshAuth();
  }

  // ── Workspace state ──
  const root = $derived(assistant.workspace.current);
  const hasRoot = $derived(root != null);
  const branch = $derived(assistant.workspaceBranch);
  $effect(() => {
    if (assistant.workspace.current) void assistant.loadWorkspaceBranch();
  });

  function fmtAgo(input: number | string | null): string {
    if (input == null) return "—";
    const ms = typeof input === "number" ? input : Date.parse(input);
    if (!Number.isFinite(ms)) return typeof input === "string" ? input : "—";
    const diff = Date.now() - ms;
    const min = 60_000, hr = 60 * min, day = 24 * hr;
    if (diff < min) return "just now";
    if (diff < hr) return `${Math.floor(diff / min)}m ago`;
    if (diff < day) return `${Math.floor(diff / hr)}h ago`;
    if (diff < 7 * day) return `${Math.floor(diff / day)}d ago`;
    return new Date(ms).toLocaleDateString();
  }

  const currentModel = $derived(modelLabel(assistant.model));

  // tick the hour each minute so the greeting refreshes across day boundaries
  let nowHour = $state(new Date().getHours());
  $effect(() => {
    const t = setInterval(() => { nowHour = new Date().getHours(); }, 60_000);
    return () => clearInterval(t);
  });
  function greeting(hr: number): string {
    if (hr < 5) return "Still up";
    if (hr < 12) return "Good morning";
    if (hr < 18) return "Good afternoon";
    return "Good evening";
  }
  const greet = $derived(greeting(nowHour));

  // ── Recents — featured (most recent) + two more ──
  const recentChats = $derived(
    assistant.conversations
      .filter((c) => c.id !== assistant.currentConvoId)
      .filter((c) => c.messageCount >= 3)
      .slice(0, 3),
  );
  const featured = $derived(recentChats[0] ?? null);
  const restChats = $derived(recentChats.slice(1, 3));

  function goNewTab() {
    void assistant.newTab();
    workspace.setActive("chat");
  }
  function resume(id: string) {
    void assistant.openTab(id);
    workspace.setActive("chat");
  }

  // ── Inline Ask composer ──
  let askDraft = $state("");
  let askEl = $state<HTMLTextAreaElement | null>(null);
  let modelMenuOpen = $state(false);
  const homeModels = $derived(MODEL_OPTIONS.filter((m) => !m.legacy));

  function autogrow() {
    if (!askEl) return;
    askEl.style.height = "auto";
    askEl.style.height = Math.min(askEl.scrollHeight, 132) + "px";
  }
  function pickModel(id: ModelSel) {
    assistant.setModel(id);
    modelMenuOpen = false;
    askEl?.focus();
  }
  async function submitAsk() {
    const text = askDraft.trim();
    if (!text) return;
    askDraft = "";
    if (askEl) askEl.style.height = "auto";
    await assistant.newTab();
    workspace.setActive("chat");
    void assistant.send(text);
  }
  function askKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void submitAsk();
    }
  }
  // chip "moves" — start a fresh chat seeded with the move prompt
  async function startMove(text: string) {
    await assistant.newTab();
    workspace.setActive("chat");
    void assistant.send(text);
  }
  const MOVES = [
    { label: "Explain this codebase", prompt: "Explain this codebase — map the architecture and the key modules." },
    { label: "Find a bug", prompt: "Help me find a bug — trace a failure back to its root cause." },
    { label: "Write a test", prompt: "Write a test covering an important edge case in this project." },
    { label: "Refactor a file", prompt: "Refactor a file — clean it up without changing behavior." },
  ];

  // ── Plan-limit gauges ──
  const gauges = $derived.by(() => {
    const rl = usage.rateLimits;
    if (!rl) return [] as { k: string; short: string; w: LimitWindow }[];
    const out: { k: string; short: string; w: LimitWindow }[] = [];
    if (rl.fiveHour) out.push({ k: "5-hour window", short: "5-hour", w: rl.fiveHour });
    if (rl.sevenDay) out.push({ k: "Weekly · all models", short: "weekly", w: rl.sevenDay });
    return out;
  });
</script>

<div class="home">
  <div class="aurora"></div>
  <div class="vignette"></div>

  <!-- top context strip -->
  <div class="strip anim d1">
    <span class="led"></span>
    <span class="stamp">{greet}</span>
    {#if branch}
      <span class="branch"><GitBranch size={11} />{branch}</span>
    {/if}
    <span class="spacer"></span>
    {#each gauges as g (g.k)}
      <span class="mini" use:tooltip={g.k}>
        {g.short}
        <span class="gauge"><i style="width:{Math.min(100, Math.max(2, g.w.utilization))}%"></i></span>
        {g.w.utilization.toFixed(0)}%
      </span>
    {/each}
  </div>

  {#if cliUpdAvail}
    <div class="dash-cli anim d1" role="status" data-tone={cliSummary.tone}>
      <span class="dc-ic"><ArrowUpCircle size={15} /></span>
      <span class="dc-body">
        <span class="dc-t">{cliSummary.headline}</span>
        <span class="dc-s"><code>{cliInstalled}</code> → <code>{cliUpdate.latest}</code> <span class="dc-detail">· {cliSummary.detail}</span></span>
      </span>
      <button class="dc-go" type="button" disabled={cliUpdate.updating} onclick={runCliUpdate} use:tooltip={cliUpdate.updateCommand}>
        {#if cliUpdate.updating}<Loader2 size={14} class="dc-spin" />Updating…{:else}<ArrowUpCircle size={14} />Update now{/if}
      </button>
      <button class="dc-copy" class:done={cliUpdate.copied} type="button" onclick={() => void cliUpdate.copyCommand()} use:tooltip={"Copy update command"} aria-label="Copy update command">
        {#if cliUpdate.copied}<Check size={14} />{:else}<Copy size={14} />{/if}
      </button>
      <button class="dc-x" type="button" use:tooltip={"Dismiss"} aria-label="Dismiss update notice" onclick={() => cliUpdate.dismiss()}><X size={14} /></button>
    </div>
  {/if}

  <!-- centered focus column -->
  <div class="center">
    <div class="glyph anim d2"><Sparkles size={28} /></div>
    <div class="eyebrow anim d2">New conversation</div>
    {#if hasRoot}
      <h1 class="anim d3">What's next for <span class="emer">{leafName(root!)}</span>?</h1>
    {:else}
      <h1 class="anim d3">Open a project to begin</h1>
    {/if}
    <p class="sub anim d3">Ask anything about this workspace, or pick up one of the moves below.</p>

    <div class="ask anim d4" class:filled={askDraft.trim().length > 0}>
      <span class="ag"><Sparkles size={19} /></span>
      <textarea
        bind:this={askEl}
        bind:value={askDraft}
        class="ph"
        rows="1"
        placeholder={hasRoot ? `Ask ${localLlm.askLabel} about ${leafName(root!)}…` : `Ask ${localLlm.askLabel} anything…`}
        oninput={autogrow}
        onkeydown={askKeydown}
      ></textarea>
      <div class="model-wrap">
        <button class="model" type="button" aria-haspopup="menu" aria-expanded={modelMenuOpen} onclick={() => (modelMenuOpen = !modelMenuOpen)}>
          <span class="dot"></span>{currentModel}
        </button>
        {#if modelMenuOpen}
          <button class="model-backdrop" type="button" aria-label="Close model menu" onclick={() => (modelMenuOpen = false)}></button>
          <div class="model-menu" role="menu">
            {#each homeModels as m (m.id)}
              <button class="mm-row" class:on={assistant.model === m.id} role="menuitemradio" aria-checked={assistant.model === m.id} type="button" onclick={() => pickModel(m.id)}>
                <span class="mm-name">{m.label}<span class="mm-ver">{m.version}</span></span>
                <span class="mm-blurb">{m.blurb}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
      <button class="send" type="button" disabled={!askDraft.trim()} onclick={submitAsk} use:tooltip={"Send · Enter"} aria-label="Send"><Send size={16} /></button>
    </div>

    <div class="chips anim d5">
      {#each MOVES as mv (mv.label)}
        <button class="chip" type="button" onclick={() => void startMove(mv.prompt)}>
          {#if mv.label === "Explain this codebase"}<svg class="ico" viewBox="0 0 24 24"><path d="M16 18l6-6-6-6M8 6l-6 6 6 6"/></svg>
          {:else if mv.label === "Find a bug"}<svg class="ico" viewBox="0 0 24 24"><circle cx="11" cy="11" r="7"/><path d="M21 21l-4-4"/></svg>
          {:else if mv.label === "Write a test"}<svg class="ico" viewBox="0 0 24 24"><path d="M9 11l3 3 8-8"/><path d="M20 12v6a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h9"/></svg>
          {:else}<svg class="ico" viewBox="0 0 24 24"><path d="M14 4l6 6M3 21l4-1L20 7l-3-3L4 17z"/></svg>{/if}
          {mv.label}
        </button>
      {/each}
    </div>
  </div>

  <!-- bottom resume rail: featured Continue + two recents -->
  <div class="rail anim d6">
    <div class="rail-h">Jump back in</div>
    <div class="resumes">
      {#if featured}
        <button class="resume feature" onclick={() => resume(featured.id)} use:tooltip={`${featured.title} · ${featured.model}`}>
          <span class="ri"><MessageSquare size={15} /></span>
          <span class="rb"><span class="rt">{featured.title}</span><span class="rs">{featured.lastSnippet ?? `${featured.messageCount} msg`}</span></span>
          <span class="rcont">Resume<ChevronRight size={13} /></span>
        </button>
      {:else}
        <button class="resume feature" onclick={() => void assistant.pickFolder()}>
          <span class="ri"><FolderOpen size={15} /></span>
          <span class="rb"><span class="rt">Open a folder</span><span class="rs">Pick any project on your disk to begin</span></span>
          <span class="rcont">Browse<ChevronRight size={13} /></span>
        </button>
      {/if}
      {#each restChats as c (c.id)}
        <button class="resume" onclick={() => resume(c.id)} use:tooltip={`${c.title} · ${c.model}`}>
          <span class="ri"><MessageSquare size={15} /></span>
          <span class="rb"><span class="rt">{c.title}</span><span class="rs">{c.lastSnippet ?? `${c.messageCount} msg`}</span></span>
          <span class="rw">{fmtAgo(c.updatedAt)}</span>
        </button>
      {/each}
      {#if restChats.length < 2}
        <button class="resume" onclick={goNewTab}>
          <span class="ri"><Sparkles size={15} /></span>
          <span class="rb"><span class="rt">New chat</span><span class="rs">Start a fresh conversation</span></span>
          <span class="rw"></span>
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .home { height: 100%; overflow: hidden; position: relative; display: flex; flex-direction: column; background: var(--bg); color: var(--fg); }
  .ico { width: 1em; height: 1em; display: block; fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }

  /* ambient aurora */
  .aurora { position: absolute; inset: 0; overflow: hidden; pointer-events: none; z-index: 0; }
  .aurora::before, .aurora::after { content: ""; position: absolute; border-radius: 50%; filter: blur(80px); opacity: 0.5; }
  .aurora::before { width: 720px; height: 480px; top: -180px; left: 50%; transform: translateX(-50%); background: radial-gradient(closest-side, oklch(0.55 0.10 var(--accent-h) / 0.55), transparent); }
  .aurora::after { width: 560px; height: 420px; bottom: -160px; left: 14%; background: radial-gradient(closest-side, oklch(0.50 0.08 230 / 0.30), transparent); }
  .vignette { position: absolute; inset: 0; z-index: 0; pointer-events: none; background: radial-gradient(1200px 760px at 50% -6%, transparent 40%, oklch(0.118 0.003 250 / 0.55) 100%); }
  .strip, .center, .rail, .dash-cli { position: relative; z-index: 1; }

  /* top strip */
  .strip { display: flex; align-items: center; gap: 14px; padding: 18px 30px; flex: none; color: var(--fg-subtle); }
  .led { width: 7px; height: 7px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 0 0 var(--ring); }
  @media (prefers-reduced-motion: no-preference) { .led { animation: breathe 2.6s ease-in-out infinite; } }
  @keyframes breathe { 0%,100% { box-shadow: 0 0 0 2px color-mix(in oklch, var(--accent) 26%, transparent); } 50% { box-shadow: 0 0 0 5px color-mix(in oklch, var(--accent) 12%, transparent); } }
  .stamp { font-family: var(--font-mono); font-size: 11px; letter-spacing: 0.06em; text-transform: uppercase; color: var(--fg-subtle); }
  .branch { display: inline-flex; align-items: center; gap: 6px; font-family: var(--font-mono); font-size: 11px; color: var(--info); background: var(--info-soft); padding: 3px 9px; border-radius: 999px; }
  .spacer { flex: 1; }
  .mini { display: inline-flex; align-items: center; gap: 8px; font-size: 11px; color: var(--fg-subtle); font-variant-numeric: tabular-nums; }
  .gauge { width: 70px; height: 5px; border-radius: 999px; background: var(--bg-inset); overflow: hidden; }
  .gauge > i { display: block; height: 100%; border-radius: 999px; background: linear-gradient(90deg, oklch(0.62 0.15 var(--accent-h)), oklch(0.80 0.16 var(--accent-h))); }

  /* centered focus column */
  .center { flex: 1; min-height: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 22px; padding: 0 32px; margin-top: -18px; }
  .glyph { width: 60px; height: 60px; border-radius: var(--radius-xl); background: var(--accent-soft); color: var(--accent); display: grid; place-items: center; box-shadow: 0 0 0 1px var(--ghost-border), 0 16px 50px -14px color-mix(in oklab, var(--accent) 45%, transparent); }
  .eyebrow { font-family: var(--font-mono); font-size: 11px; letter-spacing: 0.16em; text-transform: uppercase; color: var(--fg-faint); }
  h1 { margin: 0; font-size: 42px; font-weight: 700; letter-spacing: -0.03em; line-height: 1.04; text-align: center; color: var(--fg); }
  h1 .emer { color: var(--accent); }
  .sub { margin: -10px 0 0; color: var(--fg-muted); font-size: 15px; text-align: center; max-width: 52ch; text-wrap: balance; }

  /* Ask bar */
  .ask { width: 740px; max-width: 92%; display: flex; align-items: center; gap: 12px; padding: 14px 14px 14px 18px; background: color-mix(in oklch, var(--bg-inset) 92%, var(--accent) 8%); border: 1px solid var(--border-strong); border-radius: var(--radius-2xl); box-shadow: 0 0 0 4px color-mix(in oklab, var(--accent) 6%, transparent), 0 24px 60px -22px black; }
  @media (prefers-reduced-motion: no-preference) { .ask { animation: askGlow 4.2s ease-in-out infinite; } }
  @keyframes askGlow { 0%,100% { box-shadow: 0 0 0 4px color-mix(in oklab, var(--accent) 5%, transparent), 0 24px 60px -22px black; } 50% { box-shadow: 0 0 0 5px color-mix(in oklab, var(--accent) 11%, transparent), 0 24px 60px -22px black; } }
  .ask:focus-within { border-color: var(--ghost-border); }
  .ag { width: 36px; height: 36px; border-radius: var(--radius); background: var(--accent-soft); color: var(--accent); display: grid; place-items: center; flex: none; }
  .ph { flex: 1; min-width: 0; resize: none; border: 0; background: transparent; color: var(--fg); font: inherit; font-size: 16px; line-height: 1.4; padding: 4px 0; max-height: 132px; overflow-y: auto; }
  .ph::placeholder { color: var(--fg-subtle); }
  .ph:focus { outline: none; }
  .model-wrap { position: relative; flex: none; }
  .model { display: inline-flex; align-items: center; gap: 7px; height: 34px; padding: 0 13px; border-radius: var(--radius); background: var(--surface); border: 1px solid var(--border); color: var(--fg-2); font: inherit; font-size: 12px; font-weight: 600; cursor: pointer; }
  .model:hover { background: var(--surface-hover); border-color: var(--border-strong); }
  .model .dot { width: 7px; height: 7px; border-radius: 999px; background: var(--accent); }
  .model-backdrop { position: fixed; inset: 0; z-index: 40; background: transparent; border: 0; cursor: default; }
  .model-menu { position: absolute; z-index: 41; bottom: calc(100% + 8px); right: 0; min-width: 248px; display: flex; flex-direction: column; gap: 1px; padding: 5px; background: var(--bg-elev-2, var(--surface)); border: 1px solid var(--border-strong); border-radius: var(--radius-lg); box-shadow: 0 12px 32px -8px color-mix(in oklab, black 55%, transparent); }
  .mm-row { display: flex; flex-direction: column; gap: 1px; padding: 8px 10px; border: 0; border-radius: var(--radius); background: transparent; color: var(--fg); text-align: left; cursor: pointer; }
  .mm-row:hover { background: var(--surface-hover); }
  .mm-row.on { background: var(--accent-soft); }
  .mm-name { font-size: 12px; font-weight: 650; display: flex; align-items: baseline; gap: 6px; }
  .mm-row.on .mm-name { color: var(--accent); }
  .mm-ver { font-family: var(--font-mono); font-size: 10px; font-weight: 600; color: var(--fg-subtle); }
  .mm-blurb { font-size: 11px; color: var(--fg-muted); }
  .send { width: 40px; height: 34px; border-radius: var(--radius); background: var(--accent); color: var(--accent-fg); display: grid; place-items: center; flex: none; border: 0; cursor: pointer; box-shadow: inset 0 1px 0 color-mix(in oklch, white 28%, transparent), 0 3px 14px -3px color-mix(in oklab, var(--accent) 55%, transparent); }
  .send:hover:not(:disabled) { background: var(--accent-hover); }
  .send:disabled { opacity: 0.4; cursor: default; box-shadow: none; }

  .chips { display: flex; gap: 10px; flex-wrap: wrap; justify-content: center; max-width: 760px; }
  .chip { display: inline-flex; align-items: center; gap: 8px; padding: 9px 14px; border-radius: 999px; background: var(--surface); border: 1px solid var(--border); color: var(--fg-2); font: inherit; font-size: 12.5px; font-weight: 500; cursor: pointer; transition: border-color var(--dur-fast) var(--ease-soft), background var(--dur-fast) var(--ease-soft); }
  .chip:hover { border-color: var(--ghost-border); background: var(--surface-hover); }
  .chip .ico { color: var(--accent); }

  /* resume rail */
  .rail { flex: none; padding: 0 30px 26px; }
  .rail-h { display: flex; align-items: center; gap: 9px; font-size: 10px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.09em; color: var(--fg-faint); margin-bottom: 12px; }
  .rail-h::after { content: ""; flex: 1; height: 1px; background: linear-gradient(to right, color-mix(in oklch, var(--border) 80%, transparent), transparent); }
  .resumes { display: grid; grid-template-columns: 1.5fr 1fr 1fr; gap: 12px; }
  .resume { display: flex; align-items: center; gap: 12px; padding: 13px 15px; border-radius: var(--radius-lg); background: var(--surface); border: 1px solid var(--border); min-width: 0; text-align: left; font: inherit; color: var(--fg); cursor: pointer; transition: border-color var(--dur-fast) var(--ease-soft), transform var(--dur-fast) var(--ease-soft); }
  .resume:hover { border-color: var(--border-strong); transform: translateY(-1px); }
  .resume.feature { background: linear-gradient(135deg, var(--accent-soft), transparent 64%), var(--surface); border-color: var(--ghost-border); }
  .ri { width: 32px; height: 32px; border-radius: var(--radius); background: var(--accent-soft); color: var(--accent); display: grid; place-items: center; flex: none; }
  .resume.feature .ri { background: var(--accent); color: var(--accent-fg); }
  .rb { min-width: 0; flex: 1; display: flex; flex-direction: column; }
  .rt { font-size: 13px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .rs { font-size: 11px; color: var(--fg-subtle); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-top: 1px; }
  .rcont { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; font-weight: 650; color: var(--accent); white-space: nowrap; flex: none; }
  .rw { margin-left: auto; font-family: var(--font-mono); font-size: 10.5px; color: var(--fg-faint); flex: none; }

  /* CLI update banner */
  .dash-cli { flex: none; display: flex; align-items: center; gap: 11px; margin: 0 30px; padding: 8px 10px 8px 12px; border-radius: 12px; background: color-mix(in oklab, var(--accent) 9%, var(--surface)); border: 1px solid color-mix(in oklab, var(--accent) 32%, var(--border)); }
  .dash-cli[data-tone="warn"] { background: color-mix(in oklab, var(--warn) 9%, var(--surface)); border-color: color-mix(in oklab, var(--warn) 32%, var(--border)); }
  .dash-cli[data-tone="danger"] { background: color-mix(in oklab, var(--danger) 9%, var(--surface)); border-color: color-mix(in oklab, var(--danger) 32%, var(--border)); }
  .dc-ic { width: 26px; height: 26px; border-radius: var(--radius); flex: none; display: grid; place-items: center; background: var(--accent-soft); color: var(--accent); }
  .dash-cli[data-tone="warn"] .dc-ic { background: color-mix(in oklab, var(--warn) 14%, transparent); color: var(--warn); }
  .dash-cli[data-tone="danger"] .dc-ic { background: color-mix(in oklab, var(--danger) 14%, transparent); color: var(--danger); }
  .dc-body { min-width: 0; display: flex; flex-direction: row; align-items: baseline; gap: 9px; }
  .dc-t { font-size: 12px; font-weight: 650; color: var(--fg); white-space: nowrap; flex: none; }
  .dc-s { font-size: 11px; color: var(--fg-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dc-s code { font-family: var(--font-mono); color: var(--fg-2); }
  .dash-cli[data-tone="warn"] .dc-detail { color: var(--warn); }
  .dash-cli[data-tone="danger"] .dc-detail { color: var(--danger); }
  .dc-go { margin-left: auto; flex: none; display: inline-flex; align-items: center; gap: 6px; height: 30px; padding: 0 12px; border-radius: var(--radius); font: inherit; font-size: 11px; font-weight: 650; cursor: pointer; background: var(--accent); color: var(--accent-fg); border: 1px solid transparent; }
  .dc-go:hover:not(:disabled) { background: var(--accent-hover); }
  .dc-go:disabled { opacity: 0.7; cursor: default; }
  :global(.dash-cli .dc-go .dc-spin) { animation: dc-spin 0.8s linear infinite; }
  @keyframes dc-spin { to { transform: rotate(360deg); } }
  .dc-copy { flex: none; display: inline-flex; align-items: center; justify-content: center; width: 30px; height: 30px; border-radius: var(--radius); font: inherit; cursor: pointer; background: var(--surface-hover); color: var(--fg-2); border: 1px solid var(--border); }
  .dc-copy:hover { color: var(--fg); }
  .dc-copy.done { color: var(--ok, var(--accent)); }
  .dc-x { flex: none; display: inline-flex; align-items: center; justify-content: center; width: 28px; height: 28px; border-radius: var(--radius); cursor: pointer; background: transparent; border: 0; color: var(--fg-faint); }
  .dc-x:hover { color: var(--fg); background: var(--surface-hover); }

  /* staggered entrance */
  @media (prefers-reduced-motion: no-preference) {
    .anim { opacity: 0; animation: rise var(--dur-page) var(--ease-page) forwards; }
    .d1 { animation-delay: 0.04s; } .d2 { animation-delay: 0.12s; } .d3 { animation-delay: 0.20s; }
    .d4 { animation-delay: 0.30s; } .d5 { animation-delay: 0.40s; } .d6 { animation-delay: 0.50s; }
  }
  @keyframes rise { from { opacity: 0; transform: translateY(12px); } to { opacity: 1; transform: none; } }

  /* narrow: let it scroll */
  @media (max-width: 900px) {
    .home { overflow-y: auto; }
    .resumes { grid-template-columns: 1fr; }
    .ask { width: 100%; }
  }
</style>

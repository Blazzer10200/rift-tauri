<script lang="ts">
  import {
    MessageSquare, Sparkles, Send, FolderOpen, Folder, FolderGit2,
    GitBranch, ChevronRight, History, X, ArrowUpCircle, Copy, Check, Loader2,
  } from "lucide-svelte";
  import { onMount } from "svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import { cliUpdate } from "$lib/state/cliUpdate.svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { tooltip } from "$lib/actions/tooltip";

  // Claude Code CLI update — the dashboard is the launch surface, so kick the
  // (throttled) npm check here and surface a dismissible banner if one's live.
  const cliInstalled = $derived(assistant.auth?.cliVersion ?? null);
  const cliUpdAvail = $derived(cliUpdate.availableAny(assistant.auth?.installs, cliInstalled));
  const cliSummary = $derived(cliUpdate.summary(assistant.auth?.installs));
  onMount(() => { void cliUpdate.maybeCheck(); });
  // Keep the update command method-aware (npm vs native).
  $effect(() => { cliUpdate.setMethod(assistant.auth?.installMethod ?? null); });

  async function runCliUpdate() {
    const ok = await cliUpdate.runUpdate();
    if (ok) await assistant.refreshAuth();
  }

  // ── Workspace state — the local folder Claude operates on ──
  const root = $derived(assistant.workspace.current);
  const hasRoot = $derived(root != null);
  const recents = $derived(assistant.workspace.recent);
  const branch = $derived(assistant.workspaceBranch);

  // Resolve the git branch lazily whenever a root is set.
  $effect(() => {
    if (assistant.workspace.current) void assistant.loadWorkspaceBranch();
  });

  function leafName(p: string): string {
    const norm = p.replace(/\\/g, "/").replace(/\/$/, "");
    const parts = norm.split("/");
    return parts[parts.length - 1] || norm;
  }
  function shortPath(p: string): string {
    const norm = p.replace(/\\/g, "/").replace(/\/$/, "");
    const parts = norm.split("/");
    if (parts.length <= 2) return norm;
    return `…/${parts.slice(-2).join("/")}`;
  }
  // Display-only strip of Windows extended-length prefixes; stored value keeps them.
  function dispPath(p: string): string {
    return p.replace(/^\\\\\?\\UNC\\/, "\\\\").replace(/^\\\\\?\\/, "");
  }

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

  // Default model the composer opens a new chat with — display-only mirror.
  const MODEL_LABELS: Record<string, string> = {
    sonnet: "Sonnet 4.6", haiku: "Haiku 4.5", "claude-opus-4-7": "Opus 4.7", opus: "Opus 4.8",
  };
  const modelLabel = $derived(MODEL_LABELS[assistant.model] ?? String(assistant.model));

  // #189: tick the hour each minute so the greeting refreshes across day
  // boundaries instead of freezing at the value computed when first mounted.
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

  // ── Recents — real conversations w/ a real back-and-forth ──
  const recentChats = $derived(
    assistant.conversations
      .filter((c) => c.id !== assistant.currentConvoId)
      .filter((c) => c.messageCount >= 3)
      .slice(0, 6),
  );

  function go(draft?: string) {
    if (draft != null) assistant.composerDraft = draft;
    workspace.setActive("chat");
  }
  function resume(id: string) {
    void assistant.openTab(id);
    workspace.setActive("chat");
  }
</script>

<div class="hf">
  <div class="hf-inner">
    <!-- Hero -->
    <section class="hf-hero">
      <div class="hf-hero-l">
        <div class="hf-eyebrow"><span class="led"></span>{greet}</div>
        {#if hasRoot}
          <h1 class="hf-title">What's next for <span class="emer">{leafName(root!)}</span>?</h1>
          <p class="hf-sub">Claude reads, greps, and edits across your workspace in place.</p>
        {:else}
          <h1 class="hf-title">Open a project to begin</h1>
          <p class="hf-sub">Point Claude at any folder on your disk — it reads, greps, and edits in place.</p>
        {/if}
      </div>
      <div class="hf-hero-actions">
        <button class="hf-btn primary" onclick={() => go()}><Sparkles size={15} />New chat</button>
      </div>
    </section>

    <!-- Ask Claude -->
    <button class="dash-ask" onclick={() => go()}>
      <span class="da-glyph"><Sparkles size={16} /></span>
      <span class="da-text">{assistant.composerDraft || (hasRoot ? `Ask Claude about ${leafName(root!)}…` : "Ask Claude anything…")}</span>
      <span class="da-model"><span class="da-dot"></span>{modelLabel}</span>
      <span class="da-send"><Send size={14} /></span>
    </button>

    {#if cliUpdAvail}
      <div class="dash-cli" role="status" data-tone={cliSummary.tone}>
        <span class="dc-ic"><ArrowUpCircle size={17} /></span>
        <span class="dc-body">
          <span class="dc-t">{cliSummary.headline}</span>
          <span class="dc-s">
            <code>{cliInstalled}</code> → <code>{cliUpdate.latest}</code>
            <span class="dc-detail">· {cliSummary.detail}</span>
          </span>
        </span>
        <button
          class="dc-go"
          type="button"
          disabled={cliUpdate.updating}
          onclick={runCliUpdate}
          use:tooltip={cliUpdate.updateCommand}
        >
          {#if cliUpdate.updating}<Loader2 size={14} class="dc-spin" />Updating…{:else}<ArrowUpCircle size={14} />Update now{/if}
        </button>
        <button
          class="dc-copy"
          class:done={cliUpdate.copied}
          type="button"
          onclick={() => void cliUpdate.copyCommand()}
          use:tooltip={"Copy update command"}
          aria-label="Copy update command"
        >
          {#if cliUpdate.copied}<Check size={14} />{:else}<Copy size={14} />{/if}
        </button>
        <button class="dc-x" type="button" use:tooltip={"Dismiss"} aria-label="Dismiss update notice" onclick={() => cliUpdate.dismiss()}><X size={14} /></button>
      </div>
    {/if}

    <div class="hf-grid">
      <!-- Workspace -->
      <div class="hf-card">
        <div class="l3-card-head">
          <span class="ci"><FolderGit2 size={14} /></span>
          <span class="t">Workspace</span>
          {#if branch}<span class="badge info"><GitBranch size={11} /> {branch}</span>{/if}
        </div>

        {#if hasRoot}
          <div class="ws-current">
            <span class="ws-cur-ic"><Folder size={15} /></span>
            <span class="ws-cur-body">
              <span class="ws-cur-name">{leafName(root!)}</span>
              <span class="ws-cur-path mono">{dispPath(root!)}</span>
            </span>
          </div>
          <button class="hf-btn ws-change" onclick={() => void assistant.pickFolder()}>
            <FolderOpen size={14} />Change folder
          </button>
        {:else}
          <button class="ws-open" type="button" onclick={() => void assistant.pickFolder()}>
            <span class="ws-open-ic"><FolderOpen size={18} /></span>
            <span class="ws-open-body">
              <span class="ws-open-t">Open folder…</span>
              <span class="ws-open-s">Pick any project on your disk</span>
            </span>
            <ChevronRight size={16} />
          </button>
        {/if}

        {#if recents.length > 0}
          <div class="hf-divider"></div>
          <div class="hf-feed-label">Recent folders</div>
          <div class="ws-recents">
            {#each recents.slice(0, 5) as r (r)}
              <div class="ws-recent-row">
                <button class="ws-recent" type="button" onclick={() => void assistant.setRoot(r)} use:tooltip={r}>
                  <span class="ws-recent-ic"><Folder size={13} /></span>
                  <span class="ws-recent-t">{leafName(r)}</span>
                  <span class="ws-recent-meta mono">{shortPath(r)}</span>
                </button>
                <button class="ws-recent-x" type="button" use:tooltip={"Forget"} onclick={() => void assistant.removeRecentRoot(r)}><X size={11} /></button>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Jump back in -->
      <div class="hf-card">
        <div class="l3-card-head">
          <span class="ci"><MessageSquare size={14} /></span>
          <span class="t">Jump back in</span>
          <span class="badge ok">{recentChats.length} saved</span>
        </div>
        <div class="hf-chats">
          {#if recentChats.length === 0}
            <div class="hf-empty">No conversations yet — start one from the composer.</div>
          {:else}
            {#each recentChats as c (c.id)}
              <button class="hf-chat" onclick={() => resume(c.id)} use:tooltip={`${c.title} · ${c.model}`}>
                <span class="hf-chat-ico"><MessageSquare size={13} /></span>
                <span class="hf-chat-body">
                  <span class="hf-chat-t">{c.title}</span>
                  <span class="hf-chat-s">{c.messageCount} msg</span>
                </span>
                <span class="hf-chat-w">{fmtAgo(c.updatedAt)}</span>
              </button>
            {/each}
          {/if}
        </div>
        <button class="hf-allchats" onclick={() => go()}>
          <History size={13} />Browse all conversations<ChevronRight size={13} />
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .hf { height: 100%; overflow: hidden; padding: 32px 40px; }
  .hf-inner { height: 100%; max-width: 1080px; margin: 0 auto; display: flex; flex-direction: column; gap: 22px; min-height: 0; }

  .hf-hero { display: flex; align-items: flex-end; justify-content: space-between; gap: 24px; flex: none; }
  .hf-hero-l { min-width: 0; }
  .hf-eyebrow { display: inline-flex; align-items: center; gap: 8px; font-family: var(--font-mono); font-size: 11px; font-weight: 500; letter-spacing: 0.04em; text-transform: uppercase; color: var(--fg-subtle); margin-bottom: 11px; }
  .hf-eyebrow .led { width: 7px; height: 7px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 8px color-mix(in oklab, var(--accent) 60%, transparent); }
  @media (prefers-reduced-motion: no-preference) {
    .hf-eyebrow .led { animation: led-breathe 2.6s ease-in-out infinite; }
  }
  @keyframes led-breathe { 0%,100% { opacity: 1; } 50% { opacity: 0.5; } }
  .hf-title { margin: 2px 0 0; font-size: 33px; font-weight: 700; letter-spacing: -0.03em; line-height: 1.08; color: var(--fg); }
  .hf-title .emer { color: var(--accent); }
  .hf-sub { margin: 12px 0 0; color: var(--fg-muted); font-size: var(--fs-lg); }

  .hf-hero-actions { display: flex; align-items: center; gap: 9px; flex-shrink: 0; }
  .hf-btn { display: inline-flex; align-items: center; gap: 8px; height: 40px; padding: 0 16px; border-radius: var(--radius-lg); font: inherit; font-weight: 600; font-size: var(--fs-sm); cursor: pointer; border: 1px solid var(--border); background: var(--surface); color: var(--fg); transition: background 140ms, border-color 140ms; }
  .hf-btn:hover { background: var(--surface-hover); border-color: var(--border-strong); }
  .hf-btn.primary { background: var(--accent); color: var(--accent-fg); border-color: transparent; box-shadow: 0 4px 16px -4px color-mix(in oklab, var(--accent) 55%, transparent); }
  .hf-btn.primary:hover { background: var(--accent-hover); }

  .dash-ask {
    flex: none; display: flex; align-items: center; gap: 13px; width: 100%; text-align: left;
    height: 62px; padding: 0 14px 0 16px;
    background: var(--bg-inset); border: 1px solid var(--border-strong); border-radius: 16px;
    cursor: text; transition: border-color 140ms ease, box-shadow 140ms ease;
  }
  .dash-ask:hover { border-color: var(--ghost-border); box-shadow: 0 0 0 3px var(--ring); }
  .da-glyph { width: 32px; height: 32px; border-radius: 9px; background: var(--accent-soft); color: var(--accent); display: grid; place-items: center; flex-shrink: 0; }
  .da-text { flex: 1; color: var(--fg-subtle); font-size: var(--fs-lg); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .da-model { display: inline-flex; align-items: center; gap: 7px; flex-shrink: 0; height: 32px; padding: 0 12px; border-radius: 9px; background: var(--surface); border: 1px solid var(--border); color: var(--fg); font-size: var(--fs-sm); font-weight: 600; }
  .da-model .da-dot { width: 7px; height: 7px; border-radius: 999px; background: var(--accent); flex-shrink: 0; }
  .da-send { width: 36px; height: 32px; border-radius: 8px; background: var(--accent); color: var(--accent-fg); display: grid; place-items: center; flex-shrink: 0; box-shadow: 0 2px 10px -2px color-mix(in oklab, var(--accent) 50%, transparent); }

  /* CLI update banner — slim accent strip under the Ask bar. */
  .dash-cli {
    flex: none; display: flex; align-items: center; gap: 13px;
    padding: 11px 12px 11px 14px; border-radius: 14px;
    background: color-mix(in oklab, var(--accent) 9%, var(--surface));
    border: 1px solid color-mix(in oklab, var(--accent) 32%, var(--border));
    animation: dash-cli-in 240ms var(--ease-page, ease);
  }
  .dash-cli[data-tone="warn"] {
    background: color-mix(in oklab, var(--warn) 9%, var(--surface));
    border-color: color-mix(in oklab, var(--warn) 32%, var(--border));
  }
  .dash-cli[data-tone="danger"] {
    background: color-mix(in oklab, var(--danger) 9%, var(--surface));
    border-color: color-mix(in oklab, var(--danger) 32%, var(--border));
  }
  @keyframes dash-cli-in { from { opacity: 0; transform: translateY(-4px); } to { opacity: 1; transform: translateY(0); } }
  @media (prefers-reduced-motion: reduce) { .dash-cli { animation: none; } }
  .dash-cli .dc-ic {
    width: 32px; height: 32px; border-radius: 9px; flex-shrink: 0;
    display: grid; place-items: center;
    background: var(--accent-soft); color: var(--accent);
  }
  .dash-cli[data-tone="warn"] .dc-ic { background: color-mix(in oklab, var(--warn) 14%, transparent); color: var(--warn); }
  .dash-cli[data-tone="danger"] .dc-ic { background: color-mix(in oklab, var(--danger) 14%, transparent); color: var(--danger); }
  .dash-cli .dc-body { min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .dash-cli .dc-t { font-size: var(--fs-sm); font-weight: 650; color: var(--fg); }
  .dash-cli .dc-s { font-size: var(--fs-xs); color: var(--fg-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dash-cli .dc-s code { font-family: var(--font-mono); color: var(--fg-2); }
  .dash-cli[data-tone="warn"] .dc-detail { color: var(--warn); }
  .dash-cli[data-tone="danger"] .dc-detail { color: var(--danger); }
  .dash-cli .dc-go {
    margin-left: auto; flex-shrink: 0; display: inline-flex; align-items: center; gap: 6px;
    height: 30px; padding: 0 12px; border-radius: 8px; font: inherit; font-size: var(--fs-xs); font-weight: 650;
    cursor: pointer; background: var(--accent); color: var(--accent-fg); border: 1px solid transparent;
    transition: background 130ms ease, opacity 130ms ease;
  }
  .dash-cli .dc-go:hover:not(:disabled) { background: var(--accent-hover); }
  .dash-cli .dc-go:disabled { opacity: 0.7; cursor: default; }
  :global(.dash-cli .dc-go .dc-spin) { animation: dc-spin 0.8s linear infinite; }
  @keyframes dc-spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { :global(.dash-cli .dc-go .dc-spin) { animation: none; } }
  .dash-cli .dc-copy {
    flex-shrink: 0; display: inline-flex; align-items: center; justify-content: center;
    width: 30px; height: 30px; border-radius: 8px; font: inherit;
    cursor: pointer; background: var(--surface-hover); color: var(--fg-2); border: 1px solid var(--border);
    transition: background 130ms ease, color 130ms ease;
  }
  .dash-cli .dc-copy:hover { color: var(--fg); }
  .dash-cli .dc-copy.done { color: var(--ok, var(--accent)); }
  .dash-cli .dc-x {
    flex-shrink: 0; display: inline-flex; align-items: center; justify-content: center;
    width: 28px; height: 28px; border-radius: 8px; cursor: pointer;
    background: transparent; border: 0; color: var(--fg-faint);
    transition: color 120ms ease, background 120ms ease;
  }
  .dash-cli .dc-x:hover { color: var(--fg); background: var(--surface-hover); }

  .hf-grid { flex: 1; min-height: 0; display: grid; grid-template-columns: 1.25fr 1fr; gap: 20px; }
  .hf-card { min-height: 0; overflow: hidden; display: flex; flex-direction: column; padding: 24px 24px 18px; border-radius: 18px; background: var(--surface); border: 1px solid var(--border); box-shadow: inset 0 1px 0 color-mix(in oklch, white 2.5%, transparent); }
  .l3-card-head { display: flex; align-items: center; gap: 9px; flex: none; margin-bottom: 20px; }
  .l3-card-head .ci { width: 26px; height: 26px; border-radius: 8px; display: grid; place-items: center; background: var(--accent-soft); color: var(--accent); }
  .l3-card-head .t { font-weight: 650; font-size: var(--fs-md); color: var(--fg); }
  .l3-card-head .badge { margin-left: auto; display: inline-flex; align-items: center; gap: 5px; font-size: 10.5px; font-weight: 600; padding: 3px 9px; border-radius: 999px; }
  .badge.ok { background: var(--ok-soft); color: var(--ok); }
  .badge.info { background: var(--info-soft); color: var(--info); }

  /* Workspace current-folder block */
  .ws-current { display: flex; align-items: center; gap: 12px; flex: none; margin-bottom: 14px; }
  .ws-cur-ic { width: 38px; height: 38px; border-radius: 11px; background: var(--accent-soft); color: var(--accent); display: grid; place-items: center; flex-shrink: 0; }
  .ws-cur-body { min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .ws-cur-name { font-size: var(--fs-md); font-weight: 650; color: var(--fg); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ws-cur-path { font-size: var(--fs-xs); color: var(--fg-faint); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .mono { font-family: var(--font-mono, monospace); }
  .ws-change { align-self: flex-start; flex: none; }

  .ws-open {
    display: grid; grid-template-columns: 40px 1fr auto; align-items: center; gap: 13px;
    width: 100%; padding: 15px 16px; text-align: left; cursor: pointer; color: var(--fg);
    background: var(--accent-soft); border: 1px solid var(--ghost-border); border-radius: var(--radius-lg);
    font: inherit; flex: none;
    transition: background 140ms ease, transform 140ms ease;
  }
  .ws-open:hover { background: color-mix(in oklab, var(--accent) 14%, transparent); transform: translateY(-1px); }
  .ws-open-ic { width: 40px; height: 40px; border-radius: 11px; display: grid; place-items: center; background: var(--accent-soft); color: var(--accent); }
  .ws-open-body { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .ws-open-t { font-size: var(--fs-md); font-weight: 600; }
  .ws-open-s { font-size: var(--fs-xs); color: var(--fg-muted); }

  .hf-divider { height: 1px; background: var(--border); margin: 18px 0 0; flex: none; }
  .hf-feed-label { font-size: 10px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.08em; color: var(--fg-faint); margin: 16px 2px 4px; flex: none; }

  .ws-recents { flex: 1; min-height: 0; overflow: auto; display: flex; flex-direction: column; gap: 2px; }
  .ws-recent-row { display: flex; align-items: stretch; gap: 2px; border-radius: 10px; }
  .ws-recent { flex: 1; min-width: 0; display: grid; grid-template-columns: 24px auto 1fr; align-items: center; gap: 11px; padding: 9px 11px; text-align: left; cursor: pointer; color: var(--fg); background: transparent; border: 1px solid transparent; border-radius: 10px; font: inherit; transition: background 130ms ease, border-color 130ms ease; }
  .ws-recent:hover { background: var(--surface-hover); border-color: var(--border); }
  .ws-recent-ic { width: 24px; height: 24px; border-radius: 7px; display: grid; place-items: center; background: var(--bg-elev-2); color: var(--fg-muted); }
  .ws-recent:hover .ws-recent-ic { color: var(--accent); }
  .ws-recent-t { font-size: var(--fs-sm); font-weight: 500; color: var(--fg); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ws-recent-meta { font-size: var(--fs-xs); color: var(--fg-faint); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ws-recent-x { background: transparent; border: 0; padding: 0 8px; color: var(--fg-faint); cursor: pointer; opacity: 0; display: inline-flex; align-items: center; border-radius: 8px; transition: opacity 120ms, color 120ms; }
  .ws-recent-row:hover .ws-recent-x { opacity: 1; }
  .ws-recent-x:hover { color: var(--danger); }

  .hf-empty { color: var(--fg-subtle); font-size: var(--fs-sm); padding: 8px 6px; }

  .hf-chats { flex: 1; min-height: 0; display: flex; flex-direction: column; gap: 2px; overflow: auto; }
  .hf-chat { display: grid; grid-template-columns: 28px 1fr auto; align-items: center; gap: 11px; padding: 11px 8px; border-radius: 9px; background: transparent; border: 0; width: 100%; text-align: left; font: inherit; cursor: pointer; transition: background 120ms; }
  .hf-chat:hover { background: var(--surface-hover); }
  .hf-chat-ico { width: 28px; height: 28px; border-radius: 8px; background: var(--accent-soft); color: var(--accent); display: grid; place-items: center; }
  .hf-chat-body { min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .hf-chat-t { font-size: var(--fs-sm); font-weight: 500; color: var(--fg); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hf-chat-s { font-size: 10.5px; color: var(--fg-faint); }
  .hf-chat-w { font-family: var(--font-mono); font-size: 10.5px; color: var(--fg-subtle); }

  .hf-allchats { display: inline-flex; align-items: center; gap: 8px; align-self: flex-start; margin-top: 16px; padding: 6px 2px; flex: none; border: 0; background: transparent; color: var(--fg-muted); font: inherit; font-size: var(--fs-sm); font-weight: 600; cursor: pointer; transition: color 130ms, gap 130ms; }
  .hf-allchats:hover { color: var(--accent); gap: 10px; }
  .hf-allchats :global(svg:first-child) { color: var(--accent); }
</style>

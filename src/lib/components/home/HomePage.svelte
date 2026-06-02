<script lang="ts">
  import {
    ArrowDownUp, MessageSquare, Sparkles, Send, RefreshCcw,
    GitBranch, Upload, ChevronRight, History,
  } from "lucide-svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import { connection, type ActivityKind, type ActivityRow } from "$lib/state/connection.svelte";
  import { workspace, type WorkspaceId } from "$lib/state/workspace.svelte";
  import { tooltip } from "$lib/actions/tooltip";

  // ── Real sync state (no fabricated absolutes — only fields the engine exposes) ──
  const server = $derived(connection.selected);
  const connected = $derived(server != null);
  const conflicts = $derived(connection.conflicts.length);
  const localAhead = $derived(connection.dirtyEdits.size);
  const queued = $derived(connection.status?.pending ?? connection.pendingReuploads.length);
  const watches = $derived(connection.status?.watches ?? 0);
  const flagged = $derived(conflicts + localAhead + queued);
  const scanAt = $derived(connection.lastScanAt);

  type HeroState = "offline" | "attention" | "clear";
  const heroState = $derived<HeroState>(!connected ? "offline" : flagged > 0 ? "attention" : "clear");
  const pct = $derived(connected ? Math.max(0, 100 - flagged) : 0);

  const pillCls = $derived.by<"ok" | "warn" | "danger" | "info" | "muted">(() => {
    switch (connection.pill) {
      case "Connected": return "ok";
      case "Syncing": return "info";
      case "Conflict": return "danger";
      case "Lock-blocked": return "warn";
      case "Sync error": return "danger";
      default: return "muted";
    }
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

  function dotTone(k: ActivityKind): "ok" | "warn" | "info" {
    if (k === "conflict" || k === "error" || k === "block" || k === "drift") return "warn";
    if (k === "system") return "info";
    return "ok";
  }
  function badgeLabel(r: ActivityRow): string {
    const a = r.action || r.kind;
    return a.charAt(0).toUpperCase() + a.slice(1);
  }
  const feed = $derived(connection.activityFeed.slice(0, 4));

  // ── Recents — real conversations w/ a real back-and-forth ──
  const recentChats = $derived(
    assistant.conversations
      .filter((c) => c.id !== assistant.currentConvoId)
      .filter((c) => c.messageCount >= 3)
      .slice(0, 6),
  );

  // ── Navigation (folds redesign destinations that don't exist yet onto live ones) ──
  function go(target: string, draft?: string) {
    if (draft != null) assistant.composerDraft = draft;
    const remap: Record<string, WorkspaceId> = { conflicts: "sync", history: "chat" };
    workspace.setActive((remap[target] ?? target) as WorkspaceId);
  }
  function resume(id: string) {
    void assistant.openTab(id);
    workspace.setActive("chat");
  }
</script>

<div class="hf">
  <div class="hf-inner">
    <!-- Honest status hero -->
    <section class="hf-hero" data-state={heroState}>
      <div class="hf-hero-l">
        <div class="hf-eyebrow"><span class="led"></span>{connected ? `Watching · ${server?.name}` : "Not connected"}</div>
        {#if heroState === "offline"}
          <h1 class="hf-title">No server connected</h1>
          <p class="hf-sub">Connect an SFTP server to start mirroring and drift detection.</p>
        {:else if heroState === "attention"}
          <h1 class="hf-title">
            {#if conflicts > 0}{conflicts} file{conflicts === 1 ? "" : "s"} need <span class="amber">your review</span>
            {:else}{flagged} change{flagged === 1 ? "" : "s"} <span class="amber">pending</span>{/if}
          </h1>
          <p class="hf-sub"><span class="mono">{flagged} flagged</span> · last scan {fmtAgo(scanAt)}</p>
        {:else}
          <h1 class="hf-title">{server?.name} is <span class="emer">fully in sync</span></h1>
          <p class="hf-sub"><span class="mono">0 drift</span> · {watches} folders watched · last scan {fmtAgo(scanAt)}</p>
        {/if}
      </div>
      <div class="hf-hero-actions">
        {#if heroState === "offline"}
          <button class="hf-btn primary" onclick={() => go("sync")}><RefreshCcw size={15} />Connect server</button>
        {:else if heroState === "attention"}
          {#if conflicts > 0}
            <button class="hf-btn primary" onclick={() => go("conflicts")}><GitBranch size={15} />Resolve {conflicts} conflict{conflicts === 1 ? "" : "s"}</button>
          {/if}
          {#if localAhead > 0}
            <button class="hf-btn" onclick={() => go("sync")}><Upload size={15} />Push {localAhead}</button>
          {/if}
          <button class="hf-btn icon" use:tooltip={"Open Sync"} aria-label="Open Sync" onclick={() => go("sync")}><RefreshCcw size={16} /></button>
        {:else}
          <button class="hf-btn" onclick={() => go("sync")}><RefreshCcw size={15} />Rescan</button>
          <button class="hf-btn primary" onclick={() => go("chat")}><Sparkles size={15} />New chat</button>
        {/if}
      </div>
    </section>

    <!-- Signature feature — Ask Claude -->
    <button class="dash-ask" onclick={() => go("chat")}>
      <span class="da-glyph"><Sparkles size={16} /></span>
      <span class="da-text">{assistant.composerDraft || (server ? `Ask Claude about ${server.name}…` : "Ask Claude anything…")}</span>
      <span class="da-send"><Send size={14} /></span>
    </button>

    <div class="hf-grid">
      <!-- Sync health (activity folded in) -->
      <div class="hf-card">
        <div class="l3-card-head">
          <span class="ci"><ArrowDownUp size={14} /></span>
          <span class="t">Sync health</span>
          <span class="badge {pillCls}">{connection.pill}</span>
        </div>

        <div class="hf-metrics">
          <button class="hf-metric" onclick={() => go("conflicts")}>
            <div class="mv" class:danger={conflicts > 0} class:ok={conflicts === 0}>{conflicts}</div>
            <div class="ml">Conflicts</div>
          </button>
          <button class="hf-metric" onclick={() => go("sync")}>
            <div class="mv" class:warn={localAhead > 0} class:ok={localAhead === 0}>{localAhead}</div>
            <div class="ml">Local ahead</div>
          </button>
          <button class="hf-metric" onclick={() => go("sync")}>
            <div class="mv" class:warn={queued > 0} class:ok={queued === 0}>{queued}</div>
            <div class="ml">Queued</div>
          </button>
        </div>

        <div class="hf-meter">
          <div class="bar"><i style="width: {pct}%"></i></div>
          <span class="pct">{pct}%</span>
        </div>

        <div class="hf-divider"></div>
        <div class="hf-feed-label">Latest activity</div>
        <div class="hf-feed">
          {#if feed.length === 0}
            <div class="hf-empty">No activity yet.</div>
          {:else}
            {#each feed as f, i (i)}
              <div class="l3-fi">
                <span class="fd {dotTone(f.kind)}"></span>
                <span class="ft"><b>{badgeLabel(f)}</b> {f.rel_path ?? f.file ?? f.resource}</span>
                <span class="fw">{fmtAgo(f.at)}</span>
              </div>
            {/each}
          {/if}
        </div>
        <button class="hf-allchats" onclick={() => go("sync")}>
          <ArrowDownUp size={13} />Open Sync dashboard<ChevronRight size={13} />
        </button>
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
        <button class="hf-allchats" onclick={() => go("history")}>
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
  .hf-eyebrow .led { width: 7px; height: 7px; border-radius: 50%; background: var(--fg-faint); }
  .hf-hero[data-state="attention"] .hf-eyebrow .led { background: var(--warn); box-shadow: 0 0 8px color-mix(in oklch, var(--warn) 60%, transparent); }
  .hf-hero[data-state="clear"] .hf-eyebrow .led { background: var(--ok); box-shadow: 0 0 8px color-mix(in oklch, var(--ok) 60%, transparent); }
  @media (prefers-reduced-motion: no-preference) {
    .hf-hero[data-state="clear"] .hf-eyebrow .led { animation: led-breathe 2.6s ease-in-out infinite; }
  }
  @keyframes led-breathe { 0%,100% { opacity: 1; } 50% { opacity: 0.5; } }
  .hf-title { margin: 2px 0 0; font-size: 33px; font-weight: 700; letter-spacing: -0.03em; line-height: 1.08; color: var(--fg); }
  .hf-title .amber { color: var(--warn); }
  .hf-title .emer { color: var(--accent); }
  .hf-sub { margin: 12px 0 0; color: var(--fg-muted); font-size: var(--fs-lg); }
  .hf-sub .mono { font-family: var(--font-mono); color: var(--fg-2); }

  .hf-hero-actions { display: flex; align-items: center; gap: 9px; flex-shrink: 0; }
  .hf-btn { display: inline-flex; align-items: center; gap: 8px; height: 40px; padding: 0 16px; border-radius: var(--radius-lg); font: inherit; font-weight: 600; font-size: var(--fs-sm); cursor: pointer; border: 1px solid var(--border); background: var(--surface); color: var(--fg); transition: background 140ms, border-color 140ms; }
  .hf-btn:hover { background: var(--surface-hover); border-color: var(--border-strong); }
  .hf-btn.icon { width: 40px; padding: 0; justify-content: center; color: var(--fg-muted); }
  .hf-btn.primary { background: var(--accent); color: var(--accent-fg); border-color: transparent; box-shadow: 0 4px 16px -4px color-mix(in oklch, var(--accent) 55%, transparent); }
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
  .da-send { width: 36px; height: 32px; border-radius: 8px; background: var(--accent); color: var(--accent-fg); display: grid; place-items: center; flex-shrink: 0; box-shadow: 0 2px 10px -2px color-mix(in oklch, var(--accent) 50%, transparent); }

  .hf-grid { flex: 1; min-height: 0; display: grid; grid-template-columns: 1.55fr 1fr; gap: 20px; }
  .hf-card { min-height: 0; overflow: hidden; display: flex; flex-direction: column; padding: 24px 24px 18px; border-radius: 18px; background: var(--surface); border: 1px solid var(--border); box-shadow: inset 0 1px 0 color-mix(in oklch, white 2.5%, transparent); }
  .l3-card-head { display: flex; align-items: center; gap: 9px; flex: none; margin-bottom: 20px; }
  .l3-card-head .ci { width: 26px; height: 26px; border-radius: 8px; display: grid; place-items: center; background: var(--accent-soft); color: var(--accent); }
  .l3-card-head .t { font-weight: 650; font-size: var(--fs-md); color: var(--fg); }
  .l3-card-head .badge { margin-left: auto; font-size: 10.5px; font-weight: 600; padding: 3px 9px; border-radius: 999px; }
  .badge.ok { background: var(--ok-soft); color: var(--ok); }
  .badge.warn { background: var(--warn-soft); color: var(--warn); }
  .badge.danger { background: var(--danger-soft); color: var(--danger); }
  .badge.info { background: var(--info-soft); color: var(--info); }
  .badge.muted { background: var(--bg-elev-2); color: var(--fg-muted); }

  .hf-metrics { display: flex; align-items: stretch; gap: 0; flex: none; }
  .hf-metric { flex: 1; text-align: left; background: transparent; border: 0; padding: 2px 4px 2px 0; cursor: pointer; font: inherit; transition: opacity 140ms; }
  .hf-metric + .hf-metric { border-left: 1px solid var(--border); padding-left: 20px; }
  .hf-metric:hover { opacity: 0.72; }
  .hf-metric .mv { font-family: var(--font-mono); font-size: 30px; font-weight: 680; line-height: 1; color: var(--fg); }
  .hf-metric .mv.warn { color: var(--warn); }
  .hf-metric .mv.danger { color: var(--danger); }
  .hf-metric .mv.ok { color: var(--ok); }
  .hf-metric .ml { font-size: 11.5px; color: var(--fg-subtle); margin-top: 9px; }

  .hf-meter { display: flex; align-items: center; gap: 13px; margin-top: 22px; flex: none; }
  .hf-meter .bar { flex: 1; height: 8px; border-radius: 999px; background: var(--bg-elev-3); overflow: hidden; }
  .hf-meter .bar i { display: block; height: 100%; background: var(--accent); border-radius: 999px; transition: width 400ms var(--ease-page); }
  .hf-meter .pct { font-family: var(--font-mono); font-size: var(--fs-sm); font-weight: 600; color: var(--fg); min-width: 38px; text-align: right; }

  .hf-divider { height: 1px; background: var(--border); margin: 20px 0 0; flex: none; }
  .hf-feed-label { font-size: 10px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.08em; color: var(--fg-faint); margin: 16px 2px 4px; flex: none; }
  .hf-feed { flex: 1; min-height: 0; overflow: hidden; display: flex; flex-direction: column; gap: 8px; padding: 6px 0 0; }
  .l3-fi { display: flex; align-items: center; gap: 10px; padding: 8px 6px; font-size: var(--fs-sm); }
  .l3-fi .fd { width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0; }
  .l3-fi .fd.ok { background: var(--ok); }
  .l3-fi .fd.info { background: var(--info); }
  .l3-fi .fd.warn { background: var(--warn); }
  .l3-fi .ft { flex: 1; color: var(--fg-2); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .l3-fi .ft b { color: var(--fg); font-weight: 600; }
  .l3-fi .fw { font-size: 10.5px; color: var(--fg-faint); font-family: var(--font-mono); }
  .hf-empty { color: var(--fg-subtle); font-size: var(--fs-sm); padding: 8px 6px; }

  .hf-chats { flex: 1; min-height: 0; display: flex; flex-direction: column; gap: 2px; overflow: hidden; }
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

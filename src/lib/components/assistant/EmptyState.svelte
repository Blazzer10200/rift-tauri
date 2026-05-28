<script lang="ts">
  import {
    Sparkles, FolderOpen, FolderTree, Search, FileText,
    ExternalLink, X, Folder, ChevronRight,
  } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { connection } from "../../state/connection.svelte";

  import { tooltip } from "$lib/actions/tooltip";
  // Optional tabId — when set (split-pane), suggestion clicks write into THIS
  // tab's draft instead of the focused-pane shim. Falls back to focused tab
  // when omitted (single-pane / pre-pane callers).
  let { needsAuth = false, tabId = null }: { needsAuth?: boolean; tabId?: string | null } = $props();
  const targetTab = $derived(tabId ? assistant.tabFor(tabId) : assistant.activeTab);

  type Suggestion = {
    icon: typeof FolderTree;
    title: string;
    prompt: string;
  };

  // Generic suggestions — work on any stack. Used as fallback when no
  // distinctive workspace marker is detected.
  const genericSuggestions: Suggestion[] = [
    {
      icon: FolderTree,
      title: "Map the project",
      prompt: "Give me a high-level tour of this project — entry points, key folders, and what each does.",
    },
    {
      icon: Search,
      title: "Find TODOs & FIXMEs",
      prompt: "Grep for all TODO, FIXME, and HACK comments. Group them by file and summarize what needs attention.",
    },
    {
      icon: FileText,
      title: "Explain a key file",
      prompt: "Pick the most important file in this project and walk me through what it does.",
    },
  ];

  // FiveM/RedM-aware suggestions — surfaced when `fxmanifest.lua` is detected
  // anywhere under the workspace root.
  const fivemSuggestions: Suggestion[] = [
    {
      icon: FolderTree,
      title: "List every event handler",
      prompt: "Scan every resource under this workspace. List all RegisterNetEvent / AddEventHandler / on() handlers grouped by resource, with file:line.",
    },
    {
      icon: Search,
      title: "Find missing dependencies",
      prompt: "Read every fxmanifest.lua. For each resource's declared dependencies, verify the dependency resource exists in this workspace. List any unresolved dependencies.",
    },
    {
      icon: FileText,
      title: "Show me the resource boot order",
      prompt: "Walk through how this server boots: server.cfg start order if present, fxmanifest dependencies, and any explicit ensure() calls. Diagram the load chain.",
    },
  ];

  function detectStack(files: readonly string[]): "fivem" | "generic" {
    for (const f of files) {
      const lower = f.toLowerCase();
      if (lower === "fxmanifest.lua" || lower.endsWith("/fxmanifest.lua")) return "fivem";
    }
    return "generic";
  }

  // Kick off a workspace file walk lazily once the empty-state renders.
  // Cheap (~ms) on typical FiveM resource folders; cached on the store.
  $effect(() => {
    if (assistant.workspace.current && assistant.workspaceFiles.length === 0) {
      void assistant.loadWorkspaceFiles();
    }
  });

  const stack = $derived(detectStack(assistant.workspaceFiles));
  const suggestions = $derived(stack === "fivem" ? fivemSuggestions : genericSuggestions);

  function pick(prompt: string) {
    if (targetTab) targetTab.draft = prompt;
    else assistant.composerDraft = prompt;
  }

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

  const server = $derived(connection.selected);
  const hasRoot = $derived(assistant.workspace.current != null);
  const hasSynced = $derived(server != null);
  // "Has any workspace context" — folder open OR synced server fallback active.
  // Suggestions only render when this is true; otherwise the screen focuses
  // entirely on the Open-folder CTA so users aren't confused about what's wired.
  const hasWorkspace = $derived(hasRoot || hasSynced);
  const recents = $derived(assistant.workspace.recent);

  // Resume-tiles — top 4 recent conversations excluding the current empty
  // tab. The titlebar already exposes the synced-server label, so this slot
  // earns its keep by being actionable: one click resumes a real conversation.
  const recentChats = $derived(
    assistant.conversations
      .filter((c) => c.id !== assistant.currentConvoId)
      .slice(0, 4),
  );

  function fmtAgo(ms: number): string {
    const diff = Date.now() - ms;
    const min = 60_000, hr = 60 * min, day = 24 * hr;
    if (diff < min) return "just now";
    if (diff < hr) return `${Math.floor(diff / min)}m ago`;
    if (diff < day) return `${Math.floor(diff / hr)}h ago`;
    if (diff < 7 * day) return `${Math.floor(diff / day)}d ago`;
    return new Date(ms).toLocaleDateString();
  }
</script>

<div class="empty" class:no-ws={!hasWorkspace}>
  {#if needsAuth}
    <div class="hero">
      <div class="glyph"><Sparkles size={26} /></div>
      <h2>Set up Claude</h2>
      <p>The Assistant needs the Claude Code CLI logged in, or an API key configured in Settings.</p>
    </div>
    <div class="auth-help">
      <a href="https://claude.com/download" target="_blank" rel="noreferrer">
        Download Claude Code <ExternalLink size={11}/>
      </a>
      <span class="dim">— then run <code>claude login</code> and hit refresh.</span>
      <p class="muted">Or open <strong>Settings → Assistant</strong> and paste an <code>sk-ant-api03-…</code> key.</p>
    </div>
  {:else}
    <!-- Hero — headline adapts to workspace state + whether there's history
         to resume. Subtitle suppressed when a workspace is already wired. -->
    <div class="hero">
      <div class="glyph">
        <span class="glyph-halo" aria-hidden="true"></span>
        <Sparkles size={22} />
      </div>
      {#if hasRoot || hasSynced}
        <h2>{recentChats.length > 0 ? "Pick up where you left off" : "What's on your mind?"}</h2>
      {:else}
        <h2>Open a folder to begin</h2>
        <p class="sub">Point Claude at any project on your disk — it'll read, list, and grep on demand.</p>
      {/if}
    </div>

    <!-- Workspace card — only when a folder is genuinely open. Synced-only
         state is already covered by the titlebar connection chip, so this slot
         goes to recent-chat tiles instead (see below). -->
    {#if hasRoot}
      <div class="ws-card active">
        <div class="ws-icon"><Folder size={16}/></div>
        <div class="ws-body">
          <div class="ws-title">{leafName(assistant.workspace.current!)}</div>
          <div class="ws-sub">{shortPath(assistant.workspace.current!)}</div>
        </div>
        <button
          class="ws-action"
          type="button"
          use:tooltip={"Close folder"}
          onclick={() => void assistant.clearRoot()}
        ><X size={13}/></button>
      </div>
    {:else if !hasSynced}
      <button
        class="ws-card primary"
        type="button"
        onclick={() => void assistant.pickFolder()}
      >
        <div class="ws-icon"><FolderOpen size={18}/></div>
        <div class="ws-body">
          <div class="ws-title">Open folder…</div>
          <div class="ws-sub">Pick any project on your disk</div>
        </div>
        <ChevronRight size={16} class="ws-chev"/>
      </button>

      {#if recents.length > 0}
        <div class="recents-block">
          <div class="block-label">Recent</div>
          <div class="recents">
            {#each recents.slice(0, 5) as r (r)}
              <div class="recent-row">
                <button
                  class="recent-open"
                  type="button"
                  onclick={() => void assistant.setRoot(r)}
                  use:tooltip={r}
                >
                  <Folder size={11}/>
                  <span class="recent-name">{leafName(r)}</span>
                  <span class="recent-path">{shortPath(r)}</span>
                </button>
                <button
                  class="recent-x"
                  type="button"
                  use:tooltip={"Forget"}
                  onclick={() => void assistant.removeRecentRoot(r)}
                ><X size={10}/></button>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    {/if}

    <!-- Resume tiles — surfaced when there's chat history AND a workspace is
         wired. Click resumes that tab; the current empty chat stays open in
         the background until the user types or closes it. -->
    {#if hasWorkspace && recentChats.length > 0}
      <div class="resume-block">
        <div class="block-label">Recent</div>
        <div class="chat-tiles">
          {#each recentChats as c (c.id)}
            <button
              class="chat-tile"
              type="button"
              data-model={c.model?.toLowerCase().includes("opus") ? "opus"
                : c.model?.toLowerCase().includes("haiku") ? "haiku"
                : "sonnet"}
              onclick={() => void assistant.openTab(c.id)}
              use:tooltip={c.title}
            >
              <span class="tile-title">{c.title}</span>
              <span class="tile-meta">
                <span class="tile-model-dot" aria-hidden="true"></span>
                <span class="tile-model">{c.model}</span>
                <span class="tile-dot">·</span>
                <span>{c.messageCount} msg</span>
                <span class="tile-time">{fmtAgo(c.updatedAt)}</span>
              </span>
            </button>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Suggestions only when a workspace is wired — keeps the empty/no-folder
         state focused on a single CTA. -->
    {#if hasWorkspace}
      <div class="suggestions-block">
        <div class="block-label">Try asking</div>
        <div class="suggestions">
          {#each suggestions as s (s.title)}
            <button class="card" type="button" onclick={() => pick(s.prompt)}>
              <span class="card-icon"><s.icon size={13}/></span>
              <span class="card-body">
                <span class="card-title">{s.title}</span>
                <span class="card-prompt">{s.prompt}</span>
              </span>
            </button>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .empty {
    flex: 1;
    display: flex; flex-direction: column;
    align-items: center;
    /* Anchor near the top instead of dead-center — the page felt half-loaded
       with a huge void above the hero. clamp keeps it tight on short windows
       and breathy on tall ones without going full top-aligned (which made the
       hero feel like a header). */
    justify-content: flex-start;
    padding: clamp(28px, 7vh, 88px) 20px 24px;
    min-height: 0;
    gap: 14px;
  }
  /* Stagger child entrance so the empty state feels composed top-down,
     not slammed in as one block. Uses shared `enter` keyframe (app.css). */
  .empty > * {
    animation: enter 320ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  .empty > :nth-child(1) { animation-delay: 0ms; }
  .empty > :nth-child(2) { animation-delay: 60ms; }
  .empty > :nth-child(3) { animation-delay: 100ms; }
  .empty > :nth-child(4) { animation-delay: 140ms; }
  .empty > :nth-child(5) { animation-delay: 180ms; }
  @media (prefers-reduced-motion: reduce) {
    .empty > * { animation: none; }
  }

  .hero {
    max-width: 520px;
    text-align: center;
    display: flex; flex-direction: column; align-items: center;
    gap: 4px;
  }
  .glyph {
    position: relative;
    width: 52px; height: 52px;
    margin-bottom: 8px;
    border-radius: 50%;
    display: flex; align-items: center; justify-content: center;
    background: var(--accent-soft);
    color: var(--accent);
    box-shadow:
      0 0 0 1px color-mix(in oklch, var(--accent) 30%, transparent),
      0 18px 40px color-mix(in oklch, var(--accent) 18%, transparent);
    isolation: isolate;
  }
  /* Conic-gradient halo behind the glyph — rotates slowly + softly blurred.
     Lifts the empty state out of "stock" feel without screaming for attention. */
  .glyph-halo {
    position: absolute;
    inset: -10px;
    border-radius: 50%;
    background: conic-gradient(
      from 0deg,
      color-mix(in oklch, var(--accent) 75%, transparent),
      transparent 25%,
      color-mix(in oklch, var(--accent) 55%, transparent) 50%,
      transparent 75%,
      color-mix(in oklch, var(--accent) 75%, transparent)
    );
    filter: blur(10px);
    opacity: 0.55;
    z-index: -1;
    pointer-events: none;
  }
  @media (prefers-reduced-motion: no-preference) {
    .glyph { animation: glyph-breathe 4.2s ease-in-out infinite; }
    .glyph-halo { animation: halo-spin 9s linear infinite; }
  }
  @keyframes glyph-breathe {
    0%, 100% { transform: scale(1); }
    50%      { transform: scale(1.04); }
  }
  @keyframes halo-spin {
    to { transform: rotate(360deg); }
  }
  .hero h2 {
    margin: 0;
    font-size: var(--fs-xl);
    font-weight: 600;
    color: var(--fg);
    letter-spacing: -0.01em;
  }
  .hero .sub {
    margin: 0;
    color: var(--fg-muted);
    line-height: 1.5;
    font-size: var(--fs-sm);
    max-width: 440px;
  }
  /* ── Workspace card — single focal element ─────────────────────────────── */
  .ws-card {
    display: grid;
    grid-template-columns: 36px 1fr auto;
    align-items: center;
    gap: 12px;
    width: 100%;
    max-width: 520px;
    padding: 14px 14px 14px 16px;
    border-radius: 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    text-align: left;
    font: inherit;
    color: var(--fg);
  }
  .ws-card.primary:active { transform: translateY(0) scale(0.985); }
  .ws-card.primary {
    cursor: pointer;
    background: linear-gradient(180deg,
      color-mix(in oklch, var(--accent) 16%, var(--surface)),
      color-mix(in oklch, var(--accent) 6%, var(--surface)));
    border-color: color-mix(in oklch, var(--accent) 45%, var(--border));
    transition: transform 140ms ease-out, box-shadow 140ms ease-out, border-color 140ms ease-out;
  }
  .ws-card.primary:hover {
    transform: translateY(-1px);
    box-shadow: 0 10px 28px color-mix(in oklch, var(--accent) 22%, transparent);
    border-color: var(--accent);
  }
  .ws-card.active {
    border-color: color-mix(in oklch, var(--accent) 40%, var(--border));
    background: color-mix(in oklch, var(--accent) 6%, var(--surface));
  }
  .ws-icon {
    width: 36px; height: 36px;
    border-radius: 10px;
    display: flex; align-items: center; justify-content: center;
    background: color-mix(in oklch, var(--accent) 18%, transparent);
    color: var(--accent);
  }
  .ws-body { min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .ws-title {
    font-size: var(--fs-md);
    font-weight: 600;
    color: var(--fg);
    line-height: 1.2;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .ws-sub {
    font-size: var(--fs-xs);
    color: var(--fg-muted);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  :global(.ws-card.primary .ws-chev) {
    color: var(--accent);
    opacity: 0.7;
    transition: transform 140ms;
  }
  .ws-card.primary:hover :global(.ws-chev) { transform: translateX(2px); opacity: 1; }

  .ws-action {
    background: transparent;
    border: 1px solid transparent;
    border-radius: 8px;
    padding: 6px;
    color: var(--fg-faint);
    cursor: pointer;
    display: inline-flex;
    transition: color 120ms, background 120ms, border-color 120ms;
  }
  .ws-action:hover {
    color: var(--danger);
    background: var(--surface-hover);
  }
  /* ── Recents — quiet, compact list under the primary CTA ───────────────── */
  .recents-block {
    width: 100%;
    max-width: 520px;
    display: flex; flex-direction: column;
    gap: 6px;
  }
  .block-label {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-faint);
    padding: 0 4px;
  }
  .block-label::after {
    content: "";
    flex: 1;
    height: 1px;
    background: linear-gradient(to right,
      color-mix(in oklch, var(--border) 80%, transparent),
      transparent);
  }
  .recents { display: flex; flex-direction: column; gap: 3px; }
  .recent-row {
    display: flex;
    align-items: stretch;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    overflow: hidden;
    transition: background 120ms, border-color 120ms;
  }
  .recent-row:hover {
    background: var(--surface);
    border-color: var(--border);
  }
  .recent-open {
    flex: 1;
    display: grid;
    grid-template-columns: 14px auto 1fr;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    background: transparent;
    border: 0;
    color: var(--fg-2);
    cursor: pointer;
    text-align: left;
    font: inherit;
    min-width: 0;
  }
  .recent-name {
    font-size: var(--fs-sm);
    font-weight: 500;
    color: var(--fg-2);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .recent-path {
    font-size: var(--fs-xs);
    color: var(--fg-faint);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .recent-x {
    background: transparent;
    border: 0;
    padding: 0 10px;
    color: var(--fg-faint);
    cursor: pointer;
    opacity: 0;
    transition: opacity 120ms, color 120ms;
    display: inline-flex; align-items: center;
  }
  .recent-row:hover .recent-x { opacity: 1; }
  .recent-x:hover { color: var(--danger); }

  /* ── Resume tiles — recent conversations ───────────────────────────────── */
  .resume-block {
    width: 100%;
    max-width: 520px;
    display: flex; flex-direction: column;
    gap: 8px;
  }
  .chat-tiles {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 6px;
  }
  .chat-tile {
    display: flex; flex-direction: column;
    gap: 4px;
    padding: 9px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--fg);
    cursor: pointer;
    text-align: left;
    font: inherit;
    min-width: 0;
    transition: background 140ms, border-color 140ms, transform 140ms, box-shadow 140ms;
  }
  .chat-tile:hover {
    background: var(--surface-hover);
    border-color: color-mix(in oklch, var(--accent) 35%, var(--border));
    transform: translateY(-1px);
    box-shadow: 0 8px 22px color-mix(in oklch, var(--accent) 14%, transparent);
  }
  .chat-tile:active { transform: translateY(0) scale(0.99); }
  .chat-tile:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px var(--ring);
  }
  .tile-title {
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--fg);
    line-height: 1.3;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .tile-meta {
    display: flex; align-items: center; gap: 5px;
    font-size: 10.5px;
    color: var(--fg-faint);
    overflow: hidden;
  }
  .tile-model { text-transform: capitalize; font-weight: 500; color: var(--fg-muted); }
  .tile-dot { opacity: 0.6; }
  .tile-time {
    margin-left: auto;
    padding: 1px 6px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--bg-elev-2) 60%, transparent);
    color: var(--fg-muted);
  }
  /* Per-model identity dot — same colors as the Composer model picker so
     the model-name reads at a glance, not just a string. */
  .tile-model-dot {
    width: 6px; height: 6px;
    border-radius: 999px;
    background: var(--tile-model-color, var(--fg-muted));
    box-shadow: 0 0 0 2px color-mix(in oklch, var(--tile-model-color, var(--fg-muted)) 14%, transparent),
                0 0 6px color-mix(in oklch, var(--tile-model-color, var(--fg-muted)) 45%, transparent);
    flex-shrink: 0;
  }
  .chat-tile[data-model="sonnet"] { --tile-model-color: oklch(0.74 0.13 230); }
  .chat-tile[data-model="opus"]   { --tile-model-color: oklch(0.70 0.18 295); }
  .chat-tile[data-model="haiku"]  { --tile-model-color: oklch(0.78 0.14 180); }

  /* ── Suggestions ───────────────────────────────────────────────────────── */
  .suggestions-block {
    width: 100%;
    max-width: 520px;
    display: flex; flex-direction: column;
    gap: 8px;
    margin-top: 4px;
  }
  .suggestions {
    display: flex; flex-direction: column;
    gap: 6px;
  }
  .card {
    display: grid;
    grid-template-columns: 22px 1fr;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--fg-2);
    cursor: pointer;
    text-align: left;
    font: inherit;
    transition: background 120ms, border-color 120ms, transform 120ms;
  }
  .card:hover {
    background: var(--surface-hover);
    border-color: color-mix(in oklch, var(--accent) 35%, var(--border));
    transform: translateX(2px);
  }
  .card:active { transform: translateX(2px) scale(0.985); }
  .card-icon {
    width: 22px; height: 22px;
    display: flex; align-items: center; justify-content: center;
    border-radius: 6px;
    background: color-mix(in oklch, var(--accent) 14%, transparent);
    color: var(--accent);
  }
  .card-body { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .card-title {
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--fg);
    line-height: 1.3;
  }
  /* Show the full prompt as a single ellipsised teaser — title is what users
     scan; the prompt body sells what'll get pasted once they click. Two-line
     clamp burned vertical real estate w/o adding scan value. */
  .card-prompt {
    font-size: 12px;
    color: var(--fg-faint);
    line-height: 1.35;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* ── Auth-help block (needsAuth path) ──────────────────────────────────── */
  .auth-help {
    margin-top: 8px;
    max-width: 520px;
    text-align: center;
    font-size: var(--fs-sm);
    color: var(--fg-muted);
  }
  .auth-help a {
    color: var(--accent);
    text-decoration: none;
    display: inline-flex; align-items: center; gap: 4px;
  }
  .auth-help a:hover { text-decoration: underline; }
  .auth-help .dim { color: var(--fg-subtle); }
  .auth-help .muted { font-size: var(--fs-xs); margin: 8px 0 0; }
  .auth-help code {
    font-family: var(--font-mono, monospace);
    font-size: 0.88em;
    padding: 1px 5px;
    background: var(--bg-elev-2);
    border-radius: 3px;
    color: var(--fg-2);
  }
</style>

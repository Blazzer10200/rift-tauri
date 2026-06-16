<script lang="ts">
  import { onMount } from "svelte";
  import { GitBranch, RefreshCw, X, ArrowUp, ArrowDown, Loader2, FolderGit2, Cloud } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { git } from "../../state/git.svelte";
  import { environmentDock } from "../../state/environmentDock.svelte";
  import FileDiffCard from "./FileDiffCard.svelte";

  const root = $derived(assistant.activeTab?.workspaceRoot ?? null);

  let message = $state("");
  let pushing = $state(false);

  onMount(() => { void git.refresh(root); });
  // Re-pull whenever the focused tab's workspace root changes.
  $effect(() => {
    const r = root;
    void git.refresh(r);
  });

  const status = $derived(git.status);
  const hasChanges = $derived((status?.files.length ?? 0) > 0);

  async function doCommit(push: boolean) {
    if (!message.trim() || git.committing) return;
    pushing = push;
    const ok = await git.commit(root, message, push);
    if (ok) message = "";
  }
</script>

<div class="env">
  <div class="hd">
    <span class="hd-title">Environment</span>
    <button class="hd-btn" type="button" title="Refresh" aria-label="Refresh git status" disabled={git.loading} onclick={() => git.refresh(root)}>
      {#if git.loading}<Loader2 size={14} class="spin" />{:else}<RefreshCw size={14} />{/if}
    </button>
    <button class="hd-btn" type="button" title="Close" aria-label="Close panel" onclick={() => environmentDock.toggle()}>
      <X size={15} />
    </button>
  </div>

  {#if git.error}
    <div class="empty">
      <p>{git.error}</p>
      <button class="retry" type="button" onclick={() => git.refresh(root)}>Try again</button>
    </div>
  {:else if status}
    <div class="scroll">
      <!-- Changes summary -->
      <div class="row-line">
        <span class="rl-label">Changes</span>
        {#if status.total_adds === 0 && status.total_dels === 0}
          <span class="rl-muted">none</span>
        {:else}
          <span class="rl-stat">
            <span class="adds">+{status.total_adds}</span>
            <span class="dels">−{status.total_dels}</span>
          </span>
        {/if}
      </div>

      <!-- Branch + tracking -->
      <div class="row-line">
        <GitBranch size={14} class="rl-ic" />
        <span class="rl-branch">{status.branch}</span>
        {#if status.upstream}<span class="rl-up">→ {status.upstream}</span>{/if}
        {#if status.ahead > 0 || status.behind > 0}
          <span class="track">
            {#if status.ahead > 0}<span class="t-ahead"><ArrowUp size={11} />{status.ahead}</span>{/if}
            {#if status.behind > 0}<span class="t-behind"><ArrowDown size={11} />{status.behind}</span>{/if}
          </span>
        {/if}
      </div>

      <!-- Commit or push -->
      {#if hasChanges}
        <div class="commit">
          <textarea
            class="commit-msg"
            placeholder="Commit message…"
            rows="2"
            bind:value={message}
            disabled={git.committing}
          ></textarea>
          {#if git.commitError}<div class="commit-err">{git.commitError}</div>{/if}
          <div class="commit-actions">
            <button class="ca primary" type="button" disabled={!message.trim() || git.committing} onclick={() => doCommit(false)}>
              {#if git.committing && !pushing}<Loader2 size={13} class="spin" />{/if} Commit
            </button>
            <button class="ca" type="button" disabled={!message.trim() || git.committing || !status.upstream} title={status.upstream ? "Commit and push to " + status.upstream : "No upstream branch to push to"} onclick={() => doCommit(true)}>
              {#if git.committing && pushing}<Loader2 size={13} class="spin" />{:else}<Cloud size={13} />{/if} Commit &amp; Push
            </button>
          </div>
        </div>
      {/if}

      <!-- File list -->
      <div class="files">
        {#if !hasChanges}
          <div class="clean">Working tree clean.</div>
        {:else}
          {#each status.files as f (f.path)}
            <FileDiffCard fileStat={f} {root} />
          {/each}
        {/if}
      </div>

      <!-- Sources -->
      <div class="sources">
        <div class="src-head">Sources</div>
        <div class="src-row" title={status.root}><FolderGit2 size={13} class="src-ic" /><span class="src-text">{status.root}</span></div>
        {#if status.remote}
          <div class="src-row" title={status.remote}><Cloud size={13} class="src-ic" /><span class="src-text">{status.remote}</span></div>
        {:else}
          <div class="src-row muted">No remote configured</div>
        {/if}
      </div>
    </div>
  {:else}
    <div class="empty"><Loader2 size={15} class="spin" /><p>Reading working tree…</p></div>
  {/if}
</div>

<style>
  .env { display: flex; flex-direction: column; height: 100%; min-height: 0; background: var(--bg); color: var(--fg); }
  .hd {
    display: flex; align-items: center; gap: 4px; padding: 9px 10px 9px 13px;
    border-bottom: 1px solid var(--border); flex: none;
  }
  .hd-title { flex: 1 1 auto; font-size: var(--fs-sm); font-weight: 600; letter-spacing: 0.01em; }
  .hd-btn {
    flex: none; display: grid; place-items: center; width: 26px; height: 26px;
    border: 0; border-radius: 6px; background: none; color: var(--fg-3); cursor: pointer;
  }
  .hd-btn:hover:not(:disabled) { color: var(--fg); background: color-mix(in oklab, var(--accent) 10%, transparent); }
  .hd-btn:disabled { opacity: 0.5; cursor: default; }
  :global(.spin) { animation: env-spin 1s linear infinite; }
  @keyframes env-spin { to { transform: rotate(360deg); } }

  .scroll { flex: 1 1 auto; min-height: 0; overflow-y: auto; padding: 10px; display: flex; flex-direction: column; gap: 8px; }

  .row-line { display: flex; align-items: center; gap: 7px; padding: 4px 2px; font-size: var(--fs-xs); }
  .row-line :global(.rl-ic) { color: var(--fg-3); flex: none; }
  .rl-label { color: var(--fg-2); flex: 1 1 auto; }
  .rl-muted { color: var(--fg-3); }
  .rl-stat { display: inline-flex; gap: 7px; font-variant-numeric: tabular-nums; }
  .rl-branch { font-weight: 600; }
  .rl-up { color: var(--fg-3); }
  .track { margin-left: auto; display: inline-flex; gap: 7px; font-variant-numeric: tabular-nums; }
  .t-ahead, .t-behind { display: inline-flex; align-items: center; gap: 1px; color: var(--fg-2); }
  .adds { color: var(--ok); }
  .dels { color: var(--danger); }

  .commit { display: flex; flex-direction: column; gap: 6px; padding: 2px; }
  .commit-msg {
    width: 100%; resize: vertical; min-height: 42px;
    background: var(--surface); border: 1px solid var(--border); border-radius: 8px;
    padding: 7px 9px; color: var(--fg); font: inherit; font-size: var(--fs-xs); line-height: 1.4;
  }
  .commit-msg:focus { outline: none; border-color: color-mix(in oklab, var(--accent) 45%, var(--border)); }
  .commit-err { color: var(--danger); font-size: var(--fs-xs); white-space: pre-wrap; }
  .commit-actions { display: flex; gap: 6px; }
  .ca {
    display: inline-flex; align-items: center; gap: 5px; padding: 5px 11px;
    border: 1px solid var(--border); border-radius: var(--radius-sm); background: var(--surface);
    color: var(--fg); font: inherit; font-size: var(--fs-xs); font-weight: 550; cursor: pointer;
  }
  .ca:hover:not(:disabled) { border-color: color-mix(in oklab, var(--accent) 40%, var(--border)); }
  .ca.primary { background: color-mix(in oklab, var(--accent) 22%, var(--surface)); border-color: color-mix(in oklab, var(--accent) 40%, var(--border)); }
  .ca:disabled { opacity: 0.5; cursor: default; }

  .files { display: flex; flex-direction: column; gap: 6px; }
  .clean { padding: 14px 4px; text-align: center; color: var(--fg-3); font-size: var(--fs-xs); }

  .sources { margin-top: 4px; padding-top: 8px; border-top: 1px solid var(--border); display: flex; flex-direction: column; gap: 5px; }
  .src-head { color: var(--fg-3); font-size: var(--fs-xs); font-weight: 600; text-transform: uppercase; letter-spacing: 0.04em; }
  .src-row { display: flex; align-items: center; gap: 7px; font-size: var(--fs-xs); color: var(--fg-2); min-width: 0; }
  .src-row.muted { color: var(--fg-3); }
  .src-row :global(.src-ic) { color: var(--fg-3); flex: none; }
  .src-text { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .empty { display: flex; flex-direction: column; align-items: center; gap: 10px; padding: 32px 18px; color: var(--fg-2); font-size: var(--fs-xs); text-align: center; }
  .retry {
    padding: 5px 12px; border: 1px solid var(--border); border-radius: var(--radius-sm);
    background: var(--surface); color: var(--fg); font: inherit; font-size: var(--fs-xs); cursor: pointer;
  }
  .retry:hover { border-color: color-mix(in oklab, var(--accent) 40%, var(--border)); }
</style>

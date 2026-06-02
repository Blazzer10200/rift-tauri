<script lang="ts">
  import { KeyRound, FolderSync, Server, Sparkles, Check } from "lucide-svelte";
  import ObStage from "./ObStage.svelte";
  import { uiPrefs, ACCENTS } from "$lib/state/ui-prefs.svelte";
</script>

<ObStage kind="welcome" caption="workspace · sftp · auto-sync" />

<header class="ob-head">
  <span class="ob-eyebrow">First-run setup</span>
  <h1 class="ob-title">Welcome to Rift</h1>
  <p class="ob-sub">
    A FiveM / RedM dev workspace with SFTP, auto-sync, drift reconciliation, and an
    embedded Claude assistant. The next few steps get you connected.
  </p>
</header>

<div class="ob-vlist">
  <div class="ob-vrow">
    <span class="ob-vic"><KeyRound size={17} /></span>
    <span class="ob-vbody">
      <span class="ob-vt">SSH key auth</span>
      <span class="ob-vp">Generate or import a key — no passwords stored.</span>
    </span>
  </div>
  <div class="ob-vrow">
    <span class="ob-vic"><Server size={17} /></span>
    <span class="ob-vbody">
      <span class="ob-vt">Connect your server</span>
      <span class="ob-vp">Host, port, user, key and the remote + local roots.</span>
    </span>
  </div>
  <div class="ob-vrow">
    <span class="ob-vic"><FolderSync size={17} /></span>
    <span class="ob-vbody">
      <span class="ob-vt">Auto-sync &amp; drift</span>
      <span class="ob-vp">Edits mirror both ways; conflicts surface before any I/O.</span>
    </span>
  </div>
  <div class="ob-vrow">
    <span class="ob-vic"><Sparkles size={17} /></span>
    <span class="ob-vbody">
      <span class="ob-vt">Claude assistant</span>
      <span class="ob-vp">Optional — uses your existing <code>claude</code> CLI auth.</span>
    </span>
  </div>
</div>

<div class="ob-accent">
  <span class="ob-accent-label">Personalize · accent</span>
  <div class="ob-accent-row">
    {#each ACCENTS as a (a.id)}
      <button
        type="button"
        class="ob-accent-dot"
        class:on={uiPrefs.accentHue === a.hue}
        style="--sw: oklch(0.78 0.15 {a.hue})"
        title={a.label}
        aria-label={a.label}
        aria-pressed={uiPrefs.accentHue === a.hue}
        onclick={() => uiPrefs.setAccentHue(a.hue)}
      >
        {#if uiPrefs.accentHue === a.hue}<Check size={14} />{/if}
      </button>
    {/each}
  </div>
</div>

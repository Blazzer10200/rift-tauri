<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  type ListEntry = { name: string; is_dir: boolean; size: number; mtime: number };

  let host = $state("");
  let port = $state(22);
  let user = $state("");
  let key_path = $state("");
  let remote_path = $state("/");
  let entries = $state<ListEntry[]>([]);
  let error = $state("");
  let connecting = $state(false);
  let version = $state("?");

  onMount(async () => {
    try { version = await invoke<string>("app_version"); } catch {}
  });

  async function listRemote(e: Event) {
    e.preventDefault();
    error = "";
    entries = [];
    connecting = true;
    try {
      entries = await invoke<ListEntry[]>("sftp_list", {
        args: { host, port, user, key_path, remote_path },
      });
    } catch (err) {
      error = String(err);
    } finally {
      connecting = false;
    }
  }
</script>

<main>
  <header>
    <h1>Rift v{version}</h1>
    <p class="sub">Phase 0 stub — Tauri + Svelte + russh-sftp end-to-end probe</p>
  </header>

  <form onsubmit={listRemote}>
    <label>Host <input bind:value={host} placeholder="example.com" required /></label>
    <label>Port <input type="number" bind:value={port} required /></label>
    <label>User <input bind:value={user} placeholder="blazzer" required /></label>
    <label>Key path <input bind:value={key_path} placeholder="C:/Users/BLAZZER/.ssh/id_ed25519" required /></label>
    <label>Remote path <input bind:value={remote_path} required /></label>
    <button type="submit" disabled={connecting}>{connecting ? "Connecting…" : "List"}</button>
  </form>

  {#if error}
    <pre class="err">{error}</pre>
  {/if}

  {#if entries.length}
    <table>
      <thead>
        <tr><th>Name</th><th>Type</th><th>Size</th></tr>
      </thead>
      <tbody>
        {#each entries as e}
          <tr>
            <td>{e.name}</td>
            <td>{e.is_dir ? "dir" : "file"}</td>
            <td>{e.size}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</main>

<style>
  :global(body) { background: #0F0F12; color: #E8E8EE; font-family: system-ui, sans-serif; margin: 0; }
  main { padding: 24px; max-width: 800px; margin: 0 auto; }
  header h1 { color: #8B6BE6; margin: 0 0 4px; }
  .sub { color: #7A7A85; margin: 0 0 24px; font-size: 13px; }
  form { display: grid; gap: 8px; max-width: 480px; margin-bottom: 16px; }
  label { display: grid; gap: 4px; font-size: 12px; color: #7A7A85; }
  input { background: #17171C; border: 1px solid #26262E; color: #E8E8EE; padding: 8px 10px; border-radius: 4px; }
  input:focus { outline: none; border-color: #8B6BE6; }
  button { background: #8B6BE6; color: white; border: 0; padding: 10px 16px; border-radius: 4px; cursor: pointer; font-weight: 600; }
  button:disabled { opacity: 0.5; cursor: wait; }
  .err { background: #2a1418; color: #FF5C6B; padding: 12px; border-radius: 4px; white-space: pre-wrap; font-family: monospace; font-size: 12px; }
  table { width: 100%; border-collapse: collapse; margin-top: 16px; }
  th, td { text-align: left; padding: 6px 10px; border-bottom: 1px solid #26262E; font-size: 13px; }
  th { color: #7A7A85; font-weight: 500; text-transform: uppercase; font-size: 11px; letter-spacing: 0.5px; }
</style>

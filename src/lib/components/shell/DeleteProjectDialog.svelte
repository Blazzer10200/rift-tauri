<script lang="ts">
  // Confirm before a project leaves Rift's list. Removal is config-only
  // (assistant_delete_project drops the entry + its recent-roots slot): the
  // folder on disk and its chats are untouched, so the project can be re-added.
  import * as AlertDialog from "$lib/components/ui/alert-dialog/index.js";
  import { projects } from "$lib/state/projects.svelte";
  import { notify } from "$lib/state/toast.svelte";
  import type { Project } from "$lib/state/assistant/types";

  let { project = $bindable(null) }: { project: Project | null } = $props();

  let busy = $state(false);
  let cancelRef = $state<HTMLButtonElement | null>(null);
  // Held past close so the title doesn't blank out during the exit animation.
  let name = $state("");
  $effect(() => {
    if (project) name = project.name;
  });

  async function confirmDelete() {
    const p = project;
    if (!p || busy) return;
    busy = true;
    await projects.remove(p.id);
    busy = false;
    if (projects.lastError) {
      notify.warn("Delete failed", { detail: projects.lastError });
      return;
    }
    project = null;
    notify.info("Project removed", { detail: p.name });
  }
</script>

<AlertDialog.Root
  open={project !== null}
  onOpenChange={(o) => {
    if (!o && !busy) project = null;
  }}
>
  <AlertDialog.Content
    size="sm"
    escapeKeydownBehavior={busy ? "ignore" : "close"}
    onOpenAutoFocus={(e) => {
      // Land on the safe choice: Enter must not delete.
      e.preventDefault();
      cancelRef?.focus();
    }}
  >
    <AlertDialog.Header>
      <AlertDialog.Title>Delete “{name}”?</AlertDialog.Title>
      <AlertDialog.Description>
        This removes it from Rift's project list. The folder, its files, and its chats stay on disk, so you can add it back anytime.
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer>
      <AlertDialog.Cancel bind:ref={cancelRef} disabled={busy}>Cancel</AlertDialog.Cancel>
      <AlertDialog.Action variant="danger" disabled={busy} onclick={confirmDelete}>Delete project</AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>

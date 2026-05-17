<script lang="ts">
  import SyncPage from "../sync/SyncPage.svelte";
  import type { PanelIcon } from "./index";

  // PanelShell always passes {title, icon} from PANELS; declare to satisfy
  // the contract but don't use — SyncPage owns its own chrome.
  let { title: _t, icon: _i }: { title: string; icon: PanelIcon } = $props();
</script>

<SyncPage />

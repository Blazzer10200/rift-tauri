<script lang="ts">
  // Canonical Rift brand mark. Single source for the app logo across titlebar,
  // onboarding, and welcome hero. The PNG is a self-contained dark squircle —
  // no container background needed at call sites.
  import logoUrl from "$lib/assets/rift-logo.png";

  type Props = { size?: number; class?: string };
  let { size = 24, class: cls = "" }: Props = $props();
</script>

<img
  class="riftlogo {cls}"
  src={logoUrl}
  width={size}
  height={size}
  alt=""
  draggable="false"
  aria-hidden="true"
/>

<style>
  .riftlogo {
    display: block;
    flex-shrink: 0;
    -webkit-user-drag: none;
    user-select: none;
    object-fit: contain;
  }
</style>

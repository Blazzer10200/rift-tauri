<script lang="ts">
	import { ContextMenu as ContextMenuPrimitive } from "bits-ui";
	import { cn } from "$lib/utils.js";
	import { floatingPanel, floatingMotion } from "../recipes.js";

	let {
		ref = $bindable(null),
		class: className,
		...restProps
	}: ContextMenuPrimitive.SubContentProps = $props();
</script>

<ContextMenuPrimitive.SubContent
	bind:ref
	data-slot="context-menu-sub-content"
	class={cn(
		floatingPanel,
		floatingMotion,
		"min-w-40 max-h-(--bits-menu-content-available-height) origin-(--bits-menu-content-transform-origin) overflow-x-hidden overflow-y-auto",
		className
	)}
	{...restProps}
/>

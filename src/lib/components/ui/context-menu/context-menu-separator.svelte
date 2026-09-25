<script lang="ts">
	import { ContextMenu as ContextMenuPrimitive } from "bits-ui";
	import { cn } from "$lib/utils.js";
	import { menuSeparator } from "../recipes.js";

	let {
		ref = $bindable(null),
		class: className,
		...restProps
	}: ContextMenuPrimitive.SeparatorProps = $props();
</script>

<ContextMenuPrimitive.Separator
	bind:ref
	data-slot="context-menu-separator"
	class={cn(menuSeparator, className)}
	{...restProps}
/>

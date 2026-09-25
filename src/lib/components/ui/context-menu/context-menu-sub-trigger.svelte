<script lang="ts">
	import { ContextMenu as ContextMenuPrimitive } from "bits-ui";
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import { cn, type WithoutChild } from "$lib/utils.js";
	import { menuItem } from "../recipes.js";

	let {
		ref = $bindable(null),
		class: className,
		inset,
		children,
		...restProps
	}: WithoutChild<ContextMenuPrimitive.SubTriggerProps> & {
		inset?: boolean;
	} = $props();
</script>

<ContextMenuPrimitive.SubTrigger
	bind:ref
	data-slot="context-menu-sub-trigger"
	data-inset={inset}
	class={cn(menuItem, "data-[state=open]:bg-surface-hover data-[state=open]:text-fg", className)}
	{...restProps}
>
	{@render children?.()}
	<ChevronRightIcon class="ml-auto" />
</ContextMenuPrimitive.SubTrigger>

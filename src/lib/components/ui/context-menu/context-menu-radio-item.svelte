<script lang="ts">
	import { ContextMenu as ContextMenuPrimitive } from "bits-ui";
	import CheckIcon from '@lucide/svelte/icons/check';
	import { cn, type WithoutChild } from "$lib/utils.js";
	import { menuItem, menuItemChecked, menuCheck } from "../recipes.js";

	let {
		ref = $bindable(null),
		class: className,
		inset,
		children: childrenProp,
		...restProps
	}: WithoutChild<ContextMenuPrimitive.RadioItemProps> & {
		inset?: boolean;
	} = $props();
</script>

<ContextMenuPrimitive.RadioItem
	bind:ref
	data-slot="context-menu-radio-item"
	data-inset={inset}
	class={cn(menuItem, menuItemChecked, className)}
	{...restProps}
>
	{#snippet children({ checked })}
		{@render childrenProp?.({ checked })}
		{#if checked}
			<CheckIcon class={menuCheck} />
		{/if}
	{/snippet}
</ContextMenuPrimitive.RadioItem>

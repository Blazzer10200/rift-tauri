<script lang="ts">
	import { ContextMenu as ContextMenuPrimitive } from "bits-ui";
	import MinusIcon from '@lucide/svelte/icons/minus';
	import CheckIcon from '@lucide/svelte/icons/check';
	import { cn, type WithoutChildrenOrChild } from "$lib/utils.js";
	import { menuItem, menuItemChecked, menuCheck } from "../recipes.js";
	import type { Snippet } from "svelte";

	let {
		ref = $bindable(null),
		checked = $bindable(false),
		indeterminate = $bindable(false),
		class: className,
		inset,
		children: childrenProp,
		...restProps
	}: WithoutChildrenOrChild<ContextMenuPrimitive.CheckboxItemProps> & {
		inset?: boolean;
		children?: Snippet;
	} = $props();
</script>

<ContextMenuPrimitive.CheckboxItem
	bind:ref
	bind:checked
	bind:indeterminate
	data-slot="context-menu-checkbox-item"
	data-inset={inset}
	class={cn(menuItem, menuItemChecked, className)}
	{...restProps}
>
	{#snippet children({ checked, indeterminate })}
		{@render childrenProp?.()}
		{#if indeterminate}
			<MinusIcon class={menuCheck} />
		{:else if checked}
			<CheckIcon class={menuCheck} />
		{/if}
	{/snippet}
</ContextMenuPrimitive.CheckboxItem>

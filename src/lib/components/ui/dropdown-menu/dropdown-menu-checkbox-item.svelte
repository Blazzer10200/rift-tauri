<script lang="ts">
	import { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";
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
		children: childrenProp,
		...restProps
	}: WithoutChildrenOrChild<DropdownMenuPrimitive.CheckboxItemProps> & {
		children?: Snippet;
	} = $props();
</script>

<DropdownMenuPrimitive.CheckboxItem
	bind:ref
	bind:checked
	bind:indeterminate
	data-slot="dropdown-menu-checkbox-item"
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
</DropdownMenuPrimitive.CheckboxItem>

<script lang="ts">
	import { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";
	import CheckIcon from '@lucide/svelte/icons/check';
	import { cn, type WithoutChild } from "$lib/utils.js";
	import { menuItem, menuItemChecked, menuCheck } from "../recipes.js";

	let {
		ref = $bindable(null),
		class: className,
		children: childrenProp,
		...restProps
	}: WithoutChild<DropdownMenuPrimitive.RadioItemProps> = $props();
</script>

<DropdownMenuPrimitive.RadioItem
	bind:ref
	data-slot="dropdown-menu-radio-item"
	class={cn(menuItem, menuItemChecked, className)}
	{...restProps}
>
	{#snippet children({ checked })}
		{@render childrenProp?.({ checked })}
		{#if checked}
			<CheckIcon class={menuCheck} />
		{/if}
	{/snippet}
</DropdownMenuPrimitive.RadioItem>

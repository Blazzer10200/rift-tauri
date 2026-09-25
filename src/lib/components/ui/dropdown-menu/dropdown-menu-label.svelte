<script lang="ts">
	import { cn, type WithElementRef } from "$lib/utils.js";
	import { menuLabel } from "../recipes.js";
	import type { HTMLAttributes } from "svelte/elements";

	let {
		ref = $bindable(null),
		class: className,
		inset,
		children,
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLDivElement>> & {
		inset?: boolean;
	} = $props();
</script>

<div
	bind:this={ref}
	data-slot="dropdown-menu-label"
	data-inset={inset}
	class={cn(menuLabel, className)}
	{...restProps}
>
	{@render children?.()}
</div>

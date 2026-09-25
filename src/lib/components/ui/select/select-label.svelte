<script lang="ts">
	import { cn, type WithElementRef } from "$lib/utils.js";
	import type { HTMLAttributes } from "svelte/elements";
	import { menuLabel } from "../recipes.js";

	let {
		ref = $bindable(null),
		class: className,
		children,
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLDivElement>> & {} = $props();
</script>

<div
	bind:this={ref}
	data-slot="select-label"
	class={cn(menuLabel, className)}
	{...restProps}
>
	{@render children?.()}
</div>

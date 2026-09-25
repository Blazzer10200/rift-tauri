<script lang="ts">
	import { cn, type WithElementRef } from "$lib/utils.js";
	import type { HTMLAttributes } from "svelte/elements";

	let {
		ref = $bindable(null),
		class: className,
		children,
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLDivElement>> = $props();
</script>

<div
	bind:this={ref}
	data-slot="alert-dialog-media"
	class={cn("mb-1 inline-flex size-6.5 items-center justify-center rounded-md border border-border bg-bg-elev-2 text-fg-2 [&_svg:not([class*='size-'])]:size-3.5", className)}
	{...restProps}
>
	{@render children?.()}
</div>

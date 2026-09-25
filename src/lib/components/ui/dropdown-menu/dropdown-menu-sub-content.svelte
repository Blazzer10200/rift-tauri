<script lang="ts">
	import { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";
	import { cn } from "$lib/utils.js";
	import { floatingPanel, floatingMotion } from "../recipes.js";

	let {
		ref = $bindable(null),
		class: className,
		...restProps
	}: DropdownMenuPrimitive.SubContentProps = $props();
</script>

<DropdownMenuPrimitive.SubContent
	bind:ref
	data-slot="dropdown-menu-sub-content"
	class={cn(
		floatingPanel,
		floatingMotion,
		"min-w-40 max-h-(--bits-menu-content-available-height) origin-(--bits-menu-content-transform-origin) overflow-x-hidden overflow-y-auto",
		className
	)}
	{...restProps}
/>

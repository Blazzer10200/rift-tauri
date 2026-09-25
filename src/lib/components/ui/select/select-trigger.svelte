<script lang="ts">
	import { Select as SelectPrimitive } from "bits-ui";
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import { cn, type WithoutChild } from "$lib/utils.js";
	import { fieldTrigger } from "../recipes.js";

	let {
		ref = $bindable(null),
		class: className,
		children,
		size = "default",
		...restProps
	}: WithoutChild<SelectPrimitive.TriggerProps> & {
		size?: "sm" | "default";
	} = $props();
</script>

<SelectPrimitive.Trigger
	bind:ref
	data-slot="select-trigger"
	data-size={size}
	class={cn(
		fieldTrigger,
		"data-[size=sm]:h-6 data-[size=sm]:text-sm aria-invalid:border-danger aria-invalid:ring-danger-soft",
		className
	)}
	{...restProps}
>
	{@render children?.()}
	<ChevronDownIcon
		class="text-fg-muted transition-transform duration-(--dur-fast) ease-soft in-data-[state=open]:rotate-180"
	/>
</SelectPrimitive.Trigger>

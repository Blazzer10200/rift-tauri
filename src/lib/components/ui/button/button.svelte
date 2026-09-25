<script lang="ts" module>
	import { type VariantProps, tv } from "tailwind-variants";
	import { cn, type WithElementRef } from "$lib/utils.js";
	import type { HTMLAnchorAttributes, HTMLButtonAttributes } from "svelte/elements";

	export const buttonVariants = tv({
		base: "inline-flex shrink-0 cursor-pointer items-center justify-center gap-1.5 whitespace-nowrap rounded-sm border border-transparent text-sm outline-none select-none transition duration-(--dur-fast) ease-soft focus-visible:ring-2 focus-visible:ring-ring active:not-disabled:not-aria-[haspopup]:translate-y-px disabled:cursor-not-allowed disabled:opacity-55 aria-disabled:pointer-events-none aria-disabled:opacity-55 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-3.5",
		variants: {
			variant: {
				secondary: "border-border bg-surface text-fg hover:not-disabled:border-border-strong hover:not-disabled:bg-surface-hover active:not-disabled:bg-surface-active aria-expanded:bg-surface-hover",
				ghost: "bg-transparent text-fg-muted hover:not-disabled:bg-surface-hover hover:not-disabled:text-fg aria-expanded:bg-surface-hover aria-expanded:text-fg",
				soft: "border-ghost-border bg-accent-soft text-accent hover:not-disabled:border-accent",
				primary: "bg-accent font-semibold text-accent-fg hover:not-disabled:bg-accent-hover active:not-disabled:bg-accent-active",
				danger: "border-border bg-surface text-danger hover:not-disabled:border-danger hover:not-disabled:bg-danger-soft",
				warn: "border-border bg-surface text-warn hover:not-disabled:border-warn hover:not-disabled:bg-warn-soft",
				info: "border-border bg-surface text-info hover:not-disabled:border-info hover:not-disabled:bg-info-soft",
				link: "border-0 bg-transparent text-accent underline-offset-4 hover:underline",
			},
			size: {
				default: "h-6.5 px-2.5",
				sm: "h-5.5 px-2 text-xs",
				xs: "h-5 gap-1 px-1.5 text-xs",
				lg: "h-8 px-3.5",
				icon: "size-6.5",
				"icon-sm": "size-5.5",
				"icon-xs": "size-5 [&_svg:not([class*='size-'])]:size-3",
				"icon-lg": "size-8 [&_svg:not([class*='size-'])]:size-4",
			},
		},
		compoundVariants: [{ variant: "link", class: "h-auto px-0" }],
		defaultVariants: {
			variant: "secondary",
			size: "default",
		},
	});

	export type ButtonVariant = VariantProps<typeof buttonVariants>["variant"];
	export type ButtonSize = VariantProps<typeof buttonVariants>["size"];

	export type ButtonProps = WithElementRef<HTMLButtonAttributes> &
		WithElementRef<HTMLAnchorAttributes> & {
			variant?: ButtonVariant;
			size?: ButtonSize;
		};
</script>

<script lang="ts">
	let {
		class: className,
		variant = "secondary",
		size = "default",
		ref = $bindable(null),
		href = undefined,
		type = "button",
		disabled,
		children,
		...restProps
	}: ButtonProps = $props();
</script>

{#if href}
	<a
		bind:this={ref}
		data-slot="button"
		class={cn(buttonVariants({ variant, size }), className)}
		href={disabled ? undefined : href}
		aria-disabled={disabled}
		role={disabled ? "link" : undefined}
		tabindex={disabled ? -1 : undefined}
		{...restProps}
	>
		{@render children?.()}
	</a>
{:else}
	<button
		bind:this={ref}
		data-slot="button"
		class={cn(buttonVariants({ variant, size }), className)}
		{type}
		{disabled}
		{...restProps}
	>
		{@render children?.()}
	</button>
{/if}

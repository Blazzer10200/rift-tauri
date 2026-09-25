// Shared helpers for the shadcn-svelte primitives in $lib/components/ui.
//
// `cn` = clsx-style joining + tailwind-merge conflict resolution. The stock
// export only knows Tailwind's default scales, so a Rift-only theme name
// (`rounded-card`, `shadow-float`, `ease-page`) would NOT override its default
// sibling — both classes survive and source order picks the winner. Every
// non-T-shirt name registered in app.css's @theme bridge must be listed here;
// utils.test.ts pins the behavior.
import { createCn } from "cn/config";

export const cn = createCn({
  extend: {
    theme: {
      radius: ["card", "island", "tile", "tile-inner"],
      shadow: ["float"],
      ease: ["page", "soft", "spring"],
    },
  },
});

// Prop helpers the shadcn-svelte component templates import.
export type WithoutChild<T> = T extends { child?: any } ? Omit<T, "child"> : T;
export type WithoutChildren<T> = T extends { children?: any } ? Omit<T, "children"> : T;
export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;
export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & { ref?: U | null };

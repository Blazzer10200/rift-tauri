// Pure helpers extracted from ChatTabsBar.svelte (#20 H0 — chattabsbar-split.md).
// Unit-tested in helpers.test.ts. The path helpers now live in the neutral
// utils/path.ts (one canonical home — they had drifted into ~6 copies); re-
// exported here so the shell family's existing imports stay unchanged.
export { leafName, shortPath, prettyPath } from "$lib/utils/path";

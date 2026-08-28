import { mergeConfig, defineConfig } from "vitest/config";
import { defineConfig as defineViteConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "url";
import path from "path";

const baseConfig = defineViteConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      $lib: path.resolve(
        path.dirname(fileURLToPath(import.meta.url)),
        "src/lib",
      ),
    },
  },
});

export default mergeConfig(
  baseConfig,
  defineConfig({
    test: {
      environment: "node",
      exclude: [
        "**/node_modules/**",
        "**/src-tauri/**",
        "**/dist/**",
        // Stale agent worktrees under .claude/worktrees/ carry their own
        // (un-synced) tsconfig.json whose `extends: ./.svelte-kit/tsconfig.json`
        // can't resolve, so vitest's file crawl crashed transforming files there
        // and reported them as "failed test files" (no real test failure — 728/728
        // pass). Don't let a parallel-workflow worktree pollute the test run.
        "**/.claude/**",
      ],
    },
  }),
);

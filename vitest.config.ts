import { mergeConfig, defineConfig } from "vitest/config";
import { defineConfig as defineViteConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "url";
import path from "path";

const baseConfig = defineViteConfig({
  plugins: [svelte({ hot: false })],
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
      ],
    },
  }),
);

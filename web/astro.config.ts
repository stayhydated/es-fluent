import sitemap from "@astrojs/sitemap";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";

export default defineConfig({
  site: "https://stayhydated.github.io",
  base: "/es-fluent",

  vite: {
    plugins: [wasm(), topLevelAwait(), tailwindcss()],
  },

  build: {
    assets: "_assets",
  },

  server: {
    headers: {
      "Cross-Origin-Embedder-Policy": "require-corp",
      "Cross-Origin-Opener-Policy": "same-origin",
    },
  },

  integrations: [sitemap()],
});

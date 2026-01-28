import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";

import sitemap from "@astrojs/sitemap";

export default defineConfig({
  vite: {
      plugins: [tailwindcss()],
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
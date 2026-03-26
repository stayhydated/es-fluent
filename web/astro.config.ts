import sitemap from "@astrojs/sitemap";
import tailwindcss from "@tailwindcss/vite";
import mdx from "@astrojs/mdx";
import { defineConfig } from "astro/config";

export default defineConfig({
  site: "https://stayhydated.github.io",
  base: "/es-fluent",
  vite: {
    plugins: [tailwindcss()],
  },
  build: {
    assets: "_assets",
  },
  integrations: [sitemap(), mdx()],
});

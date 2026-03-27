import sitemap from "@astrojs/sitemap";
import tailwindcss from "@tailwindcss/vite";
import mdx from "@astrojs/mdx";
import { defineConfig } from "astro/config";
import { remarkRewriteLinks } from "./remark/remark-rewrite-links";
import { PROJECT_NAME } from "./consts";

export default defineConfig({
  site: "https://stayhydated.github.io",
  base: `/${PROJECT_NAME}`,
  vite: {
    plugins: [tailwindcss()],
  },
  build: {
    assets: "_assets",
  },
  integrations: [sitemap(), mdx()],
  markdown: {
    remarkPlugins: [remarkRewriteLinks],
  },
});

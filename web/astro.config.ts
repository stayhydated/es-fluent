import sitemap from "@astrojs/sitemap";
import tailwindcss from "@tailwindcss/vite";
import mdx from "@astrojs/mdx";
import { defineConfig } from "astro/config";
import { remarkRewriteLinks } from "@stayhydated/astro-wasm-site/remark/rewrite-links";
import { PROJECT_NAME } from "./consts";

const githubBlobBase = `https://github.com/stayhydated/${PROJECT_NAME}/blob/master`;

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
    remarkPlugins: [[remarkRewriteLinks, { githubBlobBase }]],
  },
});

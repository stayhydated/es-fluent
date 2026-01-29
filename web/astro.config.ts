import sitemap from "@astrojs/sitemap";
import tailwindcss from "@tailwindcss/vite";
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

	server: {
		headers: {
			"Cross-Origin-Embedder-Policy": "require-corp",
			"Cross-Origin-Opener-Policy": "same-origin",
		},
	},

	integrations: [sitemap()],
});

# Architecture: xtask

## Purpose

`xtask` provides workspace maintenance tasks for this project.

## CLI commands

- `build bevy-demo`: builds the Trunk-hosted Bevy demo into `web/public/bevy-demo`.
- `build book`: builds mdBook documentation to `web/public/book`.
- `build llms-txt`: concatenates mdBook sources into `web/public/llms.txt` for LLM consumption.
- `build web`: builds the release SSG Dioxus site for GitHub Pages into `web/dist`.

### build bevy-demo

- `xtask/src/commands/build_bevy_demo.rs`: runs `trunk build` for `examples/bevy-example`, writes the bundle to `web/public/bevy-demo`, disables Trunk SRI metadata so the Dioxus dev server can serve the generated JS without hash mismatches, validates that the output contains a wasm artifact with the expected language marker, and writes a local `.gitignore` for the generated directory.

### build book

- `xtask/src/commands/build_book.rs`: builds mdBook via the `mdbook` crate API with output to `web/public/book`, adds `.gitignore` to exclude built files from version control.

### build llms-txt

- `xtask/src/commands/build_llms_txt.rs`: loads the mdBook, skips draft chapters, writes a linked chapter index to `llms.txt`, and writes the expanded chapter content to `llms-full.txt`.

### build web

- `xtask/src/commands/build_web.rs`: clears the previous Dioxus release `public` output, runs `dx build --platform web --ssg --release --debug-symbols false --force-sequential true`, copies the generated release `public` tree into `web/dist`, restores the stable root copies of `book/`, `bevy-demo/`, `llms.txt`, `llms-full.txt`, and `.nojekyll` that GitHub Pages and the site expect, overwrites `404.html` from `index.html` for router fallback, and writes a fresh sitemap from the `web` crate route list.

# Architecture: xtask

## Purpose

`xtask` provides workspace maintenance tasks for this project.

## CLI commands

- `build-bevy-demo`: builds the Trunk-hosted Bevy demo into `web/public/bevy-example/app`.
- `build-book`: builds mdBook documentation to `web/public/book`.
- `build-llms-txt`: concatenates mdBook sources into `web/public/llms.txt` for LLM consumption.

### build-bevy-demo

- `xtask/src/commands/build_bevy_demo.rs`: runs `trunk build` for `examples/bevy-example`, writes the bundle to `web/public/bevy-example/app`, disables Trunk SRI metadata so the Dioxus dev server can serve the generated JS without hash mismatches, validates that the output contains a wasm artifact with the expected language marker, and writes a local `.gitignore` for the generated directory.

### build-book

- `xtask/src/commands/build_book.rs`: builds mdBook via the `mdbook` crate API with output to `web/public/book`, adds `.gitignore` to exclude built files from version control.

### build-llms-txt

- `xtask/src/commands/build_llms_txt.rs`: loads the mdBook, skips draft chapters, writes a linked chapter index to `llms.txt`, and writes the expanded chapter content to `llms-full.txt`.

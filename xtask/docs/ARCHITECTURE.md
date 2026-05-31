# Architecture: xtask

## Purpose

`xtask` provides workspace maintenance tasks for this project.

## CLI commands

- `build bevy-demo`: builds the Trunk-hosted Bevy demo into `web/public/bevy-demo`.
- `build gpui-demo`: builds the Trunk-hosted GPUI demo into `web/public/gpui-demo`.
- `build book`: builds mdBook documentation to `web/public/book`.
- `build llms-txt`: exports mdBook sources into `web/public/llms.txt`, `web/public/llms-full.txt`, and per-chapter Markdown files under `web/public/llms/` for LLM consumption.
- `build web`: builds the release SSG Dioxus site for GitHub Pages into `web/dist`.
- `release plan`: computes the crates.io publish order for publishable workspace crates.
- `release publish`: prints or executes publish commands one package at a time in release order.

### build bevy-demo

- `xtask/src/commands/build_bevy_demo.rs`: runs `trunk build index.html --example bevy-example` for `examples/bevy-example`, writes the bundle to `web/public/bevy-demo`, disables Trunk SRI metadata so the Dioxus dev server can serve the generated JS without hash mismatches, validates that the output contains a wasm artifact with the `es-fluent-lang` force-link marker, and writes a local `.gitignore` for the generated directory.

### build gpui-demo

- `xtask/src/commands/build_gpui_demo.rs`: runs `trunk build index.html --example gpui-example` for `examples/gpui-example` with `--release --no-default-features --no-sri --public-url ./ --dist web/public/gpui-demo`, using `RUSTUP_TOOLCHAIN=nightly`, validates that the output includes `wasm` and JavaScript artifacts containing the expected language marker, and adds a local `.gitignore` for the generated directory. This command requires nightly for this command only.

### Shared public-repo helpers

- `xtask/src/commands/build_book.rs`, `build_llms_txt.rs`, `build_web.rs`, and `release.rs` are thin wrappers around `stayhydated-xtask` in `../stayhydated/crates/stayhydated-xtask`.
- Keep reusable maintenance behavior in `shared` when it should apply to other public repositories. Keep es-fluent-specific demo builds, command wiring, paths, constants, and validations in this workspace.

### build book

- `xtask/src/commands/build_book.rs`: calls the shared mdBook builder with output to `web/public/book`.

### build llms-txt

- `xtask/src/commands/build_llms_txt.rs`: calls the shared llms export builder with es-fluent's base URL and `xtask/templates/llms-header.md`.

### build web

- `xtask/src/commands/build_web.rs`: calls the shared Dioxus SSG packaging helper with es-fluent's Dioxus arguments, copied static/demo directories, and sitemap output.

### release

- `xtask/src/commands/release.rs`: maps CLI arguments into `stayhydated_xtask::release::PublishOptions`. The shared implementation reads Cargo metadata, topologically sorts publishable crates by non-dev workspace dependencies, prints or runs publish commands, uses `cargo hack --no-dev-deps publish` by default, guards cargo-hack manifest rewrites with a clean tracked worktree check, supports `--from <package>` for resuming, and can retry failures caused by crates.io index propagation.

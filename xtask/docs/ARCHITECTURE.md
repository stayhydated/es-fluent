# Architecture: xtask

## Purpose

`xtask` provides workspace maintenance tasks for this project.

## CLI commands

- `build-book`: builds mdBook documentation to `web/public/book`.
- `build-llms-txt`: concatenates mdBook sources into `web/public/llms.txt` for LLM consumption.
- `build-wasm-examples`: builds wasm examples declared in `web/wasm-examples.json`.
- `generate-wasm-examples-schema`: writes `web/wasm-examples.schema.json` from Rust manifest types.
- `verify-wasm-examples`: verifies that declared wasm example outputs still embed their required markers.

### build-book

- `xtask/src/commands/build_book.rs`: builds mdBook via the `mdbook` crate API with output to `web/public/book`, adds `.gitignore` to exclude built files from version control.

### build-llms-txt

- `xtask/src/commands/build_llms_txt.rs`: loads the mdBook, skips draft chapters, writes a linked chapter index to `llms.txt`, and writes the expanded chapter content to `llms-full.txt`.

### build-wasm-examples

- `xtask/src/commands/build_wasm_examples.rs`: loads `web/wasm-examples.json`, runs `wasm-pack` for each declared example, and copies any declared asset directories into the declared output paths.

### generate-wasm-examples-schema

- `xtask/src/commands/generate_wasm_examples_schema.rs`: generates `web/wasm-examples.schema.json` from the Rust manifest types in `xtask/src/wasm_examples.rs`.

### verify-wasm-examples

- `xtask/src/commands/verify_wasm_examples.rs`: loads `web/wasm-examples.json`, asserts each declared module and wasm output exists, and checks the declared marker strings inside the wasm bytes.

# xtask

Internal task runner for repository maintenance tasks.

## Commands

### `build-book`

Builds the mdBook into `web/public/book`.

```bash
cargo xtask build-book
```

### `build-llms-txt`

Builds `web/public/llms.txt` and `web/public/llms-full.txt` from the current
mdBook sources.

```bash
cargo xtask build-llms-txt
```

### `build-wasm-examples`

Builds the wasm examples declared in `web/wasm-examples.json`.

```bash
cargo xtask build-wasm-examples
```

### `generate-wasm-examples-schema`

Regenerates `web/wasm-examples.schema.json` from the Rust manifest types used by
`xtask`.

```bash
cargo xtask generate-wasm-examples-schema
```

### `verify-wasm-examples`

Verifies that the declared wasm example outputs from `web/wasm-examples.json`
exist.

```bash
cargo xtask verify-wasm-examples
```

### `verify-wasm-markers`

Verifies repo-specific wasm binary marker invariants used in CI.

```bash
cargo xtask verify-wasm-markers
```

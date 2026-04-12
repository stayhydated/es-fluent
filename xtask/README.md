# xtask

Internal task runner for repository maintenance tasks.

## Commands

### `generate-lang-names`

Regenerates the bundled locale data used by:

- `crates/es-fluent-lang/es-fluent-lang.ftl`
- `crates/es-fluent-lang/i18n/<locale>/es-fluent-lang.ftl`
- `crates/es-fluent-lang-macro/src/supported_locales.rs`

```bash
cargo xtask generate-lang-names
```

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

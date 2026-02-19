# xtask

Internal task runner for repository maintenance tasks.

## Generate language-name resources

Regenerates the bundled locale data used by:

- `crates/es-fluent-lang/es-fluent-lang.ftl`
- `crates/es-fluent-lang/i18n/<locale>/es-fluent-lang.ftl`
- `crates/es-fluent-lang-macro/src/supported_locales.rs`

Command:

```bash
cargo run -p xtask -- generate-lang-names
```

# cldr-es-fluent-lang

Legacy Python tooling for language-name generation.

This workflow has been replaced by the Rust `xtask` command, which uses ICU4X baked data:

```sh
cargo run -p xtask -- generate-lang-names
```

The command regenerates:

1. `crates/es-fluent-lang/es-fluent-lang.ftl`
1. `crates/es-fluent-lang/i18n/<locale>/es-fluent-lang.ftl`
1. `crates/es-fluent-lang-macro/src/supported_locales.rs`

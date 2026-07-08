[![Docs](https://docs.rs/es-fluent-build/badge.svg)](https://docs.rs/es-fluent-build/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-build.svg)](https://crates.io/crates/es-fluent-build)

# es-fluent-build

`es-fluent-build` provides build-script helpers for `es-fluent` applications.

Use it from `build.rs` when your crate uses the embedded, Dioxus, or Bevy
manager macros. Those macros discover locale assets at compile time, so Cargo
needs explicit rebuild hints when locale files or folders are added, removed, or
renamed.

Most application code should still depend on [`es-fluent`](../es-fluent/README.md)
and one runtime manager. Add this crate only under `[build-dependencies]`.

```toml
[build-dependencies]
es-fluent-build = "*"
```

```rust,no_run
// build.rs
fn main() {
    es_fluent_build::track_i18n_assets();
}
```

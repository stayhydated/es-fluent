# es-fluent-build

[![crates.io: es-fluent-build][crate-badge]][crate]

Build-script support for packages that own `es-fluent` messages. It tracks
configured locale assets and writes the fallback catalog consumed by derive
macros for compile-time missing-message diagnostics.

Requires Rust 1.99 or newer.

## Example

In the message-owning package's Cargo-selected build script (`build.rs` by
default), call the helper after providing `i18n.toml` and fallback FTL assets:

~~~rust,no_run
fn main() {
    es_fluent_build::track_i18n_assets();
}
~~~

Cargo rebuilds when configured locale files or directories change, and derives
receive the fallback catalog. The same helper supports both policies documented
by [`es-fluent`][facade]. The maintained [example build script][example] uses
this call.

[facade]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent/README.md
[example]: https://github.com/stayhydated/es-fluent/blob/master/examples/readme/build.rs
[crate-badge]: https://img.shields.io/crates/v/es-fluent-build.svg?label=es-fluent-build
[crate]: https://crates.io/crates/es-fluent-build

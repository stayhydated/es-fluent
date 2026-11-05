# es-fluent-build

[![crates.io: es-fluent-build][crate-badge]][crate]

Build-script support for packages that own `es-fluent` messages. It tracks
configured locale assets and writes the build manifest es-fluent macros expand
from, including the fallback catalog behind compile-time missing-message
diagnostics.

Requires Rust 1.99 or newer.

## Example

In the message-owning package's Cargo-selected [build script][example]
(`build.rs` by default), call the helper after providing `i18n.toml` and fallback
FTL assets:

~~~rust,no_run
fn main() {
    es_fluent_build::track_i18n_assets();
}
~~~

Cargo reruns the helper when `i18n.toml` or configured locale files and
directories change. The manifest records `i18n.toml`, the discovered locales and
resources, and the fallback catalog, so derive, language, and manager macros
expand from it instead of reading the package. Macros reference the manifest's
content hash, so compiler caches such as sccache or kache re-expand them exactly
when it changes. The helper supports both
[package-local missing-message policies][facade].

[facade]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent/README.md
[example]: https://github.com/stayhydated/es-fluent/blob/master/examples/readme/build.rs
[crate-badge]: https://img.shields.io/crates/v/es-fluent-build.svg?label=es-fluent-build
[crate]: https://crates.io/crates/es-fluent-build

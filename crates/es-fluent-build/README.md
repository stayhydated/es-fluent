# es-fluent-build

[![crates.io: es-fluent-build][crate-badge]][crate]

Build-script support for configured locale assets. It tracks asset changes and
writes the fallback-message catalog used by strict derive validation. Call it
from the package build script:

~~~rust,no_run
// build.rs
fn main() {
    es_fluent_build::track_i18n_assets();
}
~~~

Use the same call in a custom path selected with `[package] build = "..."`.
The helper makes Cargo rebuild when configured locale files or directories are
added, removed, or renamed. It also parses the fallback locale for both strict
and fallback-string packages and hands its catalog to derive macros for
source-spanned missing-message diagnostics.

[crate-badge]: https://img.shields.io/crates/v/es-fluent-build.svg?label=es-fluent-build
[crate]: https://crates.io/crates/es-fluent-build

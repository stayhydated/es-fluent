# es-fluent-build

[![Docs](https://docs.rs/es-fluent-build/badge.svg)](https://docs.rs/es-fluent-build/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-build.svg)](https://crates.io/crates/es-fluent-build)

Build-script support for configured locale assets. It tracks asset changes and
writes the fallback-message catalog used by strict derive validation. Add it only
as a build dependency:

~~~toml
[build-dependencies]
es-fluent-build = "*"
~~~

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
source-spanned missing-message diagnostics. See
[Incremental builds](https://stayhydated.github.io/es-fluent/book/incremental_builds.html).

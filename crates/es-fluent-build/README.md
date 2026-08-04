# es-fluent-build

[![Docs](https://docs.rs/es-fluent-build/badge.svg)](https://docs.rs/es-fluent-build/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-build.svg)](https://crates.io/crates/es-fluent-build)

Build-script tracking for locale assets scanned by the embedded, Dioxus, and
Bevy manager macros. Add it only as a build dependency:

~~~toml
[build-dependencies]
es-fluent-build = "*"
~~~

~~~rust
// build.rs
fn main() {
    es_fluent_build::track_i18n_assets();
}
~~~

The helper makes Cargo rebuild when configured locale files or directories are
added, removed, or renamed. See
[Incremental builds](https://stayhydated.github.io/es-fluent/book/incremental_builds.html).

# es-fluent-build

[![CI](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg)](https://codecov.io/gh/stayhydated/es-fluent)
[![Book](https://img.shields.io/badge/book-online-blue)](https://stayhydated.github.io/es-fluent/book/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-build.svg)](https://crates.io/crates/es-fluent-build)

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

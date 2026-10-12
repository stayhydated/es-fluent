# es-fluent-lang-macro

[![Codecov: es-fluent-lang-macro][codecov-badge]][codecov]
[![crates.io: es-fluent-lang-macro][crate-badge]][crate]

The procedural macro re-exported by [`es-fluent-lang`][facade] through its
default `macros` feature. It reads canonical locale directories from
`i18n.toml` and fills an annotated empty enum with typed locale variants,
conversions, and language-label messages.

Requires Rust 1.99 or newer.

Applications use `es_fluent_lang::es_fluent_language`; a direct dependency serves
macro integrations that need the proc-macro entry point.

[facade]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-lang/README.md
[codecov-badge]: https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg?component=es-fluent-lang-macro
[codecov]: https://codecov.io/gh/stayhydated/es-fluent
[crate-badge]: https://img.shields.io/crates/v/es-fluent-lang-macro.svg?label=es-fluent-lang-macro
[crate]: https://crates.io/crates/es-fluent-lang-macro

# es-fluent-lang-macro

[![Codecov: es-fluent-lang-macro][codecov-badge]][codecov]
[![crates.io: es-fluent-lang-macro][crate-badge]][crate]

The procedural macro behind
[`es-fluent-lang`][es-fluent-lang]. It reads canonical
locale directories from `i18n.toml` and fills an annotated empty enum
with typed locale variants and conversions.

Applications should use the re-exported macro:

~~~rust,ignore
use es_fluent_lang::es_fluent_language;

#[es_fluent_language]
pub enum Languages {}
~~~

[es-fluent-lang]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-lang/README.md
[codecov-badge]: https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg?component=es-fluent-lang-macro
[codecov]: https://codecov.io/gh/stayhydated/es-fluent
[crate-badge]: https://img.shields.io/crates/v/es-fluent-lang-macro.svg?label=es-fluent-lang-macro
[crate]: https://crates.io/crates/es-fluent-lang-macro

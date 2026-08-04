# es-fluent-lang-macro

[![Docs](https://docs.rs/es-fluent-lang-macro/badge.svg)](https://docs.rs/es-fluent-lang-macro/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-lang-macro.svg)](https://crates.io/crates/es-fluent-lang-macro)

The procedural macro behind
[`es-fluent-lang`](../es-fluent-lang/README.md). It reads canonical
locale directories from `i18n.toml` and fills an annotated empty enum
with typed locale variants and conversions.

Applications should use the re-exported macro:

~~~rust
use es_fluent_lang::es_fluent_language;

#[es_fluent_language]
pub enum Languages {}
~~~

See the [language picker guide](https://stayhydated.github.io/es-fluent/book/language_enum.html)
for default and custom label modes.

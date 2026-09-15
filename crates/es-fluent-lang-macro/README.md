# es-fluent-lang-macro

[![CI](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg)](https://codecov.io/gh/stayhydated/es-fluent)
[![Book](https://img.shields.io/badge/book-online-blue)](https://stayhydated.github.io/es-fluent/book/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-lang-macro.svg)](https://crates.io/crates/es-fluent-lang-macro)

The procedural macro behind
[`es-fluent-lang`](../es-fluent-lang/README.md). It reads canonical
locale directories from `i18n.toml` and fills an annotated empty enum
with typed locale variants and conversions.

Applications should use the re-exported macro:

~~~rust,ignore
use es_fluent_lang::es_fluent_language;

#[es_fluent_language]
pub enum Languages {}
~~~

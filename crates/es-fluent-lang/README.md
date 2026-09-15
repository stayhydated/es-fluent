# es-fluent-lang

[![CI](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg)](https://codecov.io/gh/stayhydated/es-fluent)
[![Book](https://img.shields.io/badge/book-online-blue)](https://stayhydated.github.io/es-fluent/book/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-lang.svg)](https://crates.io/crates/es-fluent-lang)

Typed locale enums and localized language labels for `es-fluent`
applications.

Annotate an empty enum:

~~~rust,ignore
use es_fluent_lang::es_fluent_language;
use strum::EnumIter;

#[es_fluent_language]
#[derive(EnumIter)]
pub enum Languages {}
~~~

The macro reads canonical locale directories from `i18n.toml`,
includes the fallback locale, and implements conversions to and from
`LanguageIdentifier`. It also implements `FluentMessage` so
the active manager can render language-picker labels:

~~~rust,ignore
for language in Languages::iter() {
    println!("{}", i18n.localize_message(&language));
}
~~~

Labels are autonyms by default. Enable `localized-langs` to render
them in the selected UI language. Use
`#[es_fluent_language(custom)]` when the application ships its own FTL
labels.

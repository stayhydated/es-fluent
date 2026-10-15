# es-fluent-lang

[![Codecov: es-fluent-lang][codecov-badge]][codecov]
[![crates.io: es-fluent-lang][crate-badge]][crate]

Typed locale enums and ICU4X language-picker labels for `es-fluent`
applications. Use this crate directly when an application needs conversions
between its supported locales and `LanguageIdentifier`, plus language names
rendered through a concrete manager.

Requires Rust 1.99 or newer. The locale macro reads the package's `i18n.toml`
and canonical locale directories, including the fallback locale.

## Features

The default feature set enables `macros`.

| Feature | Default | Purpose |
| --- | --- | --- |
| `macros` | Yes | Re-exports `es_fluent_language` from `es-fluent-lang-macro`. |
| `localized-langs` | No | Displays language names in the selected UI language instead of as autonyms. |

## Example

This excerpt from the [shared example library][example] uses the default
`macros` feature and Strum's `EnumIter`. The package supplies its own
`i18n.toml` and locale assets:

~~~rust,ignore
use es_fluent_lang::es_fluent_language;
use strum::EnumIter;

#[es_fluent_language]
#[derive(Clone, Copy, Debug, EnumIter, Eq, PartialEq)]
pub enum Languages {}
~~~

The macro fills the enum with supported locales and implements conversions and
`FluentMessage`. The [complete executable][runtime] iterates these locales and
selects each on an embedded manager. Use `#[es_fluent_language(custom)]` when the
application owns its language-label FTL instead of using ICU4X names.

[example]: https://github.com/stayhydated/es-fluent/blob/master/examples/example-shared-lib/src/lib.rs
[runtime]: https://github.com/stayhydated/es-fluent/blob/master/examples/readme/src/main.rs
[codecov-badge]: https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg?component=es-fluent-lang
[codecov]: https://codecov.io/gh/stayhydated/es-fluent
[crate-badge]: https://img.shields.io/crates/v/es-fluent-lang.svg?label=es-fluent-lang
[crate]: https://crates.io/crates/es-fluent-lang

# es-fluent

[![Codecov: es-fluent][codecov-badge]][codecov]
[![crates.io: es-fluent][crate-badge]][crate]

The public facade for typed [Project Fluent][fluent] messages in Rust.
Applications depend on this crate for message derives and runtime traits, then
choose an [embedded][embedded], [Dioxus][dioxus], or [Bevy][bevy] manager to
resolve translations.

Requires Rust 1.99 or newer. Configured message-owning packages provide
`i18n.toml`, fallback FTL, and a build script using
[`es-fluent-build`][build] for compile-time fallback validation.

## Features

The default feature set enables `derive`.

| Feature | Default | Purpose |
| --- | --- | --- |
| `default` | Yes | Enables `derive`. |
| `derive` | Yes | Re-exports `EsFluent`, `EsFluentChoice`, `EsFluentLabel`, and `EsFluentVariants`. |
| `debug-embed` | No | Enables embedded manager-core assets and embeds them in native debug builds. |
| `icu-datetime` | No | Formats `SystemTime` and `Duration` arguments with ICU4X in the active locale. |
| `chrono` | No | Enables `icu-datetime` and Chrono temporal arguments. |
| `jiff` | No | Enables `icu-datetime` and Jiff temporal arguments. |

## Example

This declaration is an excerpt from the maintained [example library][example-types]
and uses the default `derive` feature. Its package supplies the locale configuration,
fallback catalog, and [embedded module registration][example-assets].

~~~rust,ignore
use es_fluent::EsFluent;

#[derive(EsFluent)]
pub struct WelcomeMessage<'a> {
    pub name: &'a str,
    pub count: i32,
}
~~~

The CLI discovers the library target and derives this fallback message and its
arguments; translators edit the FTL value:

~~~ftl
welcome_message = Welcome Message { $name } { $count }
~~~

With `i18n` initialized for English, the [complete executable][example-runtime]
passes a borrowed message to `i18n.localize_message(&welcome)` and renders
“Welcome Message John 5”.

`EsFluentVariants` supplies field or variant messages, `EsFluentLabel` supplies
a type label, and `EsFluentChoice` supplies selector values.

## Missing-message policy

The default `strict` policy checks derived messages against the fallback catalog
at compile time. A package can select `missing_message_policy = "fallback-str"`
in its `i18n.toml` to return snake_case Rust source names after locale fallback
is exhausted. The generated keys carry their package's policy, so both policies
can coexist. Fallible `try_localize_message` and `try_localize_label` still return
`None` for missing output.

[fluent]: https://projectfluent.org/
[embedded]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-manager-embedded/README.md
[dioxus]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-manager-dioxus/README.md
[bevy]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-manager-bevy/README.md
[build]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-build/README.md
[example-types]: https://github.com/stayhydated/es-fluent/blob/master/examples/readme/src/lib.rs
[example-runtime]: https://github.com/stayhydated/es-fluent/blob/master/examples/readme/src/main.rs
[example-assets]: https://github.com/stayhydated/es-fluent/blob/master/examples/readme/src/i18n.rs
[codecov-badge]: https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg?component=es-fluent
[codecov]: https://codecov.io/gh/stayhydated/es-fluent
[crate-badge]: https://img.shields.io/crates/v/es-fluent.svg?label=es-fluent
[crate]: https://crates.io/crates/es-fluent

# es-fluent

[![CI][ci-badge]][ci]
[![Codecov][codecov-badge]][codecov]
[![Book][book-badge]][book]
[![crates.io: es-fluent][es-fluent-badge]][es-fluent-crate]

`es-fluent` provides typed [Project Fluent](https://projectfluent.org/)
localization for Rust. Derive messages from structs and enums, maintain FTL
resources with `cargo es-fluent`, and resolve them with an embedded, Dioxus, or
Bevy runtime manager.

All published workspace packages require Rust 1.99 or newer. The Dioxus manager
targets Dioxus 0.7; the Bevy manager targets Bevy 0.20.

## Crates

Applications start with [`es-fluent`][es-fluent-readme] and one concrete runtime
manager. Add [`es-fluent-lang`][es-fluent-lang-readme] for a typed language picker;
use [`es-fluent-cli`][es-fluent-cli-readme] to maintain translations. Each owning
README documents its complete Cargo feature set.

| Crate | Purpose |
| --- | --- |
| [`es-fluent`][es-fluent-readme] | Typed-message derives and runtime traits. |
| [`es-fluent-build`][es-fluent-build-readme] | Build-script asset tracking and fallback-catalog generation. |
| [`es-fluent-cli`][es-fluent-cli-readme] | FTL generation, validation, synchronization, and inspection. |
| [`es-fluent-lang`][es-fluent-lang-readme] | Typed locale enums and localized language labels. |
| [`es-fluent-manager-bevy`][es-fluent-manager-bevy-readme] | Bevy localization plugin and components. |
| [`es-fluent-manager-core`][es-fluent-manager-core-readme] | Runtime contracts for custom manager integrations. |
| [`es-fluent-manager-dioxus`][es-fluent-manager-dioxus-readme] | Dioxus client and SSR localization. |
| [`es-fluent-manager-embedded`][es-fluent-manager-embedded-readme] | Embedded localization for general Rust applications. |

## Example

This excerpt from the [maintained example library][example-types] uses the
facade's default `derive` feature. That package supplies `i18n.toml`, fallback
FTL, build-script tracking, and an embedded manager module:

~~~rust
use es_fluent::EsFluent;

#[derive(EsFluent)]
pub struct WelcomeMessage<'a> {
    pub name: &'a str,
    pub count: i32,
}
~~~

The CLI derives the message ID and arguments. Its translated English resource is:

~~~ftl
welcome_message = Welcome Message { $name } { $count }
~~~

The [complete executable][example-runtime] initializes the embedded manager and
passes `WelcomeMessage { name: "John", count: 5 }` to `i18n.localize_message`,
rendering “Welcome Message John 5”.

[ci-badge]: https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml/badge.svg?branch=master
[ci]: https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml
[codecov-badge]: https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg
[codecov]: https://codecov.io/gh/stayhydated/es-fluent
[book-badge]: https://img.shields.io/badge/Book-mdBook-blue
[book]: https://stayhydated.github.io/es-fluent/book/
[es-fluent-badge]: https://img.shields.io/crates/v/es-fluent.svg?label=es-fluent
[es-fluent-crate]: https://crates.io/crates/es-fluent
[es-fluent-readme]: crates/es-fluent/README.md
[es-fluent-build-readme]: crates/es-fluent-build/README.md
[es-fluent-cli-readme]: crates/es-fluent-cli/README.md
[es-fluent-lang-readme]: crates/es-fluent-lang/README.md
[es-fluent-manager-bevy-readme]: crates/es-fluent-manager-bevy/README.md
[es-fluent-manager-core-readme]: crates/es-fluent-manager-core/README.md
[es-fluent-manager-dioxus-readme]: crates/es-fluent-manager-dioxus/README.md
[es-fluent-manager-embedded-readme]: crates/es-fluent-manager-embedded/README.md
[example-types]: https://github.com/stayhydated/es-fluent/blob/master/examples/readme/src/lib.rs
[example-runtime]: https://github.com/stayhydated/es-fluent/blob/master/examples/readme/src/main.rs

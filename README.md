# es-fluent

[![Build status](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml)
[![Book](https://img.shields.io/badge/docs-book-black)](https://stayhydated.github.io/es-fluent/book/)
[![API docs](https://docs.rs/es-fluent/badge.svg)](https://docs.rs/es-fluent/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent.svg)](https://crates.io/crates/es-fluent)

`es-fluent` provides typed
[Project Fluent](https://projectfluent.org/) localization for Rust. Derive
messages from structs and enums, maintain FTL resources with
`cargo es-fluent`, and resolve them with an embedded, Dioxus, or Bevy
runtime manager.

## Choose a runtime

| Application | Manager |
| --- | --- |
| General Rust, CLI, TUI, or desktop | `es-fluent-manager-embedded` |
| Dioxus client or SSR | `es-fluent-manager-dioxus` |
| Bevy | `es-fluent-manager-bevy` |

Framework managers follow their framework release lines:

| Surface | Compatible line |
| --- | --- |
| Core crates, CLI, embedded manager, and language enum | `0.18.x` |
| Dioxus manager and Dioxus | `0.7.x` |
| Bevy manager and Bevy | `0.19.x` |

## Quick start

Add the facade and embedded manager:

~~~toml
[dependencies]
es-fluent = "0.18"
es-fluent-manager-embedded = "0.18"
unic-langid = "0.9"

[build-dependencies]
es-fluent-build = "0.18"
~~~

Create `i18n.toml` beside `Cargo.toml`:

~~~toml
fallback_language = "en"
assets_dir = "assets/locales"
~~~

Track locale assets and generate the strict fallback-message catalog:

~~~rust,no_run
// build.rs
fn main() {
    es_fluent_build::track_i18n_assets();
}
~~~

Keep localizable types and the manager module reachable from a library target:

~~~rust
// src/lib.rs
pub mod i18n;

use es_fluent::EsFluent;

#[derive(EsFluent)]
pub struct Greeting<'a> {
    pub name: &'a str,
}
~~~

~~~rust
// src/i18n.rs
es_fluent_manager_embedded::define_i18n_module!();
~~~

Install the CLI, create the fallback locale, and generate FTL:

~~~sh
cargo install es-fluent-cli --locked
mkdir -p assets/locales/en
cargo es-fluent doctor
cargo es-fluent generate
~~~

Follow the [getting-started tutorial](https://stayhydated.github.io/es-fluent/book/getting_started.html)
to edit the fallback message and localize it at runtime.

Configured packages use strict fallback-message validation by default. Set
`missing_message_policy = "fallback-str"` in that package's `i18n.toml` when
normal typed lookup should return the snake_case Rust type, field, or variant
name after locale fallback is exhausted. Strict and fallback-string packages
can coexist in one workspace build; fallible lookup remains `None`.

## Documentation

- [User guide](https://stayhydated.github.io/es-fluent/book/)
- [Derive reference](https://stayhydated.github.io/es-fluent/book/deriving_messages.html)
- [Runtime managers](https://stayhydated.github.io/es-fluent/book/managers.html)
- [CLI reference](https://stayhydated.github.io/es-fluent/book/cli.html)
- [Rust API documentation](https://docs.rs/es-fluent/)

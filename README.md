# es-fluent

[![CI](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg)](https://codecov.io/gh/stayhydated/es-fluent)
[![Book](https://img.shields.io/badge/book-online-blue)](https://stayhydated.github.io/es-fluent/book/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent.svg)](https://crates.io/crates/es-fluent)

`es-fluent` provides typed [Project Fluent](https://projectfluent.org/)
localization for Rust. Derive messages from structs and enums, maintain FTL
resources with `cargo es-fluent`, and resolve them with an embedded, Dioxus, or
Bevy runtime manager.

## Typed messages

Define localizable types in a library target:

~~~rust
use es_fluent::EsFluent;

#[derive(EsFluent)]
pub enum LoginMessage<'a> {
    Welcome { name: &'a str },
    SignedOut,
}
~~~

The CLI derives message IDs and arguments from the type. Edit the generated FTL
values to supply your translations:

~~~ftl
login_message-Welcome = Welcome, { $name }!
login_message-SignedOut = You are signed out.
~~~

Resolve the message through your application's manager:

~~~rust,ignore
let text = i18n.localize_message(&LoginMessage::Welcome { name: "Ada" });
~~~

`EsFluentVariants` generates field and variant labels, `EsFluentLabel` gives a
type its own label, and `EsFluentChoice` supplies Fluent selector values.

## Choose a runtime

| Application | Manager |
| --- | --- |
| General Rust, CLI, TUI, or desktop | `es-fluent-manager-embedded` |
| Dioxus client or SSR | `es-fluent-manager-dioxus` |
| Bevy | `es-fluent-manager-bevy` |

Each package owns its configuration, FTL resources, and library-reachable
manager registration. A single runtime can discover resources from several
linked packages.

## Maintain translations

~~~sh
cargo es-fluent doctor
cargo es-fluent generate
cargo es-fluent status --all-locales
cargo es-fluent check --all-locales
~~~

Generation preserves edited translations by default. `status` previews pending
localization work, and `check` verifies keys, arguments, and locale coverage.

Configured packages validate fallback messages at compile time by default. Set
`missing_message_policy = "fallback-str"` in a package's `i18n.toml` when normal
typed lookup should return the snake_case Rust type, field, or variant name
after locale fallback is exhausted. Strict and fallback-string packages can
coexist in one workspace build; fallible lookup returns `None` for missing
output.

# es-fluent

[![Docs](https://docs.rs/es-fluent/badge.svg)](https://docs.rs/es-fluent/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent.svg)](https://crates.io/crates/es-fluent)

The public facade for typed
[Project Fluent](https://projectfluent.org/) messages in Rust. It re-exports the
derive macros and runtime traits used by `es-fluent` manager
integrations.

## Add the facade

~~~toml
[dependencies]
es-fluent = "*"
~~~

Derive a typed message in a library target:

~~~rust
use es_fluent::EsFluent;

#[derive(EsFluent)]
pub enum LoginMessage<'a> {
    Welcome { name: &'a str },
    SignedOut,
}
~~~

The CLI generates message IDs and Fluent arguments from the type:

~~~ftl
login_message-Welcome = Welcome, { $name }!
login_message-SignedOut = You are signed out.
~~~

Resolve values through a concrete embedded, Dioxus, or Bevy manager:

~~~rust,ignore
let text = i18n.localize_message(&LoginMessage::Welcome { name: "Ada" });
~~~

## Derive surface

- `EsFluent` defines messages and can infer selector values for
  unit-only enums.
- `EsFluentVariants` generates localizable field or variant metadata.
- `EsFluentLabel` defines a type-level label.
- `EsFluentChoice` defines standalone selector values.

Features `icu-datetime`, `chrono`, and `jiff`
support localized temporal arguments for their matching types.

See the [derive guide](https://stayhydated.github.io/es-fluent/book/deriving_messages.html)
for attributes, generated FTL, domains, namespaces, choices, and labels. See
the [getting-started tutorial](https://stayhydated.github.io/es-fluent/book/getting_started.html)
for CLI and runtime setup.

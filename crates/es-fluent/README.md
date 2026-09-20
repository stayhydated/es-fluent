# es-fluent

[![Codecov: es-fluent][codecov-badge]][codecov]
[![crates.io: es-fluent][crate-badge]][crate]

The public facade for typed
[Project Fluent](https://projectfluent.org/) messages in Rust. It re-exports the
derive macros and runtime traits used by `es-fluent` manager
integrations.

## Typed messages

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

## Missing-message policy

Configured crates validate derived messages against the fallback locale during
compilation. Call `es_fluent_build::track_i18n_assets()` from Cargo's selected
custom-build target so the compiler can read the generated fallback catalog. A
missing message value is a source-spanned compile-time error naming the Rust
item, domain, fallback root, and recovery command by default.

Set the package-local policy when the application must keep rendering after
every locale and Fluent fallback is exhausted:

~~~toml
# i18n.toml
missing_message_policy = "fallback-str"
~~~

`strict` is the default. Strict and fallback-string packages can coexist in the
same workspace build.

`localize_message(...)` and `localize_label(...)` then return the derived Rust
source name in snake_case. Struct messages and labels use the type name, enum
messages use the variant name, and `EsFluentVariants` messages use the source
field or variant name. Fallible `try_localize_message(...)` and
`try_localize_label(...)` continue to return `None` for missing output.

[codecov-badge]: https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg?component=es-fluent
[codecov]: https://codecov.io/gh/stayhydated/es-fluent
[crate-badge]: https://img.shields.io/crates/v/es-fluent.svg?label=es-fluent
[crate]: https://crates.io/crates/es-fluent

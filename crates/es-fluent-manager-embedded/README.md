# es-fluent-manager-embedded

[![Codecov: es-fluent-manager-embedded][codecov-badge]][codecov]
[![crates.io: es-fluent-manager-embedded][crate-badge]][crate]

Embedded localization for general Rust applications, including CLIs, TUIs,
desktop apps, and services. Depend on this concrete manager alongside the
[`es-fluent`][facade] message facade; retain a cloneable `EmbeddedI18n` handle in
application state.

Requires Rust 1.99 or newer. Release builds and Wasm debug builds embed the
configured assets. Native debug builds read them from disk unless `debug-embed`
is enabled.

## Features

The default feature set enables `macros` and its implicit
`es-fluent-manager-macros` dependency feature.

| Feature | Default | Purpose |
| --- | --- | --- |
| `macros` | Yes | Re-exports the embedded `define_i18n_module!` registration macro. |
| `es-fluent-manager-macros` | Yes | Implicit feature enabling the optional registration-macro dependency. |
| `debug-embed` | No | Embeds assets in native debug builds through manager-core, the facade, and `rust-embed`. |

## Example

The [complete executable][example-runtime] supplies the
[`WelcomeMessage` type][example-types], locale configuration, fallback catalog,
and [library module registration][example-assets]. This adapted excerpt uses the
public manager directly inside a function returning `Result<(), es_fluent_manager_embedded::EmbeddedInitError>`:

~~~rust,ignore
use es_fluent_manager_embedded::EmbeddedI18n;
use unic_langid::langid;

let i18n = EmbeddedI18n::try_new_with_language(langid!("en"))?;
let welcome = WelcomeMessage {
    name: "John",
    count: 5,
};
println!("WelcomeMessage: {}", i18n.localize_message(&welcome));
~~~

It propagates initialization errors and renders
“WelcomeMessage: Welcome Message John 5” in English.

Clones share locale state. Use strict initialization or selection when every
linked application module must support the requested locale; failed selections
preserve the previous state. See the [public API][api] for selection semantics
and the [facade][facade] for package-local missing-message policy.

[facade]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent/README.md
[api]: https://docs.rs/es-fluent-manager-embedded/latest/es_fluent_manager_embedded/struct.EmbeddedI18n.html
[example-types]: https://github.com/stayhydated/es-fluent/blob/master/examples/readme/src/lib.rs
[example-runtime]: https://github.com/stayhydated/es-fluent/blob/master/examples/readme/src/main.rs
[example-assets]: https://github.com/stayhydated/es-fluent/blob/master/examples/readme/src/i18n.rs
[codecov-badge]: https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg?component=es-fluent-manager-embedded
[codecov]: https://codecov.io/gh/stayhydated/es-fluent
[crate-badge]: https://img.shields.io/crates/v/es-fluent-manager-embedded.svg?label=es-fluent-manager-embedded
[crate]: https://crates.io/crates/es-fluent-manager-embedded

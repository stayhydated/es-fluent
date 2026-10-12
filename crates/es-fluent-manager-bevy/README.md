# es-fluent-manager-bevy

[![Codecov: es-fluent-manager-bevy][codecov-badge]][codecov]
[![crates.io: es-fluent-manager-bevy][crate-badge]][crate]

Typed localization for Bevy applications. Depend on this concrete manager
alongside the [`es-fluent`][facade] message facade. Its plugin loads FTL resources,
refreshes `FluentText<T>` when the locale changes, and exposes `BevyI18n` for
localization in systems.

Requires Rust 1.99 or newer and Bevy 0.20. Message-owning packages provide
`i18n.toml`, fallback assets, build-script tracking, and library-reachable module
registration.

## Features

The default feature set enables `macros`, its implicit
`es-fluent-manager-macros` dependency feature, and `file_watcher`.

| Feature | Default | Purpose |
| --- | --- | --- |
| `default` | Yes | Enables `macros` and `file_watcher`. |
| `macros` | Yes | Re-exports `define_i18n_module!` and the `BevyFluentText` derive. |
| `es-fluent-manager-macros` | Yes | Implicit feature enabling the optional registration and text macro dependency. |
| `file_watcher` | Yes | Enables Bevy's `embedded_watcher` and `file_watcher` asset features. |

## Example

Excerpt from the [maintained Bevy demo][example]. Its host supplies `app` with
`DefaultPlugins`, an asset root containing `i18n`, registered message modules,
and a typed `Languages` enum:

~~~rust,ignore
use es_fluent_manager_bevy::{I18nPlugin, I18nPluginConfig};

app.add_plugins(I18nPlugin::with_config(
    I18nPluginConfig::new(Languages::default().into()).with_asset_path("i18n"),
));
~~~

The plugin loads the selected language and updates localized text components.
Derive `BevyFluentText` for messages stored in `FluentText<T>`; the demo also
shows locale-change events. Use `#[locale]` for fields refreshed from the
requested locale and `I18nSet` for system ordering. See the [facade][facade] for
the shared missing-message policy.

[facade]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent/README.md
[example]: https://github.com/stayhydated/es-fluent/blob/master/examples/bevy-example/src/lib.rs
[codecov-badge]: https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg?component=es-fluent-manager-bevy
[codecov]: https://codecov.io/gh/stayhydated/es-fluent
[crate-badge]: https://img.shields.io/crates/v/es-fluent-manager-bevy.svg?label=es-fluent-manager-bevy
[crate]: https://crates.io/crates/es-fluent-manager-bevy

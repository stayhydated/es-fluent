# es-fluent-manager-bevy

[![CI](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg)](https://codecov.io/gh/stayhydated/es-fluent)
[![Book](https://img.shields.io/badge/book-online-blue)](https://stayhydated.github.io/es-fluent/book/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-bevy.svg)](https://crates.io/crates/es-fluent-manager-bevy)

Typed localization for Bevy `0.19.x`. The plugin loads configured FTL
resources, updates `FluentText<T>` components when the locale changes,
and exposes `BevyI18n` for direct localization in systems.

Register package resources from a library module:

~~~rust,ignore
es_fluent_manager_bevy::define_i18n_module!();
~~~

Register the plugin:

~~~rust,no_run
use bevy::prelude::*;
use es_fluent_manager_bevy::I18nPlugin;
use unic_langid::langid;

App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(I18nPlugin::with_language(langid!("en")))
    .run();
~~~

Derive `BevyFluentText` for values used directly as
`FluentText<T>`. Use `#[locale]` on named fields that must
refresh from the requested locale, and use `I18nSet` when application
systems need explicit ordering around localization phases.

Configured packages call `es_fluent_build::track_i18n_assets()` from Cargo's
selected custom-build target. Derived fallback-locale messages are compile-time
checked by default. Set `missing_message_policy = "fallback-str"` in the owning
package's `i18n.toml` when `BevyI18n` and `FluentText<T>` should use snake_case
field, variant, or type names after locale fallback is exhausted.

# es-fluent-manager-macros

[![Docs](https://docs.rs/es-fluent-manager-macros/badge.svg)](https://docs.rs/es-fluent-manager-macros/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-macros.svg)](https://crates.io/crates/es-fluent-manager-macros)

Compile-time module registration and Bevy text macros shared by the embedded,
Dioxus, and Bevy managers.

Applications should call the re-exported macro from their concrete manager:

~~~rust,ignore
es_fluent_manager_embedded::define_i18n_module!();
// or:
// es_fluent_manager_dioxus::define_i18n_module!();
// es_fluent_manager_bevy::define_i18n_module!();
~~~

Place the call in a library-reachable module so CLI discovery and runtime
registration use the same package owner. Depend on this crate directly only
when implementing a custom manager macro integration.

Because these macros scan locale assets at compile time, use
[`es-fluent-build`](../es-fluent-build/README.md) to track additions,
removals, and renames.

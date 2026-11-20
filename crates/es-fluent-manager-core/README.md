# es-fluent-manager-core

[![Codecov: es-fluent-manager-core][codecov-badge]][codecov]
[![crates.io: es-fluent-manager-core][crate-badge]][crate]

Shared runtime contracts for custom `es-fluent` manager integrations. Depend
on this crate directly to implement module registration or a localizer, use
`FluentManager`, or plan resources. Applications normally choose a concrete
[embedded][embedded-manager], [Dioxus][dioxus-manager], or [Bevy][bevy-manager]
manager alongside the [`es-fluent`][facade] facade.

Requires Rust 1.99 or newer.

The [public contracts][api] describe module discovery, language selection,
typed keys, and final Fluent bundle lookup. A module creates one localizer for
each selected locale. Custom modules can build a `ModuleLocalizer` from each
fallback locale's resources; it assembles the fallback-chain bundles once and
serves lookups from them. `FluentManager::from_modules` also accepts modules
created at runtime, such as asset-loaded ones. `EmbeddedI18nModule` serves
locale tables compiled into the binary by the embedded manager's registration
macro.

[facade]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent/README.md
[embedded-manager]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-manager-embedded/README.md
[dioxus-manager]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-manager-dioxus/README.md
[bevy-manager]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-manager-bevy/README.md
[api]: https://docs.rs/es-fluent-manager-core/latest/es_fluent_manager_core/
[codecov-badge]: https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg?component=es-fluent-manager-core
[codecov]: https://codecov.io/gh/stayhydated/es-fluent
[crate-badge]: https://img.shields.io/crates/v/es-fluent-manager-core.svg?label=es-fluent-manager-core
[crate]: https://crates.io/crates/es-fluent-manager-core

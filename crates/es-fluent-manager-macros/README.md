# es-fluent-manager-macros

[![Codecov: es-fluent-manager-macros][codecov-badge]][codecov]
[![crates.io: es-fluent-manager-macros][crate-badge]][crate]

Compile-time module registration and Bevy text macros shared by the concrete
[`es-fluent`][facade] managers. Applications use `define_i18n_module!` from the
[embedded][embedded] or [Bevy][bevy] manager's default `macros` feature, or the
[Dioxus][dioxus] manager's unconditional re-export. Depend on this crate directly
when implementing a custom manager macro integration.

Requires Rust 1.99 or newer.

The macro call belongs in a library-reachable module so CLI inventory discovery
and runtime registration use the same package owner. [`es-fluent-build`][build]
tracks locale additions, removals, and renames for these compile-time macros.
The [maintained embedded module][example] demonstrates the re-exported call;
Bevy additionally re-exports `BevyFluentText` for localized components.

[facade]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent/README.md
[embedded]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-manager-embedded/README.md
[bevy]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-manager-bevy/README.md
[dioxus]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-manager-dioxus/README.md
[build]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-build/README.md
[example]: https://github.com/stayhydated/es-fluent/blob/master/examples/readme/src/i18n.rs
[codecov-badge]: https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg?component=es-fluent-manager-macros
[codecov]: https://codecov.io/gh/stayhydated/es-fluent
[crate-badge]: https://img.shields.io/crates/v/es-fluent-manager-macros.svg?label=es-fluent-manager-macros
[crate]: https://crates.io/crates/es-fluent-manager-macros

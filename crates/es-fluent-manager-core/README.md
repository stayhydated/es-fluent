# es-fluent-manager-core

[![Codecov: es-fluent-manager-core][codecov-badge]][codecov]
[![crates.io: es-fluent-manager-core][crate-badge]][crate]

Shared runtime contracts for custom `es-fluent` manager integrations. Depend
on this crate directly to implement module registration or a localizer, use
`FluentManager`, or plan resources. Applications normally choose a concrete
[embedded][embedded-manager], [Dioxus][dioxus-manager], or [Bevy][bevy-manager]
manager alongside the [`es-fluent`][facade] facade.

Requires Rust 1.99 or newer.

## Features

The default feature set enables `embedded`.

| Feature | Default | Purpose |
| --- | --- | --- |
| `embedded` | Yes | Provides `rust-embed` asset discovery and embedded module localizers. |
| `debug-embed` | No | Enables `embedded` and `rust-embed/debug-embed` for native debug assets. |

Disable `embedded` when an asset-backed integration does not use `rust-embed`.
The [public contracts][api] describe module discovery, language selection,
typed keys, and final Fluent bundle lookup.

[facade]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent/README.md
[embedded-manager]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-manager-embedded/README.md
[dioxus-manager]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-manager-dioxus/README.md
[bevy-manager]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-manager-bevy/README.md
[api]: https://docs.rs/es-fluent-manager-core/latest/es_fluent_manager_core/
[codecov-badge]: https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg?component=es-fluent-manager-core
[codecov]: https://codecov.io/gh/stayhydated/es-fluent
[crate-badge]: https://img.shields.io/crates/v/es-fluent-manager-core.svg?label=es-fluent-manager-core
[crate]: https://crates.io/crates/es-fluent-manager-core

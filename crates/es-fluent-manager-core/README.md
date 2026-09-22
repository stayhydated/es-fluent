# es-fluent-manager-core

[![Codecov: es-fluent-manager-core][codecov-badge]][codecov]
[![crates.io: es-fluent-manager-core][crate-badge]][crate]

Shared runtime contracts for custom `es-fluent` manager
integrations. The crate provides `FluentManager`, localization module
registration, language-selection policy, typed message keys and arguments,
resource plans, and optional embedded-asset support.

Most applications should use a concrete manager:

- [Embedded][embedded-manager]
- [Dioxus][dioxus-manager]
- [Bevy][bevy-manager]

Custom integrations construct a manager, select a language, and keep typed keys
until the final Fluent bundle lookup:

~~~rust,no_run
use es_fluent_manager_core::FluentManager;
use unic_langid::langid;

fn main() -> std::io::Result<()> {
    let manager = FluentManager::try_new_with_discovered_modules()
        .map_err(|errors| std::io::Error::other(format!("{errors:?}")))?;
    manager
        .select_language(&langid!("en"))
        .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
    Ok(())
}
~~~

Disable the default `embedded` feature when an asset-backed
integration does not need `rust-embed`.

[embedded-manager]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-manager-embedded/README.md
[dioxus-manager]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-manager-dioxus/README.md
[bevy-manager]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-manager-bevy/README.md
[codecov-badge]: https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg?component=es-fluent-manager-core
[codecov]: https://codecov.io/gh/stayhydated/es-fluent
[crate-badge]: https://img.shields.io/crates/v/es-fluent-manager-core.svg?label=es-fluent-manager-core
[crate]: https://crates.io/crates/es-fluent-manager-core

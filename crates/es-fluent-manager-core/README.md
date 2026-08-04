# es-fluent-manager-core

[![Docs](https://docs.rs/es-fluent-manager-core/badge.svg)](https://docs.rs/es-fluent-manager-core/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-core.svg)](https://crates.io/crates/es-fluent-manager-core)

Shared runtime contracts for custom `es-fluent` manager
integrations. The crate provides `FluentManager`, localization module
registration, language-selection policy, typed message keys and arguments,
resource plans, and optional embedded-asset support.

Most applications should use a concrete manager:

- [Embedded](../es-fluent-manager-embedded/README.md)
- [Dioxus](../es-fluent-manager-dioxus/README.md)
- [Bevy](../es-fluent-manager-bevy/README.md)

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
integration does not need `rust-embed`. See the
[Rust API documentation](https://docs.rs/es-fluent-manager-core/) for extension
contracts.

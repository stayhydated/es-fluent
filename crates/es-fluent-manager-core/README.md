[![Docs](https://docs.rs/es-fluent-manager-core/badge.svg)](https://docs.rs/es-fluent-manager-core/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-core.svg)](https://crates.io/crates/es-fluent-manager-core)

# es-fluent-manager-core

The `es-fluent-manager-core` crate defines the runtime contracts shared by the
`es-fluent` managers. It owns the common manager, module, localizer, fallback,
and resource-planning abstractions used by both embedded and asset-based runtime
integrations.

## Key API

- `FluentManager`: central runtime entry point for selecting locales and formatting
  messages
- `I18nModule` and `I18nModuleRegistration`: discovery and registration contracts
  for localization modules
- `Localizer`: runtime formatter interface used by managers
- `EmbeddedAssets` and `EmbeddedI18nModule`: reusable support for embedded assets
- `ModuleData`, `I18nModuleDescriptor`, and resource-plan helpers for asset-driven
  managers such as Bevy

## Who should use it

Most applications should use a concrete manager crate instead:

- [`es-fluent-manager-embedded`](../es-fluent-manager-embedded/README.md)
- [`es-fluent-manager-bevy`](../es-fluent-manager-bevy/README.md)

Reach for `es-fluent-manager-core` directly when building a custom runtime
integration or reusing the shared fallback and module-registration logic.

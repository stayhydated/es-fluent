[![Docs](https://docs.rs/es-fluent-manager-core/badge.svg)](https://docs.rs/es-fluent-manager-core/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-core.svg)](https://crates.io/crates/es-fluent-manager-core)

# es-fluent-manager-core

The `es-fluent-manager-core` crate defines the runtime contracts shared by the
`es-fluent` managers. It owns the common manager, module, localizer, fallback,
and resource-planning abstractions used by both embedded and asset-based runtime
integrations.

## Key API

- `FluentManager`: central runtime entry point for selecting locales and formatting
  messages, with optional domain-scoped lookup via `localize_in_domain`
- `I18nModule` and `I18nModuleRegistration`: discovery and registration contracts
  for localization modules
- `try_filter_module_registry` and `FluentManager::try_new_with_discovered_modules`:
  strict discovery helpers that fail fast on invalid metadata or repeated
  registrations of the same kind
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

`FluentManager::localize()` remains a first-match search across discovered
localizers. If your application needs explicit routing, prefer
`FluentManager::localize_in_domain()` and keep domains unique.

If you want startup to fail on registry conflicts instead of logging and
skipping them, construct the manager through the strict path:

```rust
use es_fluent_manager_core::FluentManager;

let manager = FluentManager::try_new_with_discovered_modules()
    .expect("registry conflicts must be fixed before startup");
```

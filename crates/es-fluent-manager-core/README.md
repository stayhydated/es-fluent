[![Docs](https://docs.rs/es-fluent-manager-core/badge.svg)](https://docs.rs/es-fluent-manager-core/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-core.svg)](https://crates.io/crates/es-fluent-manager-core)

# es-fluent-manager-core

The `es-fluent-manager-core` crate defines the runtime contracts shared by the
`es-fluent` managers. It owns the common manager, module, localizer, fallback,
and resource-planning abstractions used by embedded, Dioxus, and asset-based
runtime integrations.

## Key API

- `FluentManager`: central runtime entry point for selecting locales and formatting
  messages after an initial `select_language(...)` call, with optional
  domain-scoped lookup via `localize_in_domain`
- `DiscoveredRuntimeI18nModules`: cached, validated runtime-capable module
  discovery for integrations that need many request-local managers without
  repeating inventory validation. Metadata-only registrations are validated but
  are not stored in this cache.
- `LanguageSelectionPolicy` plus `FluentManager::select_language_strict()`: choose
  between best-effort locale switching and transactional switching
- `I18nModule` and `I18nModuleRegistration`: discovery and registration contracts
  for localization modules
- `I18nModuleRegistration::contributes_to_language_selection()`: lets utility
  runtime modules follow locale changes without making unsupported locales look
  supported
- `FluentManager::new_with_discovered_modules()` and
  `FluentManager::try_new_with_discovered_modules()`: strict discovery helpers
  that fail fast on invalid metadata or repeated registrations of the same kind
- `Localizer`: runtime formatter interface used by managers
- `EmbeddedAssets` and `EmbeddedI18nModule`: reusable support for embedded assets
- `BundleBuildError`: structured diagnostics for embedded locale switches that
  fail while assembling a Fluent bundle
- `ModuleData`, `I18nModuleDescriptor`, and resource-plan helpers for asset-driven
  managers such as Bevy

## Who should use it

Most applications should use a concrete manager crate instead:

- [`es-fluent-manager-embedded`](../es-fluent-manager-embedded/README.md)
- [`es-fluent-manager-dioxus`](../es-fluent-manager-dioxus/README.md)
- [`es-fluent-manager-bevy`](../es-fluent-manager-bevy/README.md)

Reach for `es-fluent-manager-core` directly when building a custom runtime
integration or reusing the shared fallback and module-registration logic.

`FluentManager::localize()` is a first-match search across discovered runtime
localizers. Prefer typed `localize_message(...)` wrappers or
`FluentManager::localize_in_domain()` for multi-module apps; use
`localize(...)` directly only for simple single-domain apps or intentional
first-match lookup.

Strict discovery is now the default constructor behavior. Construction does not
select a language, so custom runtime integrations must select the initial
language before lookup:

```rust,no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use es_fluent_manager_core::FluentManager;
use unic_langid::langid;

let manager = FluentManager::new_with_discovered_modules();
manager.select_language(&langid!("en"))?;

let value = manager.localize_in_domain("app", "hello", None);
# let _ = value;
# Ok(())
# }
```

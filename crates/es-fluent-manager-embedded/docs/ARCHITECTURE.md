# es-fluent-manager-embedded Architecture

`es-fluent-manager-embedded` adapts embedded Fluent assets to an explicit runtime
manager handle.

## Runtime state

The crate returns `EmbeddedI18n`, which owns an `Arc<FluentManager>`. It does not
publish that manager into `es-fluent` global state.

```rust
let i18n = EmbeddedI18n::try_new_with_language(langid!("en-US"))?;
let text = i18n.localize_message(&message);
```

## Initialization

`EmbeddedI18n::try_new()` performs strict module discovery and builds a manager
without selecting a locale.

`EmbeddedI18n::try_new_with_language(...)` performs strict discovery, selects the
initial locale, and returns the initialized handle only after successful language
selection.

## Language changes

Language changes are scoped to the `EmbeddedI18n` value:

```rust
i18n.select_language(langid!("fr"))?;
```

Failed switches keep the previous ready locale active because the underlying
`FluentManager` only publishes accepted localizers after successful selection.

## Macro integration

`define_i18n_module!` is re-exported from
`es-fluent-manager-macros::define_embedded_i18n_module`. It generates embedded
asset registration and inventory metadata consumed by `FluentManager` discovery.

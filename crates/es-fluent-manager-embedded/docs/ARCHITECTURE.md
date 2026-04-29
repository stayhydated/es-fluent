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

`select_language(...)` uses the shared best-effort policy, allowing modules that
do not support the requested locale to be skipped when at least one module can
serve it. `select_language_strict(...)` keeps transactional behavior for callers
that require all discovered modules to accept the locale.

## Lookup helpers

`EmbeddedI18n` implements `FluentLocalizer`, so typed messages use
`localize_message(...)` and direct string IDs can use `localize(...)` or
`localize_in_domain(...)`. The inherent fallback helpers mirror
`FluentLocalizerExt`, including `localize_message_silent(...)` for callers that
want ID fallback without warning logs.

## Macro integration

`define_i18n_module!` is re-exported from
`es-fluent-manager-macros::define_embedded_i18n_module`. It generates embedded
asset registration and inventory metadata consumed by `FluentManager` discovery.

# es-fluent-derive Architecture

`es-fluent-derive` emits typed Fluent metadata and explicit-context runtime
implementations.

## Macro output

`#[derive(EsFluent)]` emits:

1. `impl FluentMessage`, which renders through a caller-provided localization
   closure.
2. inventory metadata used by generation and validation.

It does not emit context-free display/localization implementations.

`#[derive(EsFluentVariants)]` emits generated unit enums that also implement
`FluentMessage` and inventory metadata.

`#[derive(EsFluentThis)]` emits `ThisFtl::this_ftl(localizer)`, which resolves a
type-level key through an explicit `FluentLocalizer`.

Enum derives can override the generated base key with `resource = "..."`, route
lookup through an explicit manager domain with `domain = "..."`, and opt out of
inventory collection with `skip_inventory`. Variant-level `key = "..."`
overrides the key suffix. Field-level `skip`, `choice`, `value`, and
`arg_name` affect the generated argument map before it reaches the localization
closure.

`#[derive(EsFluentVariants)]` shares the same generated-enum path for structs
and enums. `keys = [...]` creates keyed generated enums, `derive(...)` adds
traits to those enums, `namespace = ...` routes their inventory metadata, and
`#[fluent_variants(skip)]` filters individual fields or variants.

## Nested values

Generated argument insertion uses `FluentArgumentValue` autoref dispatch:

- nested `FluentMessage` values are rendered with the same localization closure
  as the outer message;
- ordinary values fall back to `Into<FluentValue>`.

This keeps all runtime localization scoped to the caller-provided manager.

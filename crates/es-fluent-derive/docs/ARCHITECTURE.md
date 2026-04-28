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

## Nested values

Generated argument insertion uses `FluentArgumentValue` autoref dispatch:

- nested `FluentMessage` values are rendered with the same localization closure
  as the outer message;
- ordinary values fall back to `Into<FluentValue>`.

This keeps all runtime localization scoped to the caller-provided manager.

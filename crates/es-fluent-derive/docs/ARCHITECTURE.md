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

`#[derive(EsFluentLabel)]` emits `FluentLabel::localize_label(localizer)`, which resolves a
type-level key through an explicit `FluentLocalizer`.

Enum derives can override the generated base key with `resource = "..."`, route
lookup through an explicit manager domain with `domain = "..."`, and opt out of
inventory collection with `skip_inventory`. Variant-level `key = "..."`
overrides the key suffix. Field-level `skip`, `default`, `arg`, `choice`,
`optional`, and `value` affect the generated argument map before it reaches the
localization closure. `choice`, `optional`, and `value` are mutually exclusive
field strategies.

The derive layer consumes typed accessors from `es-fluent-derive-core` for
field argument names, variant keys, enum resource IDs, and enum domains, so
literal attribute spans remain available until diagnostics or semantic model
creation.
It also consumes derive-core lowered container models for `EsFluent`,
`EsFluentVariants`, `EsFluentLabel`, and `EsFluentChoice`, so token emission
sees named fields, tuple fields, generated variant seeds, label type kinds, and
choice variants only after the Rust shape has been checked.
Generated message IDs are constructed as typed semantic values in derive-core
before token emission; the derive crate stringifies them only when writing
runtime calls and inventory metadata.
Namespace values are also carried with spans through derive-core and the derive
namespace resolver. Labels and generated variant enums may inherit a container
namespace, but multiple namespace sources for the same generated output are
reported as attribute conflicts instead of being resolved by precedence.

`#[derive(EsFluentVariants)]` shares the same generated-enum path for structs
and enums. `keys = [...]` creates keyed generated enums, `derive(...)` adds
traits to those enums through the semantic `GeneratedEnumModel`, `namespace =
...` routes their inventory metadata, and `#[fluent_variants(skip)]` filters
individual fields or variants.

## Nested values

Generated argument insertion uses `FluentArgumentValue` autoref dispatch:

- nested `FluentMessage` values are rendered with the same localization closure
  as the outer message;
- ordinary values fall back to `Into<FluentValue>`.

The chosen runtime value strategy (`borrowed`, `optional`, `choice`, or
explicit `value = ...` transform) is stored in the semantic `ArgumentModel`
before token emission, so metadata and insertion logic describe the same
argument entry. Optional omission is driven by explicit
`#[fluent(optional)]`; the derive layer does not infer optional behavior from
the Rust type syntax.

`#[derive(EsFluentChoice)]` builds a semantic `ChoiceModel` before token
emission. The model owns the final `rename_all` mapping for each variant, and
the generated `as_fluent_choice` match arms only consume that mapping.

This keeps all runtime localization scoped to the caller-provided manager.

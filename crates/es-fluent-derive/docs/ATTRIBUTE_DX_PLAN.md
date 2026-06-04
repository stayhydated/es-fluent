# Attribute DX Compile Error Plan

This plan tracks follow-up improvements for attribute combinations that should
be reported earlier or with clearer compiler diagnostics.

## Current Repo State

As of 2026-06-04, this plan is implemented end to end:

- The hidden marker-supertrait pattern now covers both cross-derive contracts:
  `origin` requires `EsFluentLabel`, and `variants` requires
  `EsFluentVariants`.
- `crates/es-fluent-derive/tests/ui/fluent_variants_origin_requires_label.rs`
  covers the inverse `origin` guard.
- `crates/es-fluent-derive/tests/ui/fluent_label_variants_requires_variants.rs`
  covers the reciprocal `variants` guard.
- `crates/es-fluent-derive/tests/renamed_facade.rs` already has a passing
  renamed-facade fixture with both `EsFluentLabel` and `EsFluentVariants` on a
  type using `#[fluent_label(origin, variants)]`.
- Empty generated-variants output is rejected when explicit
  `#[fluent_variants(keys = ...)]` or `#[fluent_label(variants)]` output was
  requested.
- Selector arguments now emit a spanned `EsFluentChoice` trait-bound helper
  before conversion, with passing coverage for derived, manual, generic, and
  borrowed and nested selector types.
- Bevy `#[locale]` fields preserve field types and attribute spans, and refresh
  generation now emits an early `TryFrom<&LanguageIdentifier>` assertion before
  the existing assignment path.

## Guiding Rule

Prefer direct derive-core validation when one derive macro can see the full
invalid state. Use the hidden marker-supertrait pattern only for cross-derive
contracts, where two independent derive macros need to prove that they were both
expanded on the same Rust type.

The marker pattern should stay behind `es_fluent::__private`, remain generic
aware, and be tested with a renamed `es-fluent` facade so generated paths do not
assume the dependency name.

## 1. Require `EsFluentVariants` For `#[fluent_label(variants)]`

Status: Complete. `EsFluentVariants` emits
`EsFluentVariantsLabelWasDerived` when it processes
`#[fluent_label(variants)]`, and `EsFluentLabel` emits the reciprocal
`EsFluentLabelVariantsRequiresEsFluentVariants` requirement when `variants` is
requested.

Problem: `#[fluent_label(variants)]` is only useful when
`#[derive(EsFluentVariants)]` also runs. Without that derive, the user expects
variant-label metadata but only gets the label derive output.

Implementation:

- Add private marker traits in `crates/es-fluent/src/lib.rs`, mirroring the
  existing `origin` guard.
- Have `EsFluentVariants` emit a marker when it actually processes
  `#[fluent_label(variants)]`.
- Have `EsFluentLabel` emit a requirement impl when `variants` is requested.
- Preserve support for generics and where clauses through `split_for_impl()`.
- Keep the check independent of derive order.

Validation:

- Add a trybuild failure for `#[derive(EsFluentLabel)]` with
  `#[fluent_label(origin, variants)]` but no `EsFluentVariants`.
- Add a passing trybuild case with both derives. The renamed-facade integration
  fixture already covers a valid combined derive with the facade dependency
  renamed, so either extend that fixture to assert the new marker path or add a
  focused equivalent.
- Update codegen snapshots for the new marker and requirement impls.

Doc impact:

- Update public docs only if the user-facing wording changes. The docs already
  describe `variants` as a combination with `EsFluentVariants`, so this may only
  need a short note that missing the derive is rejected.

## 2. Reject Empty `#[fluent_variants(keys = ...)]` Output

Status: Complete. `EsFluentVariantsExpansion` rejects explicit
`keys = [...]` when all generated variant seeds are filtered out.

Problem: explicit `keys = [...]` asks for generated enums. If all fields or
variants are skipped, or the source shape produces no generated members, the
macro currently has no useful work to emit.

Implementation:

- Add direct validation in `EsFluentVariantsExpansion`, after skip filtering and
  target construction.
- Trigger only when the user explicitly supplied `keys = [...]`.
- Report the error on the `keys` attribute span when available.
- Phrase the message around the actionable problem: no unskipped fields or
  variants remain for generated variant enums.

Validation:

- Add trybuild failures for a struct and enum where `keys = [...]` is present
  and every possible generated member is skipped.
- Add a unit-level expansion test for the no-target branch so the validation
  stays close to the model builder.

Doc impact:

- No public workflow doc change unless the existing docs imply that empty output
  is accepted.

## 3. Reject Empty `#[fluent_label(variants)]` Output

Status: Complete. The empty-seed branch also rejects
`#[fluent_label(variants)]` when no unskipped generated targets remain.

Problem: `#[fluent_label(variants)]` promises a generated variants-label entry.
If `EsFluentVariants` produces no generated variant targets, the label cannot
describe anything useful.

Implementation:

- Add direct validation in `EsFluentVariantsExpansion`, because it owns the
  filtered targets and generated label model.
- Trigger when `LabelOpts::has_variants()` is true and no variant targets remain.
- Prefer the `variants` flag span in the diagnostic.
- Keep this separate from item 1: item 1 checks that `EsFluentVariants` exists;
  this item checks that it has meaningful output.

Validation:

- Add trybuild failures for `#[fluent_label(variants)]` with all fields or
  variants skipped.
- Add a passing case where at least one unskipped member remains.

Doc impact:

- Add a concise note in the derive README and book only if the existing examples
  need to explain skipped members interacting with generated variant labels.

## 4. Improve `#[fluent(selector)]` Trait Errors

Status: Complete. Selector conversion emits a spanned helper requiring the
selected field type to implement `EsFluentChoice`, and the conversion goes
through that helper instead of surfacing a generated-code method lookup error.

Problem: selector fields rely on `EsFluentChoice`. Missing or invalid selector
implementations can surface as generated-code trait errors instead of a focused
message near the selected field.

Implementation:

- Do not use a derive marker here. Manual `EsFluentChoice` implementations are
  valid and should remain supported.
- Add a small generated assertion near each selector field conversion, with a
  helper function or const that requires the selected field type to implement
  `EsFluentChoice`.
- Use `quote_spanned!` with the selector attribute span so the error points at
  `#[fluent(selector)]`.
- Confirm that references, generics, and nested path types still infer cleanly.

Validation:

- Add trybuild failures for a selector field whose type does not implement
  `EsFluentChoice`.
- Add passing trybuild cases for a derived `EsFluentChoice`, a manual
  `EsFluentChoice` impl, and a generic selector type with an explicit bound.

Doc impact:

- No behavior docs needed unless examples are expanded to show manual
  `EsFluentChoice` implementations.

## 5. Assert Bevy `#[locale]` Field Conversion Earlier

Status: Complete. `crates/es-fluent-manager-macros` preserves locale field
types and attribute spans, emits an early conversion assertion, and has
trybuild coverage for missing and valid conversions.

Problem: `#[locale]` refresh code assigns `TryFrom<&LanguageIdentifier>` output
back into the marked field. Type or trait mismatches can currently appear inside
the generated refresh implementation.

Implementation:

- Keep this in `crates/es-fluent-manager-macros`; it is not an
  `es-fluent-derive` marker problem.
- Generate a lightweight assertion for each `#[locale]` field requiring the
  field type to satisfy `TryFrom<&LanguageIdentifier>`.
- Use the `#[locale]` attribute span when building the assertion.
- Preserve the current assignment path so behavior does not change for valid
  types.

Validation:

- Add trybuild failures in `crates/es-fluent-manager-macros` for a `#[locale]`
  field whose type lacks the conversion.
- Add passing coverage for a generated language enum and a user-defined type
  with a manual `TryFrom<&LanguageIdentifier>` implementation.
- Run `cargo test -p es-fluent-manager-macros` after snapshots are updated.

Doc impact:

- Update the Bevy manager README/book section only if the current wording does
  not clearly state that locale fields must be refreshable from a
  `LanguageIdentifier`.

## Suggested Order

1. Implement item 1 first. It is the direct reciprocal of the existing
   `origin` guard and should reuse the same helper structure.
2. Implement items 2 and 3 together. They are direct validations in the same
   expansion model and share skip-filtering fixtures.
3. Implement item 4 next. It improves a common error path without forbidding
   manual trait implementations.
4. Implement item 5 last. It touches a different macro crate and should be
   validated separately.

## Handoff Checklist

- Add focused trybuild fixtures for every new compile-time failure.
- Update insta snapshots for generated marker or assertion tokens.
- Run `cargo fmt`.
- Run `cargo test -p es-fluent -p es-fluent-derive-core -p es-fluent-derive`
  for derive changes.
- Run `cargo test -p es-fluent-manager-macros` for the Bevy locale follow-up.
- Update README, book, and `.agents/skills/use-es-fluent` guidance only for
  behavior that changes the public application workflow.

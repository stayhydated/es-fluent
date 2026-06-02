# Type-Safe Macro DX Rework Plan

## Assessment

The current shape is viable. The derive pipeline already has the right broad
architecture: raw attribute context checks run before Darling, parsed literals
become shared Fluent newtypes, lowered models separate Rust shape validation
from token emission, and expansion models keep most runtime/inventory metadata
typed until the proc-macro boundary.

The remaining DX gap is not a need for a rewrite. It is that several internal
layers can still represent states that should be impossible after parsing:
field attributes are loose boolean/value bags, tuple fields carry `skipped`
plus `Option<ArgumentModel>`, parent context can be reparsed separately from the
container options, generated enum derives are revalidated late in codegen, and
some adjacent macros still use ad hoc parsing and hard-coded crate paths.

Because backward compatibility is not a goal, we can simplify these surfaces
aggressively instead of preserving legacy attribute forms or intermediate APIs.

## Goals

- Make invalid attribute combinations unrepresentable after the first typed
  parse/lower step.
- Keep source spans attached to the typed value that needs them for diagnostics
  or `quote_spanned!`.
- Keep codegen APIs narrow enough that raw strings cannot be passed where a
  Fluent id, argument name, domain, namespace, or generated key is required.
- Share macro diagnostics and crate-path resolution across all public macro
  surfaces.
- Improve compile-error quality by collecting related attribute mistakes where
  practical.

## Non-Goals

- Preserving old macro internals or legacy attribute syntaxes.
- Reworking runtime localization manager APIs without a direct type-safety or
  macro-DX reason.
- Moving implementation details into READMEs or the book. Public docs should be
  updated only when user-facing behavior changes.

## Phase 1: Close The Attribute Grammar

Current state:

- `grammar.rs` centralizes accepted keys and value shapes, which is good.
- A single `AttributeKey` enum and a flat `ATTRIBUTE_RULES` table still make the
  checker operate as a generic key/location lookup.
- `validate_attribute_for_location` stops at the first invalid item in an
  attribute list.

Rework:

- Split the grammar into typed family specs:
  `FluentSpec`, `FluentVariantsSpec`, `FluentLabelSpec`,
  `FluentChoiceSpec`, and `LanguageSpec`.
- Parse raw `syn::Meta` into a typed `AttributeSet<F>` that enforces:
  unique keys, accepted locations, and expected value shape.
- Keep the current user-facing help text, but attach it to the typed spec rather
  than duplicating static strings around the checker.
- Accumulate all invalid keys/shapes in a single attribute list and emit all
  related diagnostics where the proc-macro boundary supports it.

Acceptance checks:

- No generic `HashMap<AttributeKey, ...>` is needed for per-attribute duplicate
  detection.
- Wrong-location, wrong-shape, duplicate-key, and unknown-key snapshots still
  point at the offending attribute item.
- `#[es_fluent_language(...)]` uses the same typed attribute diagnostic path as
  derive attributes.

## Phase 2: Make Field Strategy Impossible To Misrepresent

Current state:

- `FluentFieldAttributeArgs` stores `skip`, `selector`, `optional`, `value`, and
  `arg` independently.
- `argument_value_strategy` later rejects impossible combinations.
- `ArgumentValueStrategy` is typed, but it is produced after loose option state
  has already crossed several helper boundaries.

Rework:

- Replace loose field flags with a closed directive:

```rust
pub enum FieldDirective {
    Skip,
    Argument(FieldArgumentDirective),
}

pub struct FieldArgumentDirective {
    pub name: Option<SpannedValue<ArgName>>,
    pub value: FieldValueDirective,
}

pub enum FieldValueDirective {
    Borrowed { span: Span },
    Optional { span: Span, inner_ty: syn::Type },
    Choice { span: Span },
    Transform(ValueTransform),
}
```

- Build `FieldDirective` once from raw parsed attributes.
- Move optional field type validation into this construction step and preserve
  the span of the `Option<T>` syntax.
- Make `FluentField` expose `directive()` instead of separate `is_skipped`,
  `is_selector`, `is_optional`, and `value` checks.

Acceptance checks:

- No code outside raw option parsing needs to check `selector + value`,
  `optional + selector`, `optional + value`, or `skip + arg`.
- `field_value_strategy` no longer returns `Option<ArgumentValueStrategy>`.
- Tests cover all conflicting field strategies at the typed directive boundary.

## Phase 3: Preserve Typed Shape Through Expansion

Current state:

- Lowered models introduce index newtypes, but expansion converts some of them
  back to `usize`.
- Enum tuple fields use `skipped: bool` plus `Option<ArgumentModel>`.
- Generated variants targets expose `Vec<syn::Path>` and codegen rebuilds
  `DerivePathList`.

Rework:

- Promote index newtypes that survive into expansion:
  `DeclarationIndex`, `TupleFieldIndex`, and `ExposedArgumentIndex`.
- Replace tuple field state with a closed enum:

```rust
pub enum EsFluentTupleField {
    Skipped { index: TupleFieldIndex },
    Argument {
        index: TupleFieldIndex,
        argument: ArgumentModel,
    },
}
```

- Make `EsFluentVariantsTarget` own a `GeneratedEnumModel` or a typed target
  model that includes `DerivePathList`, generated messages, label metadata,
  namespace, and domain.
- Remove late `DerivePathList::from_paths(...)` from derive crate codegen.

Acceptance checks:

- Codegen does not perform semantic validation; it only turns validated models
  into tokens.
- Expansion structs cannot encode a skipped tuple field that also has an
  argument, or an exposed tuple field without an argument.
- Generated enum derive path errors are produced before entering
  `es-fluent-derive`.

## Phase 4: Parse Parent Context Once

Current state:

- `ContainerContext::from_struct_options` and `from_enum_options` are typed.
- `ContainerContext::from_derive_input` manually reparses parent
  `#[fluent(...)]` attributes for label and variants derives.
- The manual parent parser still has an unused `id` slot.

Rework:

- Introduce a typed `ContainerEnvelope` built once per derive input:

```rust
pub enum ContainerEnvelope {
    Struct(StructContainer),
    Enum(EnumContainer),
}

pub struct StructContainer {
    pub ident: syn::Ident,
    pub generics: syn::Generics,
    pub namespace: Option<SpannedNamespaceRule>,
}

pub struct EnumContainer {
    pub ident: syn::Ident,
    pub generics: syn::Generics,
    pub domain: Option<SpannedValue<DomainName>>,
    pub namespace: Option<SpannedNamespaceRule>,
}
```

- Derive-specific options should borrow or own this envelope instead of
  reparsing parent attributes.
- Remove the unused parent `id` path from non-message derives.
- Validate raw attributes once at the expansion boundary, then pass a
  validated input wrapper into option parsing.

Acceptance checks:

- `context.rs` no longer manually parses `Punctuated<Meta, Token![,]>`.
- Label and variants derives cannot accidentally accept parent keys that their
  location grammar rejected.
- Raw attribute validation is not duplicated for enum variants.

## Phase 5: Narrow Codegen To Typed Token Emitters

Current state:

- Codegen mostly consumes typed semantic values, then stringifies them near
  `quote!`.
- The final `new_unchecked(...)` calls are spread across several emitters.
- Some codegen structs still store raw `String` values for names and docs.

Rework:

- Add small token helpers for the only allowed unchecked emissions:
  `static_domain_tokens`, `static_entry_id_tokens`,
  `static_argument_name_tokens`, and `namespace_rule_tokens`.
- Keep `new_unchecked` calls inside those helpers only.
- Replace raw `String` fields in codegen-facing structs where they encode a
  known category:
  `RustSourceName`, `RustTypeName`, `GeneratedDocName`, or `GeneratedKeyName`.
- Make generated documentation strings the only place where free-form strings
  are still expected.

Acceptance checks:

- `rg "new_unchecked" crates/es-fluent-derive/src` finds only the typed token
  helper functions.
- `InventoryVariantSpec` and `LocalizeCallSpec` cannot be constructed with raw
  message ids, domains, or argument names.
- Snapshot tests still prove the emitted public tokens are unchanged unless a
  deliberate breaking cleanup is made.

## Phase 6: Share Macro Support Beyond EsFluent Derives

Current state:

- `es-fluent-lang-macro` reuses `LanguageMode`, but still has its own crate-path
  resolver and expansion model.
- `es-fluent-manager-macros` has ad hoc `#[locale]` parsing and hard-coded
  crate paths like `::es_fluent_manager_bevy`.

Rework:

- Add a small macro support module or crate for:
  crate path resolution, structured diagnostic conversion, typed attribute
  parsing, and common snapshot helpers.
- Move the language macro to an explicit expansion model similar to the derive
  expansion models.
- Give `#[locale]` a typed attribute spec so wrong shape and wrong location
  diagnostics match the rest of the macro ecosystem.
- Resolve manager crate paths with `proc_macro_crate`, including renamed
  dependency tests.

Acceptance checks:

- Renamed dependency tests exist for manager macros, not just the language
  macro and derive facade.
- `#[locale(...)]`, wrong target shapes, and unsupported tuple fields have
  consistent structured diagnostics.
- Hard-coded public crate paths are limited to fallback branches.

## Phase 7: Tighten Runner And CLI Type Boundaries

Current state:

- Runner protocol payloads already use typed Fluent ids, argument names, source
  lines, package names, and i18n paths.
- Some CLI-facing discovery structs still store crate names and paths as raw
  `String` or `PathBuf` and later rely on `expect(...)` when constructing typed
  runner requests.

Rework:

- Store discovered crate names as `PackageName` in `CrateInfo`.
- Change `RunnerMetadataStore` methods to accept `&PackageName` instead of
  `&str`.
- Introduce typed wrappers for manifest directory and source directory when the
  path has already passed discovery validation.
- Push path-to-string conversion to the runner-generation edge and keep the
  rest of the CLI model path-typed.

Acceptance checks:

- Production CLI code does not call `PackageName::try_new(...).expect(...)` for
  discovered crates.
- Metadata paths cannot be built from unvalidated crate name strings.
- Existing runner JSON round-trip tests still pass.

## Phase 8: Remove Legacy Surface Area Deliberately

Current state:

- Some tests and diagnostics explicitly mention rejected legacy formats, which
  is useful now but should not force old concepts to remain in the model.

Rework:

- Keep user-facing diagnostics for common mistakes, but do not preserve legacy
  parser branches internally.
- Prefer one accepted spelling per concept:
  flags are bare flags, booleans are explicit booleans, expressions are Rust
  expressions, namespaces are literal or known rule identifiers.
- Remove internal APIs whose only purpose is to bridge old attribute shapes.

Acceptance checks:

- Tests assert rejected syntax only at the public macro boundary.
- Internal option and semantic models do not contain names like `legacy`,
  `old_format`, or compatibility-specific branches.

## Suggested Implementation Order

1. Field directives first. This gives the most immediate internal type-safety
   win without changing public macro output.
2. Typed expansion shape next, especially tuple field and generated enum target
   models.
3. Parent context parsing cleanup after expansion is stable.
4. Attribute grammar closure and diagnostic aggregation.
5. Codegen token helper narrowing.
6. Shared macro support for language and manager macros.
7. Runner/CLI type boundary cleanup.
8. Public documentation updates only for intentional user-facing behavior
   changes.

## Validation Strategy

- For derive-core-only changes:
  `cargo test -p es-fluent-derive-core`
- For proc-macro token changes:
  `cargo test -p es-fluent-derive --all-targets`
- For language or manager macro changes:
  `cargo test -p es-fluent-lang-macro --all-targets`
  and `cargo test -p es-fluent-manager-macros --all-targets`
- For runner/CLI type-boundary changes:
  `cargo test -p es-fluent-runner -p es-fluent-cli --all-targets`
- For cross-surface macro behavior:
  use the narrow crate tests above first, then `just check` if the change spans
  public examples, generated output, or multiple macro crates.

## Current Recommendation

Do not rewrite the whole architecture. Keep the current pipeline, but make the
intermediate models more closed and typed. The highest-value rework is to move
from "loose parsed options plus validation helpers" to "validated directives and
expansion models that cannot encode invalid states." That lines up directly
with the project goal of excellent DX: clearer internals, better spans, fewer
late failures, and fewer places where codegen has to defend against impossible
input.

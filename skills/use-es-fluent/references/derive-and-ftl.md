# Derive and FTL Patterns

Use this reference when adding localizable Rust types or explaining how generated FTL should look.

## Message Derive

`#[derive(EsFluent)]` turns structs and enum variants into typed messages:

```rust
use es_fluent::EsFluent;

#[derive(EsFluent)]
pub enum LoginError {
    InvalidPassword,
    UserNotFound { username: String },
    Something(String, String, String),
    SomethingArgNamed(
        #[fluent(arg = "input")] String,
        #[fluent(arg = "expected")] String,
        #[fluent(arg = "details")] String,
    ),
}

#[derive(EsFluent)]
pub struct WelcomeMessage<'a> {
    pub name: &'a str,
    pub count: i32,
}
```

Typical generated fallback FTL:

```ftl
## LoginError

login_error-InvalidPassword = Invalid Password
login_error-UserNotFound = User Not Found { $username }
login_error-Something = Something { $f0 } { $f1 } { $f2 }
login_error-SomethingArgNamed = Something Arg Named { $input } { $expected } { $details }

## WelcomeMessage

welcome_message = Welcome Message { $name } { $count }
```

Use `i18n.localize_message(&value)` through the concrete manager or integration context.

## Field and Variant Attributes

Common `#[fluent(...)]` attributes:

- `arg = "..."`: rename an exposed Fluent argument.
- `skip`: exclude a field from arguments, or on a single-field enum variant delegate rendering to the wrapped message.
- `value = |x: &String| x.len()`: transform a field before inserting it as a Fluent argument.
- Plain `Option<T>` fields are inferred as optional Fluent arguments and are omitted when `None`.
- `selector` on `Option<T>` fields creates an optional selector argument.
- `selector` and `value = ...` cannot be combined on the same field. Explicit value attributes override `Option<T>` inference.
- `key = "..."`: override an enum variant key suffix. On unit-only `EsFluent` enums, this also overrides the inferred selector value.
- `skip` and `key = "..."` cannot be combined on the same enum variant.
- `id = "..."`: override an enum base key.
- `domain = "..."`: route enum lookup to a specific manager domain.

`id` and `domain` are enum-only. Struct message containers accept `namespace = ...`.

Generated FTL keys must be unique within each output file. `cargo es-fluent generate`, `clean`, and `check` fail when two derived items produce the same key.

Transparent wrapper variants:

```rust
use es_fluent::EsFluent;

#[derive(EsFluent)]
pub enum NetworkError {
    ApiUnavailable,
}

#[derive(EsFluent)]
pub enum TransactionError {
    #[fluent(skip)]
    Network(NetworkError),
}
```

Only the wrapped message needs FTL:

```ftl
## NetworkError

network_error-ApiUnavailable = API is unavailable
```

## Choices

Use unit-only `EsFluent` enums for selector values inside another message:
variants serialize as kebab-case by default, so `GenderChoice::Male` becomes
`male` and a compound variant like `VeryFriendly` becomes `very-friendly`.
Derived choice values are emitted as validated `StaticFluentVariantKey` values.
Use `#[fluent_choice(rename_all = "...")]` on the same enum to change selector casing. Styles that generate invalid selector values, such as values containing spaces, are rejected at compile time. Use standalone `#[derive(EsFluentChoice)]` only for selector enums that should not also be registered as messages.

```rust
use es_fluent::EsFluent;

#[derive(EsFluent)]
pub enum GenderChoice {
    Male,
    Female,
    Other,
}

#[derive(EsFluent)]
pub struct Greeting<'a> {
    pub name: &'a str,
    #[fluent(selector)]
    pub gender: Option<&'a GenderChoice>,
}
```

FTL can branch on the serialized selector:

```ftl
greeting =
    { $gender ->
        [male] Welcome, Mr. { $name }
        [female] Welcome, Ms. { $name }
       *[other] Welcome, { $name }
    }
```

## Variants and Labels

Use `EsFluentVariants` to generate message enums for struct fields or enum variants:

```rust
use es_fluent::{EsFluent, EsFluentVariants};

#[derive(EsFluentVariants)]
#[fluent_variants(keys = ["label", "description"])]
pub struct LoginFormVariants {
    pub username: String,
    pub password: String,
}

#[derive(EsFluent)]
pub struct ActiveFormField {
    #[fluent(selector)]
    pub field: LoginFormVariantsLabelVariants,
}

#[derive(EsFluentVariants)]
pub enum SettingsTab {
    General,
    Notifications,
    Privacy,
}
```

Typical usage:

```rust
let username = i18n.localize_message(&LoginFormVariantsLabelVariants::Username);
let active = i18n.localize_message(&ActiveFormField {
    field: LoginFormVariantsLabelVariants::Username,
});
let tab = i18n.localize_message(&SettingsTabVariants::Notifications);
```

Generated variant enums implement `EsFluentChoice`, so they can be used directly in `#[fluent(selector)]` fields. `keys = [...]` values must be lowercase snake_case. Use `#[fluent_variants(skip)]` to omit a field or variant. Generated enums derive `Clone`, `Copy`, `Debug`, `Eq`, `Hash`, and `PartialEq` automatically; use `derive(...)` inside `#[fluent_variants(...)]` for additional traits, not `EsFluentChoice`.

Use `EsFluentLabel` for a type-level label:

```rust
use es_fluent::{EsFluentLabel, FluentLabel as _};

#[derive(EsFluentLabel)]
#[fluent(namespace = "forms")]
pub enum Gender {
    Male,
    Female,
    Other,
}

let title = Gender::localize_label(&i18n);
let maybe_title = Gender::try_localize_label(&i18n);
```

`#[derive(EsFluentLabel)]` generates a type-level label, validated static label metadata, and both fallback and fallible label localization methods. `#[derive(EsFluentVariants)]` also gives generated variant enums label keys inferred from their generated enum names.

## Namespaces

Use namespaces to split generated FTL into files. The same `#[fluent(namespace = ...)]` syntax works with `EsFluent`, `EsFluentLabel`, and `EsFluentVariants`.

Use exactly one namespace source for each generated output. When multiple
derives are combined on one type, either inherit a shared namespace from
`#[fluent(namespace = ...)]` or set one on the specific
`#[fluent_label(...)]` / `#[fluent_variants(...)]` output, but do not combine
those namespace sources.

```rust
#[derive(EsFluent)]
#[fluent(namespace = "ui")] // -> assets_dir/{locale}/{crate}/ui.ftl
struct Button;

#[derive(EsFluent)]
#[fluent(namespace = file)] // -> assets_dir/{locale}/{crate}/{file_stem}.ftl
struct Dialog;

#[derive(EsFluent)]
#[fluent(namespace = file_relative)] // -> assets_dir/{locale}/{crate}/ui/button.ftl
struct Modal;

#[derive(EsFluent)]
#[fluent(namespace = folder)] // -> assets_dir/{locale}/{crate}/{parent_folder}.ftl
struct FolderModal;

#[derive(EsFluent)]
#[fluent(namespace = folder_relative)] // -> assets_dir/{locale}/{crate}/ui.ftl
struct FolderRelativeModal;
```

If `i18n.toml` has `namespaces = [...]`, string namespaces are validated against the allowlist by the compiler and the CLI during `generate` and `watch`.
Literal namespace strings must also be safe locale-relative paths: no empty
segments, `.`/`..`, backslashes, absolute paths, surrounding whitespace, or
`.ftl` suffix.

## Inventory Discovery

Keep derived message types reachable from a library target. The CLI collects derive inventory from library targets. It does not discover binary-only types that live only in `src/main.rs`.

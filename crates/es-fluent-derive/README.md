[![Docs](https://docs.rs/es-fluent-derive/badge.svg)](https://docs.rs/es-fluent-derive/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-derive.svg)](https://crates.io/crates/es-fluent-derive)

# es-fluent-derive

Procedural macros for the `es-fluent` localization system.

This crate is the engine that transforms your Rust structs and enums into Fluent messages. It is designed to be used via the `es-fluent` crate, not directly.

All macros provided by this crate are fully independent and composable. You can use them individually or together on the same type depending on your needs.

## Features

### `#[derive(EsFluent)]`

Turns an enum or struct into a localizable message.

- **Enums**: Each variant becomes a message ID (e.g., `MyEnum::Variant` -> `my_enum-Variant`).
- **Structs**: The struct itself becomes the message ID (e.g., `MyStruct` -> `my_struct`).
- **Fields**: Fields are automatically exposed as arguments to the Fluent message.
- **Typed runtime lookup**: Generated types implement `FluentMessage` for managers that provide an explicit context.

```rs
use es_fluent::EsFluent;

#[derive(EsFluent)]
pub enum LoginError {
    InvalidPassword, // no params
    UserNotFound { username: String }, // exposed as $username in the ftl file
    Something(String, String, String), // exposed as $f0, $f1, $f2 in the ftl file
    SomethingArgNamed(
        #[fluent(arg = "input")] String,
        #[fluent(arg = "expected")] String,
        #[fluent(arg = "details")] String,
    ), // exposed as $input, $expected, $details
}

// Usage with an explicit manager:
// i18n.localize_message(&LoginError::InvalidPassword)
// i18n.localize_message(&LoginError::UserNotFound { username: "john".to_string() })
// i18n.localize_message(&LoginError::Something("a".to_string(), "b".to_string(), "c".to_string()))

#[derive(EsFluent)]
pub struct UserProfile<'a> {
    pub name: &'a str, // exposed as $name in the ftl file
    pub gender: &'a str, // exposed as $gender in the ftl file
}

// usage: i18n.localize_message(&UserProfile { name: "John", gender: "male" })
```

Common derive attributes:

- `arg = "..."` on a field renames that exposed Fluent argument (works on struct fields, enum named fields, and enum tuple fields).
- `#[fluent(skip)]` on a field excludes that field from generated arguments.
- `#[fluent(value = |x: &String| x.len())]` transforms a field before inserting it as a Fluent argument.
- Plain `Option<T>` fields are inferred as optional Fluent arguments and are omitted when `None`.
- `#[fluent(selector)]` on `Option<T>` fields creates an optional selector argument.
- `#[fluent(selector)]` and `#[fluent(value = ...)]` are mutually exclusive on the same field. Explicit value attributes override `Option<T>` inference.
- `#[fluent(key = "...")]` on an enum variant overrides that variant's key suffix. On unit-only `EsFluent` enums, it also overrides the inferred selector value.
- `#[fluent(skip)]` and `#[fluent(key = "...")]` cannot be combined on the same enum variant.
- `#[fluent(id = "...")]` on an enum overrides the base key, and `domain = "..."` routes lookup to a specific manager domain.
- `id = "..."` and `domain = "..."` are enum-only. Struct message containers accept `namespace = ...`; struct messages resolve in the current crate's domain.
- Generated FTL keys must be unique within each output file. `generate`, `clean`, and `check` fail when two derived items produce the same key.
- For namespaced types, `check` validates the expected namespace file; a key in `{crate}.ftl` still counts as missing if the Rust type belongs in `{crate}/{namespace}.ftl`.
- `#[fluent_variants(skip)]` omits a struct field or enum variant from generated variant enums; `keys = [...]` values must be lowercase snake_case.

Skipped single-field enum variants:

`#[fluent(skip)]` on a single-field enum variant suppresses that variant's own
key and delegates context-bound rendering to the wrapped value. This is useful for
transparent wrapper enums.

```rs
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

let _ = i18n.localize_message(&TransactionError::Network(NetworkError::ApiUnavailable));
```

```ftl
## NetworkError

network_error-ApiUnavailable = API is unavailable
```

#### Namespaces (optional)

You can split generated messages into multiple `.ftl` files by adding a namespace:

```rs
#[derive(EsFluent)]
#[fluent(namespace = "ui")] // -> {crate}/ui.ftl
struct Button;

#[derive(EsFluent)]
#[fluent(namespace = file)] // -> {crate}/{file_stem}.ftl
struct Dialog;

#[derive(EsFluent)]
#[fluent(namespace = file_relative)] // -> {crate}/ui/button.ftl
struct Modal;

#[derive(EsFluent)]
#[fluent(namespace = folder)] // -> {crate}/{parent_folder}.ftl
struct FolderModal;

#[derive(EsFluent)]
#[fluent(namespace = folder_relative)] // -> {crate}/ui.ftl
struct FolderRelativeModal;
```

Literal namespace strings must be safe locale-relative paths: no empty
segments, `.`/`..`, backslashes, absolute paths, surrounding whitespace, or
`.ftl` suffix.

The same `#[fluent(namespace = ...)]` syntax also applies to `EsFluentLabel` and `EsFluentVariants`.
Use exactly one namespace source for each generated output: either inherit a
shared namespace from `#[fluent(namespace = ...)]` or set one on
`#[fluent_label(...)]` / `#[fluent_variants(...)]`, but do not combine them.

### Choices

Unit-only enums that derive `EsFluent` can be used _inside_ another message as selectors (e.g., for gender or status). Variants serialize as kebab-case by default, so `Gender::Male` becomes `male` and a compound variant like `VeryFriendly` becomes `very-friendly`.
Derived choice values are emitted as validated `StaticFluentVariantKey` values.
Use `#[fluent_choice(rename_all = "...")]` on the same enum to change selector casing. Styles that generate invalid selector values, such as values containing spaces, are rejected at compile time.
Use standalone `#[derive(EsFluentChoice)]` only for selector enums that should not also be registered as messages.

```rs
use es_fluent::EsFluent;

#[derive(EsFluent)]
pub enum Gender {
    Male,
    Female,
    Other,
}

#[derive(EsFluent)]
pub struct UserProfile<'a> {
    pub name: &'a str,
    #[fluent(selector)] // Matches $gender -> [male]...
    pub gender: Option<&'a Gender>,
}

// usage: i18n.localize_message(&UserProfile { name: "John", gender: Some(&Gender::Male) })
```

### `#[derive(EsFluentVariants)]`

Generates key-value pair enums for struct fields or enum variants. This is
useful for generating UI labels, placeholders, or descriptions for a form
object, and it can also expose enum variants as localizable keys.

```rs
use es_fluent::{EsFluent, EsFluentVariants};

#[derive(EsFluentVariants)]
#[fluent_variants(keys = ["label", "description"])]
#[fluent(namespace = "forms")]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

// Generates enums -> keys:
// LoginFormLabelVariants::{Variants} -> (login_form_label_variants-{variant})
// LoginFormDescriptionVariants::{Variants} -> (login_form_description_variants-{variant})

#[derive(EsFluent)]
pub struct ActiveFormField {
    #[fluent(selector)]
    pub field: LoginFormLabelVariants,
}

// usage: i18n.localize_message(&LoginFormLabelVariants::Username)
// usage: i18n.localize_message(&ActiveFormField { field: LoginFormLabelVariants::Username })

#[derive(EsFluentVariants)]
pub enum SettingsTab {
    General,
    Notifications,
    Privacy,
}

// Generates enum -> keys:
// SettingsTabVariants::{General, Notifications, Privacy}
//     -> (settings_tab_variants-{variant})

// usage: i18n.localize_message(&SettingsTabVariants::Notifications)
```

`keys = [...]` values must be lowercase snake_case. Use
`#[fluent_variants(skip)]` to omit a struct field or enum variant from the
generated enums. Generated enums derive `Clone`, `Copy`, `Debug`, `Eq`, `Hash`,
and `PartialEq` automatically and implement `EsFluentChoice`, so they can be
used directly in `#[fluent(selector)]` fields. Use `derive(...)` inside
`#[fluent_variants(...)]` for additional traits; `EsFluentChoice` is already
inferred.

### `#[derive(EsFluentLabel)]`

Generates a helper implementation of the `FluentLabel` trait and registers the
type's name as a key. This is similar to `EsFluentVariants` (which registers
field- or variant-derived keys), but for the parent type itself.

- `#[derive(EsFluentLabel)]` generates typed label metadata plus `localize_label(localizer)` and `try_localize_label(localizer)`.

```rs
use es_fluent::{EsFluentLabel, FluentLabel as _};

#[derive(EsFluentLabel)]
#[fluent(namespace = "forms")]
pub enum Gender {
    Male,
    Female,
    Other,
}

// Generates key:
// (gender_label)

// usage:
// let _ = Gender::localize_label(&i18n);
// let _ = Gender::try_localize_label(&i18n);
// let _ = Gender::fluent_label_id();
```

`#[derive(EsFluentVariants)]` also gives each generated variant enum a label
key inferred from the generated enum name.

```rs
use es_fluent::{EsFluentLabel, EsFluentVariants, FluentLabel as _};

#[derive(EsFluentVariants, EsFluentLabel)]
#[fluent_variants(keys = ["label", "description"])]
#[fluent(namespace = "forms")]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

// Generates keys:
// (login_form_label_variants_label)
// (login_form_description_variants_label)

// usage: LoginFormDescriptionVariants::localize_label(&i18n)
```

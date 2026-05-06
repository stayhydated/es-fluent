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
        #[fluent(arg_name = "input")] String,
        #[fluent(arg_name = "expected")] String,
        #[fluent(arg_name = "details")] String,
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

- `arg_name = "..."` on a field renames that exposed Fluent argument (works on struct fields, enum named fields, and enum tuple fields).
- `#[fluent(skip)]` on a field excludes that field from generated arguments.
- `#[fluent(value = "...")]` or `#[fluent(value(...))]` transforms a field before inserting it as a Fluent argument.
- `#[fluent(key = "...")]` on an enum variant overrides that variant's key suffix.
- `#[fluent(resource = "...")]` on an enum overrides the base key, `domain = "..."` routes lookup to a specific manager domain, and `skip_inventory` suppresses CLI inventory registration.
- `domain = "..."` is enum-only. Struct messages resolve in the current crate's domain.
- Optional-argument omission is generated for direct `Option<T>` fields, including paths like `std::option::Option<T>`. Type aliases to `Option<T>` are treated like ordinary field types.
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
#[fluent(namespace(file(relative)))] // -> {crate}/ui/button.ftl
struct Modal;

#[derive(EsFluent)]
#[fluent(namespace = folder)] // -> {crate}/{parent_folder}.ftl
struct FolderModal;

#[derive(EsFluent)]
#[fluent(namespace(folder(relative)))] // -> {crate}/ui.ftl
struct FolderRelativeModal;
```

The same `#[fluent(namespace = ...)]` syntax also applies to `EsFluentLabel` and `EsFluentVariants`.

### `#[derive(EsFluentChoice)]`

Allows an enum to be used _inside_ another message as a selector (e.g., for gender or status).

```rs
use es_fluent::{EsFluent, EsFluentChoice};

#[derive(EsFluent, EsFluentChoice)]
#[fluent_choice(serialize_all = "snake_case")]
pub enum Gender {
    Male,
    Female,
    Other,
}

#[derive(EsFluent)]
pub struct UserProfile<'a> {
    pub name: &'a str,
    #[fluent(choice)] // Matches $gender -> [male]...
    pub gender: &'a Gender,
}

// usage: i18n.localize_message(&UserProfile { name: "John", gender: &Gender::Male })
```

### `#[derive(EsFluentVariants)]`

Generates key-value pair enums for struct fields or enum variants. This is
useful for generating UI labels, placeholders, or descriptions for a form
object, and it can also expose enum variants as localizable keys.

```rs
use es_fluent::EsFluentVariants;

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

// usage: i18n.localize_message(&LoginFormLabelVariants::Username)

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
generated enums. Use `derive(Debug, Clone)` inside `#[fluent_variants(...)]` to
add derives to the generated enums.

### `#[derive(EsFluentLabel)]`

Generates a helper implementation of the `FluentLabel` trait and registers the
type's name as a key. This is similar to `EsFluentVariants` (which registers
field- or variant-derived keys), but for the parent type itself.

- `origin`: Enabled by default. `#[derive(EsFluentLabel)]` and `#[derive(EsFluentLabel)] #[fluent_label(origin)]` both generate the type-level label. Use `#[fluent_label(origin = false)]` when deriving only variant labels through `EsFluentVariants`.
- `#[fluent_label(origin)]`: Explicitly generates an implementation where `localize_label(localizer)` returns the base key for the type.

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

// usage: Gender::localize_label(&i18n)
```

- `#[fluent_label(variants)]`: Can be combined with `EsFluentVariants` derives to generate keys for variants.

```rs
use es_fluent::{EsFluentLabel, EsFluentVariants, FluentLabel as _};

#[derive(EsFluentVariants, EsFluentLabel)]
#[fluent_label(origin, variants)]
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

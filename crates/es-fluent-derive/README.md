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

// Usage:
// LoginError::InvalidPassword.to_fluent_string()
// LoginError::UserNotFound { username: "john" }.to_fluent_string()
// LoginError::Something("a", "b", "c").to_fluent_string()

#[derive(EsFluent)]
pub struct UserProfile<'a> {
    pub name: &'a str, // exposed as $name in the ftl file
    pub gender: &'a str, // exposed as $gender in the ftl file
}

// usage: UserProfile { name: "John", gender: "male" }.to_fluent_string()
```

Argument naming attributes:

- `arg_name = "..."` on a field renames that exposed Fluent argument (works on struct fields, enum named fields, and enum tuple fields).

Skipped single-field enum variants:

`#[fluent(skip)]` on a single-field enum variant suppresses that variant's own
key and delegates `to_fluent_string()` to the wrapped value. This is useful for
transparent wrapper enums.

```rs
use es_fluent::{EsFluent, ToFluentString};

#[derive(EsFluent)]
pub enum NetworkError {
    ApiUnavailable,
}

#[derive(EsFluent)]
pub enum TransactionError {
    #[fluent(skip)]
    Network(NetworkError),
}

let _ = TransactionError::Network(NetworkError::ApiUnavailable).to_fluent_string();
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

The same `#[fluent(namespace = ...)]` syntax also applies to `EsFluentThis` and `EsFluentVariants`.

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

// usage: UserProfile { name: "John", gender: &Gender::Male }.to_fluent_string()
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

// usage: LoginFormLabelVariants::Username.to_fluent_string()

#[derive(EsFluentVariants)]
pub enum SettingsTab {
    General,
    Notifications,
    Privacy,
}

// Generates enum -> keys:
// SettingsTabVariants::{General, Notifications, Privacy}
//     -> (settings_tab_variants-{variant})

// usage: SettingsTabVariants::Notifications.to_fluent_string()
```

### `#[derive(EsFluentThis)]`

Generates a helper implementation of the `ThisFtl` trait and registers the
type's name as a key. This is similar to `EsFluentVariants` (which registers
field- or variant-derived keys), but for the parent type itself.

- `#[fluent_this(origin)]`: Generates an implementation where `this_ftl()` returns the base key for the type.

```rs
use es_fluent::EsFluentThis;

#[derive(EsFluentThis)]
#[fluent_this(origin)]
#[fluent(namespace = "forms")]
pub enum Gender {
    Male,
    Female,
    Other,
}

// Generates key:
// (gender_this)

// usage: Gender::this_ftl()
```

- `#[fluent_this(variants)]`: Can be combined with `EsFluentVariants` derives to generate keys for variants.

```rs
#[derive(EsFluentVariants, EsFluentThis)]
#[fluent_this(origin, variants)]
#[fluent_variants(keys = ["label", "description"])]
#[fluent(namespace = "forms")]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

// Generates keys:
// (login_form_label_variants_this)
// (login_form_description_variants_this)

// usage: LoginFormDescriptionVariants::this_ftl()
```

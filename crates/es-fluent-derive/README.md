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
    Something(String, String, String), // exposed as $f1, $f2, $f3 in the ftl file
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

### `#[derive(EsFluentChoice)]`

Allows an enum to be used *inside* another message as a selector (e.g., for gender or status).

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

### `#[derive(EsFluentKv)]`

Generates key-value pair enums for struct fields. This is perfect for generating UI labels, placeholders, or descriptions for a form object.

```rs
use es_fluent::EsFluentKv;

#[derive(EsFluentKv)]
#[fluent_kv(keys = ["label", "description"])]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

// Generates enums -> keys:
// LoginFormLabelKvFtl::{Variants} -> (login_form_label_kv_ftl-{variant})
// LoginFormDescriptionKvFtl::{Variants} -> (login_form_description_kv_ftl-{variant})

// usage: LoginFormLabelKvFtl::Username.to_fluent_string()
```

### `#[derive(EsFluentThis)]`

Generates a helper implementation of the `ThisFtl` trait and registers the type's name as a key. This is similar to `EsFluentKv` (which registers fields), but for the parent type itself.

- `#[fluent_this(origin)]`: Generates an implementation where `this_ftl()` returns the base key for the type.

```rs
use es_fluent::EsFluentThis;

#[derive(EsFluentThis)]
#[fluent_this(origin)]
pub enum Gender {
    Male,
    Female,
    Other,
}

// Generates key:
// (gender_this)

// usage: Gender::this_ftl()
```

- `#[fluent_this(members)]`: Can be combined with `Kv` derives to generate keys for members.

```rs
#[derive(EsFluentKv, EsFluentThis)]
#[fluent_this(origin, members)]
#[fluent_kv(keys = ["label", "description"])]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

// Generates keys:
// (login_form_label_kv_ftl_this)
// (login_form_description_kv_ftl_this)

// usage: LoginFormDescriptionKvFtl::this_ftl()
```

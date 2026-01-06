# es-fluent-derive

Procedural macros for the `es-fluent` localization system.

This crate is the engine that transforms your Rust structs and enums into Fluent messages. It is designed to be used via the `es-fluent` crate, not directly.

All macros provided by this crate are fully independent and composable. You can use them individually or together on the same type depending on your needs.

## Features

### `#[derive(EsFluent)]`

Turns an enum or struct into a localizable message.

- **Enums**: Each variant becomes a message ID (e.g., `MyEnum::Variant` -> `my_enum-variant`).
- **Structs**: The struct itself becomes the message ID (e.g., `MyStruct` -> `my_struct`).
- **Fields**: Fields are automatically exposed as arguments to the Fluent message.

```rust
use es_fluent::EsFluent;

#[derive(EsFluent)]
pub enum LoginError {
    InvalidPassword,
    UserNotFound { username: String }, // exposed as $username
}
```

### `#[derive(EsFluentChoice)]`

Allows an enum to be used *inside* another message as a selector (e.g., for gender or status).

```rust
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
```

### `#[derive(EsFluentKv)]`

Generates key-value pair enums for struct fields. This is perfect for generating UI labels, placeholders, or descriptions for a form object.

```rust
use es_fluent::EsFluentKv;

#[derive(EsFluentKv)]
#[fluent_kv(keys = ["label", "description"])]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}
// Generates:
// LoginFormLabel::{Variants} (login_form_label-{variant})
// LoginFormDescription::{Variants} (login_form_description-{variant})
```

### `#[derive(EsFluentThis)]`

Generates a helper implementation of the `ThisFtl` trait and registers the type's name as a key. This is similar to `EsFluentKv` (which registers fields), but for the parent type itself.

- `#[fluent_this(origin)]`: Generates an implementation where `this_ftl()` returns the base key for the type.
- `#[fluent_this(members)]`: Can be combined with `Kv` derives to generate keys for members.

```rust
use es_fluent::EsFluentThis;

#[derive(EsFluentThis)]
#[fluent_this(origin)]
pub struct WelcomeMessage;

// usage: WelcomeMessage::this_ftl() -> "welcome_message"
```

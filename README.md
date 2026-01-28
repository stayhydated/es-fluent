[![Build Status](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml)
[![Docs](https://docs.rs/es-fluent/badge.svg)](https://docs.rs/es-fluent/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent.svg)](https://crates.io/crates/es-fluent)

# es-fluent

Derive macros and utilities for authoring strongly-typed messages with [Project Fluent](https://projectfluent.org/).

This framework gives you:

- Derives to turn enums/structs into Fluent message IDs and arguments.
- A [cli](crates/es-fluent-cli/README.md) to generate ftl files skeleton and other utilities.
- [Language Enum Generation](crates/es-fluent-lang/README.md)
- Integration via a [embedded singleton manager](crates/es-fluent-manager-embedded/README.md) or [es-fluent-manager-bevy](crates/es-fluent-manager-bevy/README.md) for [bevy](https://bevy.org/)

## Examples

- [bevy](https://github.com/stayhydated/es-fluent/tree/master/examples/bevy-example) ([online demo](<>))
- [gpui](https://github.com/stayhydated/es-fluent/tree/master/examples/gpui-example)

## Used in

- [koruma](https://github.com/stayhydated/koruma)
- [gpui-form](https://github.com/stayhydated/gpui-form)
- [gpui-table](https://github.com/stayhydated/gpui-table)
- [gpui-storybook](https://github.com/stayhydated/gpui-storybook)

## Installation

Add the crate with the `derive` feature to access the procedural macros:

```toml
[dependencies]
es-fluent = { version = "*", features = ["derive"] }
unic-langid = "*"

# If you want to register modules with the embedded singleton and localize at runtime:
es-fluent-manager-embedded = "*"

# For Bevy integration: replace `es-fluent-manager-embedded` with  `es-fluent-manager-bevy`
es-fluent-manager-bevy = "*"
```

## Project configuration

Create an `i18n.toml` next to your `Cargo.toml`:

```toml
# Default fallback language (required)
fallback_language = "en-US"

# Path to FTL assets relative to the config file (required)
assets_dir = "assets/locales"

# Features to enable if the crate’s es-fluent derives are gated behind a feature (optional)
fluent_feature = ["my-feature"]

# Optional allowlist of namespace values for FTL file splitting
namespaces = ["ui", "errors", "messages"]
```

## Namespaces (optional)

You can route specific types into separate `.ftl` files by adding a namespace:

```rs
use es_fluent::EsFluent;

#[derive(EsFluent)]
#[fluent(namespace = "ui")]
pub struct Button {
    pub label: String,
}

#[derive(EsFluent)]
#[fluent(namespace = file)]
pub struct Dialog {
    pub title: String,
}

#[derive(EsFluent)]
#[fluent(namespace(file(relative)))]
pub struct Modal {
    pub content: String,
}
```

Output layout:

- Default: `assets_dir/{locale}/{crate}.ftl`
- Namespaced: `assets_dir/{locale}/{crate}/{namespace}.ftl`

`namespace = file` uses the source file stem (e.g., `src/ui/button.rs` -> `button`).
`namespace(file(relative))` uses the file path relative to the crate root, strips `src/`, and removes the extension (e.g., `src/ui/button.rs` -> `ui/button`).
For `EsFluentThis` (and `EsFluentVariants` when using `#[fluent_this(...)]`), the same syntax is available via `#[fluent_this(namespace = "...")]`.

If `namespaces = [...]` is set in `i18n.toml`, the CLI will validate that every namespace used by your code is in that allowlist.

## Derives

### `#[derive(EsFluent)]`

Turns an enum or struct into a localizable message.

- **Enums**: Each variant becomes a message ID (e.g., `MyEnum::Variant` -> `my_enum-Variant`).
- **Structs**: The struct itself becomes the message ID (e.g., `MyStruct` -> `my_struct`).
- **Fields**: Fields are automatically exposed as arguments to the Fluent message.

```rs
use es_fluent::{EsFluent};

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

### `#[derive(EsFluentVariants)]`

Generates key-value pair enums for struct fields. This is perfect for generating UI labels, placeholders, or descriptions for a form object.

```rs
use es_fluent::EsFluentVariants;

#[derive(EsFluentVariants)]
#[fluent_variants(keys = ["label", "description"])]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

// Generates enums -> keys:
// LoginFormLabelVariants::{Variants} -> (login_form_label-{variant})
// LoginFormDescriptionVariants::{Variants} -> (login_form_description-{variant})

// usage: LoginFormLabelVariants::Username.to_fluent_string()
```

### `#[derive(EsFluentThis)]`

Generates a helper implementation of the `ThisFtl` trait and registers the type's name as a key. This is similar to `EsFluentVariants` (which registers fields), but for the parent type itself.

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

- `#[fluent_this(members)]`: Can be combined with `EsFluentVariants` derives to generate keys for members.

```rs
#[derive(EsFluentVariants, EsFluentThis)]
#[fluent_this(origin, members)]
#[fluent_variants(keys = ["label", "description"])]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

// Generates keys:
// (login_form_label_this)
// (login_form_description_this)

// usage: LoginFormDescriptionVariants::this_ftl()
```

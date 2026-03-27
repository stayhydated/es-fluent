# es-fluent

[![Build Status](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml)
[![codecov](https://codecov.io/github/stayhydated/es-fluent/graph/badge.svg?token=EFA5XVDNLK)](https://codecov.io/github/stayhydated/es-fluent)
[![llms.txt](https://img.shields.io/badge/docs-llms--full.txt-blue)](https://stayhydated.github.io/es-fluent/llms.txt)
[![Docs](https://docs.rs/es-fluent/badge.svg)](https://docs.rs/es-fluent/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent.svg)](https://crates.io/crates/es-fluent)

Derive macros and utilities for authoring strongly-typed messages with [Project Fluent](https://projectfluent.org/).

This framework gives you:

- Derives to turn enums/structs into Fluent message IDs and arguments.
- A [cli](../es-fluent-cli/README.md) to generate ftl files skeleton and other utilities.
- [Language Enum Generation](../es-fluent-lang/README.md)
- Integration via a [embedded singleton manager](../es-fluent-manager-embedded/README.md) or [es-fluent-manager-bevy](../es-fluent-manager-bevy/README.md) for [bevy](https://bevy.org/)

## Examples

- [bevy](https://github.com/stayhydated/es-fluent/tree/master/examples/bevy-example) ([online demo](https://stayhydated.github.io/es-fluent/bevy-example/))
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

## Incremental builds for locale assets

The manager macros discover locales at compile time. To ensure locale folder/file
renames (for example `fr` to `fr-FR`) trigger rebuilds,
enable the `build` feature of `es-fluent` in build dependencies and call the
tracking helper from `build.rs`.

```toml
[build-dependencies]
es-fluent = { version = "*", features = ["build"] }
```

```rs
// build.rs
fn main() {
    es_fluent::build::track_i18n_assets();
}
```

## Namespaces (optional)

You can route specific types into separate `.ftl` files by adding a namespace. All derive macros support the same namespace options:

### `EsFluent`

```rs
use es_fluent::EsFluent;

#[derive(EsFluent)]
#[fluent(namespace = "ui")]
pub struct Button<'a>(&'a str);

#[derive(EsFluent)]
#[fluent(namespace = file)]
pub struct Dialog {
    pub title: String,
}

#[derive(EsFluent)]
#[fluent(namespace(file(relative)))]
pub enum Gender {
    Male,
    Female,
    Other(String),
    Helicopter { type: String },
}
```

### `EsFluentThis`

```rs
use es_fluent::EsFluentThis;

#[derive(EsFluentThis)]
#[fluent_this(origin)]
#[fluent(namespace = "forms")]
pub enum Gender { Male, Female, Other }

#[derive(EsFluentThis)]
#[fluent_this(origin)]
#[fluent(namespace = file)]
pub enum Status { Active, Inactive }

#[derive(EsFluentThis)]
#[fluent_this(origin)]
#[fluent(namespace(file(relative)))]
pub struct UserProfile;

#[derive(EsFluentThis)]
#[fluent_this(origin)]
#[fluent(namespace = folder)]
pub enum FolderStatus { Active, Inactive }

#[derive(EsFluentThis)]
#[fluent_this(origin)]
#[fluent(namespace(folder(relative)))]
pub struct FolderUserProfile;
```

### `EsFluentVariants`

```rs
use es_fluent::EsFluentVariants;

#[derive(EsFluentVariants)]
#[fluent_variants(keys = ["label", "description"])]
#[fluent(namespace = "forms")]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

#[derive(EsFluentVariants)]
#[fluent(namespace = file)]
pub enum StatusVariants { Active, Inactive }
```

### Output Layout

- Default: `assets_dir/{locale}/{crate}.ftl`
- Namespaced: `assets_dir/{locale}/{crate}/{namespace}.ftl`

When namespaces are used, namespace files are treated as the canonical split
for that locale. `{crate}.ftl` remains optional for backwards compatibility.

### Namespace Values

- `namespace = "name"` - explicit namespace string
- `namespace = file` - uses the source file stem (e.g., `src/ui/button.rs` -> `button`)
- `namespace(file(relative))` - uses the file path relative to the crate root, strips `src/`, and removes the extension (e.g., `src/ui/button.rs` -> `ui/button`)
- `namespace = folder` - uses the source file parent folder (e.g., `src/ui/button.rs` -> `ui`)
- `namespace(folder(relative))` - uses the parent folder path relative to the crate root, strips `src/` when nested, and keeps `src` for root module files (e.g., `src/ui/button.rs` -> `ui`)

If `namespaces = [...]` is set in `i18n.toml`, both the compiler (at compile-time) and the CLI will validate that string-based namespaces used by your code are in that allowlist.

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

use es_fluent::ToFluentString;
let _ = LoginError::InvalidPassword.to_fluent_string();
let _ = LoginError::UserNotFound { username: "john".to_string() }.to_fluent_string();
let _ = LoginError::Something("a".to_string(), "b".to_string(), "c".to_string()).to_fluent_string();

#[derive(EsFluent)]
pub struct UserProfile<'a> {
    pub name: &'a str, // exposed as $name in the ftl file
    pub gender: &'a str, // exposed as $gender in the ftl file
}

use es_fluent::ToFluentString;
let profile = UserProfile { name: "John", gender: "male" };
let _ = profile.to_fluent_string();
```

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

use es_fluent::ToFluentString;
let profile = UserProfile { name: "John", gender: &Gender::Male };
let _ = profile.to_fluent_string();
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
// LoginFormLabelVariants::{Variants} -> (login_form_label_variants-{variant})
// LoginFormDescriptionVariants::{Variants} -> (login_form_description_variants-{variant})

use es_fluent::ToFluentString;
let _ = LoginFormLabelVariants::Username.to_fluent_string();
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

use es_fluent::ThisFtl;
let _ = Gender::this_ftl();
```

- `#[fluent_this(variants)]`: Can be combined with `EsFluentVariants` derives to generate keys for variants.

```rs
#[derive(EsFluentVariants, EsFluentThis)]
#[fluent_this(origin, variants)]
#[fluent_variants(keys = ["label", "description"])]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

// Generates keys:
// (login_form_label_variants_this)
// (login_form_description_variants_this)

use es_fluent::ThisFtl;
let _ = LoginFormDescriptionVariants::this_ftl();
```

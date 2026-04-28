# es-fluent

[![Build Status](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/github/stayhydated/es-fluent/graph/badge.svg?token=EFA5XVDNLK)](https://codecov.io/github/stayhydated/es-fluent)
[![mdBook](https://img.shields.io/badge/docs-mdBook-black)](https://stayhydated.github.io/es-fluent/book/)
[![llms.txt](https://img.shields.io/badge/docs-llms.txt-blue)](https://stayhydated.github.io/es-fluent/llms.txt)
[![llms-full.txt](https://img.shields.io/badge/docs-llms--full.txt-blue)](https://stayhydated.github.io/es-fluent/llms-full.txt)
[![Docs](https://docs.rs/es-fluent/badge.svg)](https://docs.rs/es-fluent/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent.svg)](https://crates.io/crates/es-fluent)

Derive macros and utilities for authoring strongly-typed messages with [Project Fluent](https://projectfluent.org/).

This framework gives you:

- Derives to turn enums/structs into Fluent message IDs and arguments.
- A [cli](crates/es-fluent-cli/README.md) to generate ftl files skeleton and other utilities.
- [Language Enum Generation](crates/es-fluent-lang/README.md)
- Integration via the [embedded manager](crates/es-fluent-manager-embedded/README.md), the [Dioxus manager](crates/es-fluent-manager-dioxus/README.md), or [es-fluent-manager-bevy](crates/es-fluent-manager-bevy/README.md) for [Bevy](https://bevy.org/)

## Examples

- [bevy](https://github.com/stayhydated/es-fluent/tree/master/examples/bevy-example) ([online demo](https://stayhydated.github.io/es-fluent/bevy-example/))
- [dioxus client](https://github.com/stayhydated/es-fluent/tree/master/examples/dioxus-client-example)
- [dioxus SSR](https://github.com/stayhydated/es-fluent/tree/master/examples/dioxus-ssr-example)
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
es-fluent = { version = "0.16", features = ["derive"] }
unic-langid = "0.9"

# If you want to register modules with the embedded context and localize at runtime:
es-fluent-manager-embedded = "0.16"

# For Dioxus apps, enable only the runtime surface you use.
es-fluent-manager-dioxus = { version = "0.7", features = ["client"] }
# es-fluent-manager-dioxus = { version = "0.7", features = ["ssr"] }

# For Bevy integration: replace `es-fluent-manager-embedded` with  `es-fluent-manager-bevy`
es-fluent-manager-bevy = "0.18.13"
```

`es_fluent_manager_embedded::EmbeddedI18n::try_new_with_language(...)` is the simplest embedded startup path:

```rust
let i18n = es_fluent_manager_embedded::EmbeddedI18n::try_new_with_language(langid!("en-US"))?;
```

For custom runtime integrations, use
`es-fluent-manager-core::FluentManager::try_new_with_discovered_modules()`.
For Dioxus, `es-fluent-manager-dioxus` provides a provider component, hook-based client helpers, typed context-bound localization, and signal-backed locale state behind the `client` feature. Its `ssr` feature provides a request-scoped runtime. Dioxus code should use `DioxusI18n::localize_message(...)`, `ManagedI18n::localize_message(...)`, or explicit `localize*` helpers; Dioxus does not use the explicit localizer context.
The Bevy plugin uses the same strict discovery model and exposes both
`RequestedLanguageId` and `ActiveLanguageId` so systems can distinguish the
requested locale from the currently published one. Failed locale switches keep
the last ready locale active. Systems that need direct localization can request
`BevyI18n` as a `SystemParam` and call `localize_message(...)` on it.

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

If your crate uses the embedded, Dioxus, or Bevy manager macros, they discover
locales at compile time by scanning `assets_dir`. To ensure locale folder/file
renames (for example `fr` to `fr-FR`) trigger rebuilds, enable the `build`
feature of `es-fluent` in build dependencies and call the tracking helper from
`build.rs`. Crates that only use the derive macros do not need this setup.

```toml
[build-dependencies]
es-fluent = { version = "0.16", features = ["build"] }
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
pub struct Button<'a>(pub &'a str);

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
    Helicopter { type_: String },
}
```

### `EsFluentThis`

```rs
use es_fluent::EsFluentThis;

#[derive(EsFluentThis)]
#[fluent_this(origin)]
#[fluent(namespace = "forms")]
pub enum GenderThis { Male, Female, Other }

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
for that locale, and `{crate}.ftl` can still participate as an optional base
resource for non-namespaced messages.

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
    Something(String, String, String), // exposed as $f0, $f1, $f2 in the ftl file
    SomethingArgNamed(
        #[fluent(arg_name = "input")] String,
        #[fluent(arg_name = "expected")] String,
        #[fluent(arg_name = "details")] String,
    ), // exposed as $input, $expected, $details
}

let _ = i18n.localize_message(&LoginError::InvalidPassword);
let _ = i18n.localize_message(&LoginError::UserNotFound { username: "john".to_string() });
let _ = i18n.localize_message(&LoginError::Something("a".to_string(), "b".to_string(), "c".to_string()));
let _ = i18n.localize_message(&LoginError::SomethingArgNamed("a".to_string(), "b".to_string(), "c".to_string()));

#[derive(EsFluent)]
pub struct WelcomeMessage<'a> {
    pub name: &'a str, // exposed as $name in the ftl file
    pub count: i32,    // exposed as $count in the ftl file
}

let welcome = WelcomeMessage { name: "John", count: 5 };
let _ = i18n.localize_message(&welcome);
```

Argument naming attributes:

- `arg_name = "..."` on a field renames that exposed Fluent argument (works on struct fields, enum named fields, and enum tuple fields).

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

### `#[derive(EsFluentChoice)]`

Allows an enum to be used _inside_ another message as a selector (e.g., for gender or status).

```rs
use es_fluent::{EsFluent, EsFluentChoice};

#[derive(EsFluent, EsFluentChoice)]
#[fluent_choice(serialize_all = "snake_case")]
pub enum GenderChoice {
    Male,
    Female,
    Other,
}

#[derive(EsFluent)]
pub struct Greeting<'a> {
    pub name: &'a str,
    #[fluent(choice)] // Matches $gender -> [male]...
    pub gender: &'a GenderChoice,
}

use es_fluent::FluentMessage;
let greeting = Greeting { name: "John", gender: &GenderChoice::Male };
let _ = i18n.localize_message(&greeting);
```

### `#[derive(EsFluentVariants)]`

Generates key-value pair enums for struct fields or enum variants. This is
useful for generating UI labels, placeholders, or descriptions for a form
object, and it can also expose enum variants as localizable keys.

```rs
use es_fluent::EsFluentVariants;

#[derive(EsFluentVariants)]
#[fluent_variants(keys = ["label", "description"])]
pub struct LoginFormVariants {
    pub username: String,
    pub password: String,
}

// Generates enums -> keys:
// LoginFormVariantsLabelVariants::{Variants} -> (login_form_variants_label_variants-{variant})
// LoginFormVariantsDescriptionVariants::{Variants} -> (login_form_variants_description_variants-{variant})

use es_fluent::FluentMessage;
let _ = i18n.localize_message(&LoginFormVariantsLabelVariants::Username);

#[derive(EsFluentVariants)]
pub enum SettingsTab {
    General,
    Notifications,
    Privacy,
}

// Generates enum -> keys:
// SettingsTabVariants::{General, Notifications, Privacy}
//     -> (settings_tab_variants-{variant})

let _ = i18n.localize_message(&SettingsTabVariants::Notifications);
```

### `#[derive(EsFluentThis)]`

Generates a helper implementation of the `ThisFtl` trait and registers the
type's name as a key. This is similar to `EsFluentVariants` (which registers
field- or variant-derived keys), but for the parent type itself.

- `#[fluent_this(origin)]`: Generates an implementation where `this_ftl(localizer)` returns the base key for the type.

```rs
use es_fluent::EsFluentThis;

#[derive(EsFluentThis)]
#[fluent_this(origin)]
pub enum GenderThisOnly {
    Male,
    Female,
    Other,
}

// Generates key:
// (gender_this_only_this)

use es_fluent::ThisFtl;
let _ = GenderThisOnly::this_ftl(&i18n);
```

- `#[fluent_this(variants)]`: Can be combined with `EsFluentVariants` derives to generate keys for variants.

```rs
#[derive(EsFluentVariants, EsFluentThis)]
#[fluent_this(origin, variants)]
#[fluent_variants(keys = ["label", "description"])]
pub struct LoginFormCombined {
    pub username: String,
    pub password: String,
}

// Generates keys:
// (login_form_combined_label_variants_this)
// (login_form_combined_description_variants_this)

use es_fluent::ThisFtl;
let _ = LoginFormCombinedDescriptionVariants::this_ftl(&i18n);
```

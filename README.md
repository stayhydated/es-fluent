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

- [bevy](examples/bevy-example) ([online demo](https://stayhydated.github.io/es-fluent/bevy-example/))
- [dioxus client](examples/dioxus-client-example)
- [dioxus SSR](examples/dioxus-ssr-example)
- [gpui](examples/gpui-example)

## Used in

- [koruma](https://github.com/stayhydated/koruma)
- [gpui-form](https://github.com/stayhydated/gpui-form)
- [gpui-table](https://github.com/stayhydated/gpui-table)
- [gpui-storybook](https://github.com/stayhydated/gpui-storybook)

## Version compatibility

| Surface                                           | Version line | Runtime        |
| :------------------------------------------------ | :----------- | :------------- |
| `es-fluent`, CLI, embedded manager, language enum | `0.16.x`     | General Rust   |
| `es-fluent-manager-dioxus`                        | `0.7.x`      | Dioxus `0.7.x` |
| `es-fluent-manager-bevy`                          | `0.18.x`     | Bevy `0.18.x`  |

## Installation

Add `es-fluent`; derive macros are enabled by default:

```toml
[dependencies]
es-fluent = "0.16"
unic-langid = "0.9"

# If you want to register modules with the embedded context and localize at runtime:
# Default zero-setup runtime manager for this quick start.
es-fluent-manager-embedded = "0.16"

# For Dioxus apps, enable only the runtime surface you use.
# es-fluent-manager-dioxus = { version = "0.7", features = ["client"] }
# es-fluent-manager-dioxus = { version = "0.7", features = ["ssr"] }

# For Bevy integration: replace `es-fluent-manager-embedded` with  `es-fluent-manager-bevy`
# es-fluent-manager-bevy = "0.18.13"
```

`es_fluent_manager_embedded::EmbeddedI18n::try_new_with_language(...)` is the simplest embedded startup path:

```ignore
let i18n = es_fluent_manager_embedded::EmbeddedI18n::try_new_with_language(langid!("en"))?;
```

Use `try_new_with_language_strict(...)` instead when every discovered module
must support the startup locale.

For ordinary applications, keep an explicit concrete manager handle in
application state and use typed lookup on that handle:

```toml
[dependencies]
es-fluent = "0.16"
es-fluent-manager-embedded = "0.16"
```

```no_run
use es_fluent::EsFluent;
use es_fluent_manager_embedded::EmbeddedI18n;
use unic_langid::langid;

#[derive(EsFluent)]
struct Greeting<'a> {
    name: &'a str,
}

fn main() -> Result<(), String> {
    let i18n = EmbeddedI18n::try_new_with_language(langid!("en"))
        .map_err(|error| error.to_string())?;
    let greeting = i18n.localize_message(&Greeting { name: "Ada" });

    Ok(())
}
```

Prefer `localize_message(...)` on the concrete manager handle. Raw string-ID
lookup remains available in `es-fluent-manager-core` for integration code.
Application-facing APIs are intentionally enum-first.

For custom runtime integrations, create a `FluentManager`, select the initial
language, and either wrap it in your integration type or import the public
extension trait for generic typed lookup:

```toml
[dependencies]
es-fluent = "0.16"
es-fluent-manager-core = "0.16"
```

```no_run
use es_fluent::{EsFluent, FluentLocalizerExt as _};
use es_fluent_manager_core::FluentManager;
use unic_langid::langid;

#[derive(EsFluent)]
struct Greeting<'a> {
    name: &'a str,
}

fn main() -> Result<(), String> {
    let manager = FluentManager::try_new_with_discovered_modules()
        .map_err(|errors| format!("{errors:?}"))?;
    manager
        .select_language(&langid!("en"))
        .map_err(|error| error.to_string())?;

    let greeting = manager.localize_message(&Greeting { name: "Ada" });
    let _ = greeting;

    Ok(())
}
```

For Dioxus, `es-fluent-manager-dioxus` provides a provider component,
hook-based client helpers, typed context-bound localization, and signal-backed
locale state behind the `client` feature. Its `ssr` feature provides a
request-scoped runtime. Dioxus code should use
`DioxusI18n::localize_message(...)`, `ManagedI18n::localize_message(...)`, or
typed label helpers through the component or SSR request context.
Dioxus does not use the generic embedded localizer handle or install a
process-wide localizer.
For Bevy, systems that need direct localization can request `BevyI18n` as a
`SystemParam` and call `localize_message(...)` on it. The plugin also exposes
`RequestedLanguageId` and `ActiveLanguageId` for systems that need to
distinguish user intent from the currently published locale. Detailed Bevy
asset readiness and fallback-manager behavior is documented in
`crates/es-fluent-manager-bevy/docs/ARCHITECTURE.md`.

## Project configuration

For a new crate, start with the CLI scaffold:

```sh
cargo es-fluent init
```

This creates `i18n.toml`, `assets/locales/en/`, `src/i18n.rs`, and a
`pub mod i18n;` declaration in `src/lib.rs`. Use `--manager dioxus` or
`--manager bevy` for framework-specific scaffolding, and `--build-rs` to add
locale asset rebuild tracking. Use `--locales fr-FR,zh-CN` to create more
locale directories, `--namespaces ui,errors` to write a namespace allowlist,
and `--update-cargo-toml` to add the matching dependencies.

Or create an `i18n.toml` next to your `Cargo.toml` manually:

```toml
# Default fallback language (required)
fallback_language = "en"

# Path to FTL assets relative to the config file (required)
assets_dir = "assets/locales"

# Features to enable if the crate’s es-fluent derives are gated behind a feature (optional)
fluent_feature = ["my-feature"]

# Optional allowlist of namespace values for FTL file splitting
namespaces = ["ui", "errors", "messages"]
```

Locale directory names use canonical BCP-47 tags. The executable README example
ships `en`, `fr-FR`, and `zh-CN`, with `en` as the fallback locale.

Add a new language later by seeding it from the fallback locale:

```sh
cargo es-fluent add-locale fr-FR
```

For pre-commit or CI checks, `cargo es-fluent status --all` reports pending
generation, formatting, sync, orphan cleanup, and validation work without
writing files.

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

### `EsFluentLabel`

```rs
use es_fluent::EsFluentLabel;

#[derive(EsFluentLabel)]
#[fluent_label(origin)]
#[fluent(namespace = "forms")]
pub enum GenderLabel { Male, Female, Other }

#[derive(EsFluentLabel)]
#[fluent_label(origin)]
#[fluent(namespace = file)]
pub enum Status { Active, Inactive }

#[derive(EsFluentLabel)]
#[fluent_label(origin)]
#[fluent(namespace(file(relative)))]
pub struct UserProfile;

#[derive(EsFluentLabel)]
#[fluent_label(origin)]
#[fluent(namespace = folder)]
pub enum FolderStatus { Active, Inactive }

#[derive(EsFluentLabel)]
#[fluent_label(origin)]
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

Literal string namespaces are validated at compile time as safe relative namespace paths. If `namespaces = [...]` is set in `i18n.toml`, both the compiler and the CLI validate that string-based namespaces used by your code are in that allowlist.

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

Common derive attributes:

- `arg_name = "..."` on a field renames that exposed Fluent argument (works on struct fields, enum named fields, and enum tuple fields).
- `#[fluent(skip)]` on a field excludes that field from generated arguments.
- `#[fluent(value = "...")]` or `#[fluent(value(...))]` transforms a field before inserting it as a Fluent argument.
- `#[fluent(key = "...")]` on an enum variant overrides that variant's key suffix.
- `#[fluent(resource = "...")]` on an enum overrides the base key, `domain = "..."` routes lookup to a specific manager domain, and `skip_inventory` suppresses CLI inventory registration.
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

### `#[derive(EsFluentLabel)]`

Generates a helper implementation of the `FluentLabel` trait and registers the
type's name as a key. This is similar to `EsFluentVariants` (which registers
field- or variant-derived keys), but for the parent type itself.

- `#[fluent_label(origin)]`: Generates an implementation where `localize_label(localizer)` returns the base key for the type.

```rs
use es_fluent::EsFluentLabel;

#[derive(EsFluentLabel)]
#[fluent_label(origin)]
pub enum GenderLabelOnly {
    Male,
    Female,
    Other,
}

// Generates key:
// (gender_label_only_label)

use es_fluent::FluentLabel;
let _ = GenderLabelOnly::localize_label(&i18n);
```

- `#[fluent_label(variants)]`: Can be combined with `EsFluentVariants` derives to generate keys for variants.

```rs
#[derive(EsFluentVariants, EsFluentLabel)]
#[fluent_label(origin, variants)]
#[fluent_variants(keys = ["label", "description"])]
pub struct LoginFormCombined {
    pub username: String,
    pub password: String,
}

// Generates keys:
// (login_form_combined_label_variants_label)
// (login_form_combined_description_variants_label)

use es_fluent::FluentLabel;
let _ = LoginFormCombinedDescriptionVariants::localize_label(&i18n);
```

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

## Used in

- [koruma](https://github.com/stayhydated/koruma)
- [gpui-form](https://github.com/stayhydated/gpui-form)
- [gpui-table](https://github.com/stayhydated/gpui-table)
- [gpui-storybook](https://github.com/stayhydated/gpui-storybook)

## Version Matrix

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
# es-fluent-manager-dioxus = { version = "0.7", features = ["client", "ssr"] }

# For Bevy integration, use `es-fluent-manager-bevy`.
# es-fluent-manager-bevy = "0.18.13"
```

`es_fluent_manager_embedded::EmbeddedI18n::try_new_with_language(...)` is the simplest embedded startup path:

```rust,no_run
use unic_langid::langid;

fn main() -> Result<(), es_fluent_manager_embedded::EmbeddedInitError> {
    let i18n = es_fluent_manager_embedded::EmbeddedI18n::try_new_with_language(langid!("en"))?;
    let _ = i18n;

    Ok(())
}
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

Register the embedded module from a library-reachable module, usually
`src/i18n.rs` declared by `pub mod i18n;` in `src/lib.rs`:

```rs
// src/i18n.rs
pub use es_fluent_manager_embedded::{
    EmbeddedI18n as I18n, EmbeddedInitError, LocalizationError,
};

es_fluent_manager_embedded::define_i18n_module!();
```

```rs
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

Prefer `localize_message(...)` on the concrete manager handle. The public
manager and `FluentLocalizer` lookup paths receive `StaticFluentDomain`,
`StaticFluentEntryId`, and typed Fluent argument maps, so derived message
rendering keeps validated IDs typed until the final Fluent bundle lookup.
Application-facing APIs are intentionally enum-first. Custom integrations that
need to distinguish missing lookups from message ID fallback can use
`FluentLocalizerExt::try_localize_message(...)`.

For custom runtime integrations, create a `FluentManager`, select the initial
language, and either wrap it in your integration type or import the public
extension trait for generic typed lookup:

```toml
[dependencies]
es-fluent = "0.16"
es-fluent-manager-core = "0.16"
```

```rs
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
request-scoped runtime. Dioxus translations are loaded through generated
Dioxus asset modules registered with inventory. `DioxusAssetI18nProvider` and
`SsrI18nRuntime::discovered()` use those registrations by default. Dioxus code should use
`DioxusAssetI18nHandle::localize_message(...)` or typed label helpers through
the component or SSR request context.
Runtime follower modules such as `es-fluent-lang` language labels are
discovered automatically and follow the selected asset-backed locale.
During `dx serve` debug WASM runs, changed generated FTL assets refresh the
provider context through Dioxus asset hot reload while preserving the requested
locale when possible.
For Bevy, systems that need direct localization can request `BevyI18n` as a
`SystemParam` and call `localize_message(...)` on it. The plugin also exposes
`RequestedLanguageId` and `ActiveLanguageId` for systems that need to
distinguish user intent from the currently published locale. Generated Bevy
module registrations expose FTL from the crate that owns each localization
domain as Bevy embedded assets, so apps should not copy dependency-owned domain
files into their own asset tree.

## Project configuration

Create `i18n.toml` next to your crate's `Cargo.toml`, create the fallback
locale directory, and expose an i18n module from your library target when you
use manager macros:

```rust,ignore
// src/i18n.rs
pub use es_fluent_manager_embedded::{
    EmbeddedI18n as I18n, EmbeddedInitError, LocalizationError,
};

es_fluent_manager_embedded::define_i18n_module!();
```

```rust,ignore
// src/lib.rs
pub mod i18n;
```

Use the Dioxus or Bevy manager crate in that module for framework-specific
integrations. If manager macros scan locale assets at compile time, add
`es-fluent-build` under `[build-dependencies]` and call
`es_fluent_build::track_i18n_assets();` from `build.rs` so Cargo rebuilds when
locale files are added, removed, or renamed.

Create an `i18n.toml` next to your `Cargo.toml`:

```toml
# Default fallback language (required)
fallback_language = "en"

# Path to FTL assets relative to the crate root; must stay inside the crate
# and must not use existing symlinked path components or locale targets (required)
assets_dir = "assets/locales"

# Features to enable if the crate’s es-fluent derives are gated behind a feature (optional)
fluent_feature = ["my-feature"]

# Optional allowlist of namespace values for FTL file splitting
namespaces = ["ui", "errors", "messages"]

# Optional: disable warnings when non-fallback messages copy fallback text
check_fallback_copies = false
```

Locale directory names use canonical BCP-47 tags. Deprecated aliases such as
`iw` and `src` are rejected; use canonical replacements such as `he` and `sc`.
The executable README example ships `en`, `fr-FR`, and `zh-CN`, with `en` as
the fallback locale.

Add a new language later by seeding it from the fallback locale:

```sh
cargo es-fluent add-locale fr-FR
cargo es-fluent add-locale fr-FR,zh-CN
cargo es-fluent add-locale "fr-FR, zh-CN"
```

The fallback locale directory, such as `assets/locales/en`, must exist as a
directory before syncing or adding target locales; it may be empty before
generated keys exist. Locale arguments can be passed separately or as
comma-separated lists; empty comma entries, such as `add-locale fr-FR,`, are
rejected. Existing target locale paths must be real directories, not symlinks.
If the requested locale already exists and no fallback keys are missing,
rerunning `add-locale` is a successful no-op. Duplicate requested locale
targets are ignored. If `--package` matches no configured crate, locale
creation exits non-zero instead of reporting success without writing files.
Locale creation preflights every selected crate for setup, fallback parse, and
requested-locale path errors before writing any selected crate. Unexpected
write-time I/O failures after preflight succeeds are still not rolled back.

For pre-commit or CI checks, `cargo es-fluent status --all` reports pending
generation, formatting, sync, orphan cleanup, and validation work without
editing project source or locale files. It may prepare `.es-fluent` runner
metadata and Cargo build output while checking valid crates. If `.es-fluent`
already exists, it and existing entries below it must be real paths, not
symlinks. Empty selections and setup errors are reported before that runner
preparation.

## Incremental builds for locale assets

If your crate uses the embedded, Dioxus, or Bevy manager macros, they discover
locales at compile time by scanning `assets_dir`. To ensure locale folder/file
renames (for example `fr` to `fr-FR`) trigger rebuilds, add `es-fluent-build`
to build dependencies and call the tracking helper from `build.rs`. Crates that
only use the derive macros do not need this setup.

```toml
[build-dependencies]
es-fluent-build = "0.16"
```

```rust,no_run
// build.rs
fn main() {
    es_fluent_build::track_i18n_assets();
}
```

## Namespaces (optional)

You can route specific types into separate `.ftl` files by adding a namespace. All derive macros support the same namespace options:

Use exactly one namespace source for each generated output. When multiple
derives are combined on one type, put the namespace on `#[fluent(...)]` to
share it, or put it on the specific `#[fluent_label(...)]` /
`#[fluent_variants(...)]` output, but do not combine those namespace sources.

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
#[fluent(namespace = file_relative)]
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
#[fluent(namespace = "forms")]
pub enum GenderLabel { Male, Female, Other }

#[derive(EsFluentLabel)]
#[fluent(namespace = file)]
pub enum Status { Active, Inactive }

#[derive(EsFluentLabel)]
#[fluent(namespace = file_relative)]
pub struct UserProfile;

#[derive(EsFluentLabel)]
#[fluent(namespace = folder)]
pub enum FolderStatus { Active, Inactive }

#[derive(EsFluentLabel)]
#[fluent(namespace = folder_relative)]
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

- `namespace = "name"` - explicit namespace string. Literal namespaces must be safe locale-relative paths: no empty segments, `.`/`..`, backslashes, absolute paths, surrounding whitespace, or `.ftl` suffix.
- `namespace = file` - uses the source file stem (e.g., `src/ui/button.rs` -> `button`)
- `namespace = file_relative` - uses the file path relative to the crate root, strips `src/`, and removes the extension (e.g., `src/ui/button.rs` -> `ui/button`)
- `namespace = folder` - uses the source file parent folder (e.g., `src/ui/button.rs` -> `ui`)
- `namespace = folder_relative` - uses the parent folder path relative to the crate root, strips `src/` when nested, and keeps `src` for root module files (e.g., `src/ui/button.rs` -> `ui`)

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
        #[fluent(arg = "input")] String,
        #[fluent(arg = "expected")] String,
        #[fluent(arg = "details")] String,
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

Rendering through a callback:

```rs
use es_fluent::{EsFluent, FluentArgs, FluentMessage};

#[derive(EsFluent)]
pub struct UsernameRequired {
    #[fluent(value = |value: &String| value.trim().to_string())]
    value: String,
}

let message = UsernameRequired {
    value: "  mark  ".to_string(),
};

let mut lookup = |domain, id, args: Option<&FluentArgs<'_>>| {
    // Applications usually forward these values to an es-fluent manager.
    // Koruma and GPUI integrations use this same callback shape for validator refs.
    format!("{domain}:{id}:{:?}", args.map(FluentArgs::as_raw))
};

let rendered = message.to_fluent_string_with(&mut lookup);
```

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

### Choices

Unit-only enums that derive `EsFluent` can be used _inside_ another message as selectors (e.g., for gender or status). Variants serialize as kebab-case by default, so `GenderChoice::Male` becomes `male` and a compound variant like `VeryFriendly` becomes `very-friendly`.
Derived choice values are emitted as validated `StaticFluentVariantKey` values.
Use `#[fluent_choice(rename_all = "...")]` on the same enum to change selector casing. Styles that generate invalid selector values, such as values containing spaces, are rejected at compile time.
Use standalone `#[derive(EsFluentChoice)]` only for selector enums that should not also be registered as messages.

```rs
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
    #[fluent(selector)] // Matches $gender -> [male]...
    pub gender: Option<&'a GenderChoice>,
}

use es_fluent::FluentMessage;
let greeting = Greeting { name: "John", gender: Some(&GenderChoice::Male) };
let _ = i18n.localize_message(&greeting);
```

### `#[derive(EsFluentVariants)]`

Generates key-value pair enums for struct fields or enum variants. This is
useful for generating UI labels, placeholders, or descriptions for a form
object, and it can also expose enum variants as localizable keys.

```rs
use es_fluent::{EsFluent, EsFluentVariants};

#[derive(EsFluentVariants)]
#[fluent_variants(keys = ["label", "description"])]
pub struct LoginFormVariants {
    pub username: String,
    pub password: String,
}

// Generates enums -> keys:
// LoginFormVariantsLabelVariants::{Variants} -> (login_form_variants_label_variants-{variant})
// LoginFormVariantsDescriptionVariants::{Variants} -> (login_form_variants_description_variants-{variant})

#[derive(EsFluent)]
pub struct ActiveFormField {
    #[fluent(selector)]
    pub field: LoginFormVariantsLabelVariants,
}

use es_fluent::FluentMessage;
let _ = i18n.localize_message(&LoginFormVariantsLabelVariants::Username);
let _ = i18n.localize_message(&ActiveFormField {
    field: LoginFormVariantsLabelVariants::Username,
});

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

Generated variant enums derive `Clone`, `Copy`, `Debug`, `Eq`, `Hash`, and
`PartialEq` automatically and implement `EsFluentChoice`, so they can be used
directly in `#[fluent(selector)]` fields. Add `derive(...)` inside
`#[fluent_variants(...)]` only for additional traits; `EsFluentChoice` is
already inferred.

### `#[derive(EsFluentLabel)]`

Generates a helper implementation of the `FluentLabel` trait and registers the
type's name as a key. This is similar to `EsFluentVariants` (which registers
field- or variant-derived keys), but for the parent type itself.

- `#[derive(EsFluentLabel)]` generates typed label metadata plus `localize_label(localizer)` and `try_localize_label(localizer)`.

```rs
use es_fluent::EsFluentLabel;

#[derive(EsFluentLabel)]
pub enum GenderLabelOnly {
    Male,
    Female,
    Other,
}

// Generates key:
// (gender_label_only_label)

use es_fluent::FluentLabel;
let _ = GenderLabelOnly::localize_label(&i18n);
let _ = GenderLabelOnly::try_localize_label(&i18n);
let _ = GenderLabelOnly::fluent_label_id();
let _ = GenderLabelOnly::fallback_label();
let _ = es_fluent::fallback_label::<GenderLabelOnly>();
```

Use `fallback_label::<T>()` only when generated metadata, tests, or integration
scaffolding cannot access a runtime localization context. It keeps the input
typed through `FluentLabel`, then renders the generated label id as readable
fallback text. UI code that has an `EmbeddedI18n`, `FluentManager`, or framework
manager should continue to call `T::localize_label(&i18n)` so labels follow the
active locale.

`#[derive(EsFluentVariants)]` also gives each generated variant enum a label
key inferred from the generated enum name.

```rs
#[derive(EsFluentVariants, EsFluentLabel)]
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

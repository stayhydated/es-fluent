# es-fluent

[![Build Status](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml)
[![Docs](https://docs.rs/es-fluent/badge.svg)](https://docs.rs/es-fluent/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent.svg)](https://crates.io/crates/es-fluent)

Derive macros and utilities for authoring strongly-typed messages with [Project Fluent](https://projectfluent.org/).

This crate gives you:

- Derives to turn enums/structs into Fluent message IDs and arguments.
- Integration via a embedded singleton manager (`es-fluent-manager-embedded`) or for Bevy (`es-fluent-manager-bevy`).

## Examples

- [bevy](https://github.com/stayhydated/es-fluent/tree/master/examples/bevy-example)
- [gpui](https://github.com/stayhydated/es-fluent/tree/master/examples/gpui-example)
- [cosmic](https://github.com/stayhydated/es-fluent/tree/master/examples/cosmic-example)
- [iced](https://github.com/stayhydated/es-fluent/tree/master/examples/iced-example)

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

To bootstrap `.ftl` files from your Rust types, install the CLI tool:

```sh
cargo install es-fluent-cli
```

## Project configuration

Create an `i18n.toml` next to your `Cargo.toml`:

```toml
# i18n.toml
assets_dir = "i18n"         # where your localized files live
fallback_language = "en"
```

## CLI

[![Docs](https://docs.rs/es-fluent/badge.svg)](https://docs.rs/es-fluent-cli/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent.svg)](https://crates.io/crates/es-fluent-cli)

### Generate

Run the generator to discover your crate name, parse Rust sources under `src/`, and generate or update a base FTL file at `{assets_dir}/{fallback_language}/{crate_name}.ftl`.

```sh
es-fluent generate
```

### Watch

Automatically compile and run the generator whenever you modify your source code.

```sh
es-fluent watch
```

### Clean

Remove orphan keys and groups that are no longer present in your source code.

```sh
es-fluent clean
```

For example, with `assets_dir = "../i18n"` and `fallback_language = "en"`, the file would be `../i18n/en/{crate_name}.ftl`.

## Core derives

### `#[derive(EsFluent)]`

Annotate an enum or a struct to generate message IDs and implement `es_fluent::FluentDisplay`.

```rust
use es_fluent::EsFluent;

#[derive(EsFluent)]
pub enum TeaBlend {
    EarlGrey,
    EnglishBreakfast,
    Darjeeling,
}

#[derive(EsFluent)]
pub enum Drink {
    Tea { blend: TeaBlend },
    Water,
}

#[derive(EsFluent)]
pub struct Hello<'a>(pub &'a str);
```

### Choices with `EsFluentChoice`

When a message needs to match on an enum (a Fluent select expression), implement `EsFluentChoice`. You can then mark a field with `#[fluent(choice)]` to pass its choice value instead of formatting it as a nested message.

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
pub enum Shared<'a> {
    Photos {
        user_name: &'a str,
        photo_count: &'a u32,
        #[fluent(choice)]
        user_gender: &'a Gender,
    },
}
```

A prototyping build will write skeleton FTL like:

```ftl
## Gender
gender-Male = Male
gender-Female = Female
gender-Other = Other

## Hello
hello-User = User { $user_name }

## Shared
shared-Photos = Photos { $user_name } { $photo_count } { $user_gender }
```

You can then edit it into a real copy, e.g.:

```ftl
## Gender
gender-Female = Female
gender-Helicopter = Helicopter
gender-Male = Male
gender-Other = Other

## Hello
hello-User = Hello, {$user_name}!

## Shared
shared-Photos =
    {$user_name} {$photo_count ->
        [one] added a new photo
       *[other] added {$photo_count} new photos
    } to {$user_gender ->
        [male] his stream
        [female] her stream
       *[other] their stream
    }.
```

### `#[derive(EsFluentKv)]`

You can derive on structs to produce key enums (labels, descriptions, etc.). For example:

```rust
use es_fluent::EsFluentKv;

#[derive(EsFluentKv)]
#[fluent_kv(keys = ["description", "label"])]
pub struct Address {
    pub street: String,
    pub postal_code: String,
}
```

### `#[derive(EsFluentThis)]`

Generates a helper method `this_ftl()` that returns the fluent representation of the parent.

```rust
use es_fluent::{EsFluent, EsFluentKv, EsFluentThis};

#[derive(EsFluent, EsFluentThis)]
#[fluent_this(origin)]
pub enum TeaBlend {
    EarlGrey,
    EnglishBreakfast,
    Darjeeling,
}

#[derive(EsFluent, EsFluentThis)]
#[fluent_this(origin)]
pub enum Drink {
    Tea { blend: TeaBlend },
    Water,
}

#[derive(EsFluent, EsFluentThis)]
#[fluent_this(origin)]
pub struct Hello<'a>(pub &'a str);

#[derive(EsFluentKv, EsFluentThis)]
#[fluent_this(origin, members)]
#[fluent_kv(keys = ["description", "label"])]
pub struct Address {
    pub street: String,
    pub postal_code: String,
}
```

### `#[derive(EsFluentKv)]` on structs

For key-value generation from structs, you can use `EsFluentKv`. This derive is specialized for generating keys for struct fields, often used for UI elements like labels and descriptions.

Here is an example of a `User` struct with various fields:

```rust
use es_fluent::{EsFluent, EsFluentKv};
use rust_decimal::Decimal;
use strum::EnumIter;

#[derive(Clone, Debug, Default, EnumIter, EsFluent, PartialEq)]
pub enum PreferedLanguage {
    #[default]
    English,
    French,
    Chinese,
}

#[derive(Clone, Debug, Default, EnumIter, EsFluent, PartialEq)]
pub enum EnumCountry {
    #[default]
    UnitedStates,
    France,
    China,
}

#[derive(Clone, Debug, Default, EsFluentKv)]
#[fluent_kv(keys = ["description", "label"])]
pub struct User {
    pub username: Option<String>,
    pub email: String,
    pub age: Option<u32>,
    pub balance: Decimal,
    pub subscribe_newsletter: bool,
    pub enable_notifications: bool,
    pub preferred: PreferedLanguage,
    pub country: Option<EnumCountry>,
    pub birth_date: Option<chrono::NaiveDate>,
    pub skip_me: bool,
}
```

The `#[fluent_kv(keys = ["description", "label"])]` attribute instructs the derive to generate enums `UserDescriptionFtl` and `UserLabelFtl`.

This will generate the following FTL entries:

```ftl
## EnumCountry

enum_country-China = China
enum_country-France = France
enum_country-UnitedStates = United States

## PreferedLanguage

prefered_language-Chinese = Chinese
prefered_language-English = English
prefered_language-French = French

## User

user = User

## UserDescriptionFtl

user_description_kv_ftl = User Description Ftl
user_description_kv_ftl-age = Age
user_description_kv_ftl-balance = Balance
user_description_kv_ftl-birth_date = Birth Date
user_description_kv_ftl-country = Country
user_description_kv_ftl-email = Email
user_description_kv_ftl-enable_notifications = Enable Notifications
user_description_kv_ftl-preferred = Preferred
user_description_kv_ftl-skip_me = Skip Me
user_description_kv_ftl-subscribe_newsletter = Subscribe Newsletter
user_description_kv_ftl-username = Username

## UserLabelFtl

user_label_kv_ftl = User Label Ftl
user_label_kv_ftl-age = Age
user_label_kv_ftl-balance = Balance
user_label_kv_ftl-birth_date = Birth Date
user_label_kv_ftl-country = Country
user_label_kv_ftl-email = Email
user_label_kv_ftl-enable_notifications = Enable Notifications
user_label_kv_ftl-preferred = Preferred
user_label_kv_ftl-skip_me = Skip Me
user_label_kv_ftl-subscribe_newsletter = Subscribe Newsletter
user_label_kv_ftl-username = Username
```

### Generics

Generic parameters must convert into Fluent values when used as arguments:

```rust
use es_fluent::{EsFluent, FluentValue};

#[derive(Clone, EsFluent)]
pub struct GenericStruct<T>
where
    T: Into<FluentValue<'static>> + Clone,
{
    pub field: T,
}

#[derive(Clone, EsFluent)]
pub struct GenericTupleStruct<T>(pub T)
where
    T: Into<FluentValue<'static>> + Clone;

#[derive(Clone, EsFluent)]
pub enum GenericEnum<T>
where
    T: Into<FluentValue<'static>> + Clone,
{
    Variant(T),
    StructVariant { field: T },
    Unit,
}
```

## Language Enum Generation

### #[es_fluent_language]

This macro reads your crate's `i18n.toml`, finds all available languages in your `assets_dir`, and generates an enum with a variant for each one. It also implements `Default` (using your `fallback_language`) and conversions to/from `unic_langid::LanguageIdentifier`.

### Usage

Add the dependencies:

```toml
[dependencies]
es-fluent-lang = "*"
unic-langid = "*"
```

Then, apply the macro to an empty enum:

```rs
use es_fluent::EsFluent;
use es_fluent_lang::es_fluent_language;
use strum::EnumIter;

#[es_fluent_language]
#[derive(Clone, Copy, Debug, EnumIter, EsFluent, PartialEq)]
pub enum Languages {}

// The macro generates variants from your i18n asset folders.
// If you have 'en' and 'fr-CA', it generates:
// enum Language { En, FrCA }
```

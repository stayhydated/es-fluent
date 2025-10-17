# es-fluent

Derive macros and utilities for authoring strongly-typed messages with [Project Fluent](https://projectfluent.org/).

This crate gives you:
- Derives to turn enums/structs into Fluent message IDs and arguments.
- A simple API to format values for Fluent and convert them into strings.
- Optional integration via a embedded singleton manager (`es-fluent-manager-embedded`) or for Bevy (`es-fluent-manager-bevy`).

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

To bootstrap `.ftl` files from your Rust types, add the build helper:

```toml
[build-dependencies]
es-fluent-build = "*"
```

And create a `build.rs`:

```rs
// build.rs
use es_fluent_build::FluentParseMode;

fn main() {
    if let Err(e) = es_fluent_build::FluentBuilder::new()
        .mode(FluentParseMode::Conservative)
        .build()
    {
        eprintln!("Error building FTL files: {e}");
    }
}
```

## Project configuration

Create an `i18n.toml` next to your `Cargo.toml`:

```toml
# i18n.toml
assets_dir = "i18n"      # where your localized files live
fallback_language = "en" # default language subdirectory under assets_dir
```

When you run a build, the builder will:
- Discover your crate name,
- Parse Rust sources under `src/`,
- Generate or update a base FTL file at `{assets_dir}/{fallback_language}/{crate_name}.ftl`.

For example, with `assets_dir = "../i18n"` and `fallback_language = "en"`, the file would be `../i18n/en/{crate_name}.ftl`.

## Core derives

### `#[derive(EsFluent)]` on enums

Annotate an enum or a struct to generate message IDs and, optionally, implement `es_fluent::FluentDisplay` or `std::fmt::Display`.

```rs
use es_fluent::EsFluent;

#[derive(EsFluent)]
#[fluent(display = "fluent")] // default; use "std" to implement std::fmt::Display
pub struct HelloUser<'a>(&'a str);

impl<'a> HelloUser<'a> {
    pub fn new(user_name: &'a str) -> Self {
        Self(user_name)
    }
}
```

Fields become Fluent arguments. The derive generates stable keys and formatting logic for you.

### Choices with `EsFluentChoice`

When a message needs to match on an enum (a Fluent select expression), implement `EsFluentChoice`. You can then mark a field with `#[fluent(choice)]` to pass its choice value instead of formatting it as a nested message.

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

### Display strategy

By default, `EsFluent` implements `es_fluent::FluentDisplay`, which formats through Fluent. If you prefer plain Rust `Display` for a type, use:

```rs
#[derive(EsFluent)]
#[fluent(display = "std")]
pub enum AbcStdDisplay {
    A, B, C,
}
```

This also works with `strum::EnumDiscriminants` when you want to display the discriminants.

### `#[derive(EsFluent)]` on structs (keys and “this”)

You can derive on structs to produce key enums (labels, descriptions, etc.). For example:

```rs
use es_fluent::EsFluent;

#[derive(EsFluent)]
#[fluent(display = "std")]
#[fluent(this)] // generates `Address::this_ftl()`
#[fluent(keys = ["Description", "Label"])]
pub struct Address {
    pub street: String,
    pub postal_code: String,
}
```

This expands to enums like `AddressLabelFtl` and `AddressDescriptionFtl` with variants for each field (`Street`, `PostalCode`). They implement the selected display strategy. `this` adds a helper `Address::this_ftl()` that returns the ID of the parent.

## Derive Macro Supported kinds

### Enums

- enum_unit
- enum_named
- enum_tuple

### Structs

- struct_named
- struct_tuple

### Generics

Generic parameters must convert into Fluent values when used as arguments:

```rs
use es_fluent::EsFluent;
use fluent_bundle::FluentValue;

#[derive(EsFluent)]
pub enum GenericFluentDisplay<T>
where
    for<'a> &'a T: Into<FluentValue<'a>>,
{
    A(T),
    B { c: T },
    D,
}
```

## Examples
- [bevy](https://github.com/stayhydated/es-fluent/tree/master/examples/bevy-example)
- [gpui](https://github.com/stayhydated/es-fluent/tree/master/examples/gpui-example)
- [cosmic](https://github.com/stayhydated/es-fluent/tree/master/examples/cosmic-example)
- [iced](https://github.com/stayhydated/es-fluent/tree/master/examples/iced-example)

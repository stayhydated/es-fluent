# es-fluent-derive

The `es-fluent-derive` crate provides the procedural derive macros that power the `es-fluent` localization system. These macros analyze your Rust structs and enums at compile time to automatically generate the necessary boilerplate for them to be used with Fluent.

## Macros

- **`#[derive(EsFluent)]`**: The primary macro that processes your types. It reads `#[fluent(...)]` attributes to understand how to generate translation keys and implement the `FluentDisplay` or `std::fmt::Display` traits.

- **`#[derive(EsFluentChoice)]`**: A specialized macro for enums that are used as selectable variants within a Fluent message (e.g., for gender or pluralization). It implements the `EsFluentChoice` trait, which converts enum variants into strings that Fluent can match against.

## Usage

You typically won't use this crate directly. Instead, you'll enable the `derive` feature on the main `es-fluent` crate and use the macros through it.

```rs
use es_fluent::{EsFluent, EsFluentChoice};

#[derive(EsFluent)]
#[fluent(keys = ["label"])]
pub struct User {
    pub name: String,
    pub age: u32,
}

#[derive(EsFluent, EsFluentChoice)]
#[fluent_choice(serialize_all = "snake_case")]
pub enum Gender {
    Male,
    Female,
    Other,
}
```

For more detailed examples and attribute documentation, please refer to the top-level `es-fluent` crate documentation.

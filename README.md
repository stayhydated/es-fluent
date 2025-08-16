# es-fluent

A enum/struct derive macro for [Project Fluent](https://projectfluent.org/).

given [Project Fluent](https://projectfluent.org/)'s first example

```ftl
# Simple things are simple.
hello-user = Hello, {$userName}!

# Complex things are possible.
shared-photos =
    {$userName} {$photoCount ->
        [one] added a new photo
       *[other] added {$photoCount} new photos
    } to {$userGender ->
        [male] his stream
        [female] her stream
       *[other] their stream
    }.
```

This could be represented as

```rs
use es_fluent::{EsFluent, EsFluentChoice};

#[derive(EsFluent)]
#[fluent(display = "fluent")] // optional, defaults to "fluent" if not specified
pub enum Hello<'a> {
    User { user_name: &'a str },
}

#[derive(EsFluent, EsFluentChoice)]
#[fluent_choice(serialize_all = "snake_case")]
pub enum Gender {
    Male,
    Female,
    Helicopter,
    Other,
}

#[derive(EsFluent)]
pub enum Shared<'a> {
    Photos {
        user_name: &'a str,
        photo_count: &'a u32,
        // this signals the macro to use the choice representation, since we'll
        // match against it in the ftl resource
        #[fluent(choice)]
        user_gender: &'a Gender,
    },
}
```

using any of the *prototyping tools* for building the `.ftl` resources, we'd get

```ftl
## Gender

gender-Female = Female
gender-Helicopter = Helicopter
gender-Male = Male
gender-Other = Other

## Hello

hello-User = User { $user_name }

## Shared

shared-Photos = Photos { $user_name } { $photo_count } { $user_gender }
```

which we would modify to our needs:

```ftl
## Gender

gender-Female = Female
gender-Helicopter = Helicopter
gender-Male = Male
gender-Other = Other

## Hello

hello-User = Hello, {$user_name}!

## Shared

shared-Photos = {$user_name} {$photo_count ->
        [one] added a new photo
       *[other] added {$photo_count} new photos
    } to {$user_gender ->
        [male] his stream
        [female] her stream
       *[other] their stream
    }.
```

see the output with

```sh
cargo run -p first-example
```

## Generics

all generics need to impl `for<'a> &'a {type parameter}: Into<FluentValue<'a>>`, like

```rust
#[derive(EsFluent)]
#[fluent(display = "fluent")] // optional, defaults to "fluent" if not specified
pub enum GenericFluentDisplay<T>
where
    for<'a> &'a T: Into<FluentValue<'a>>,
{
    A(T),
    B { c: T },
    D,
}
```

## Display Trait

By default, the macro will only implement the `es_fluent::FluentDisplay` trait.

Its also possible for it to only implement the `std::fmt::Display` trait. By manually adding `#[fluent(display = "std")]`, such as

```rs
#[derive(EsFluent)]
#[fluent(display = "std")]
pub enum AbcStdDisplay {
    A,
    B,
    C,
}
```

This can also be implemented for `strum::EnumDiscriminants`, such as

```rs
#[derive(EnumDiscriminants, EsFluent)]
#[fluent(display = "std")]
// doesn't need to be condensed into a single block, using multiple block is fine.
#[strum_discriminants(vis(pub), derive(EsFluent), fluent(display = "std"))]
pub enum PaymentTypeStdDisplay {
    Cash(f64),
    CreditCard { amount: f64, card_number: String },
    Robbery,
}
```

## On Structs

given

```rs
#[derive(EsFluent)]
#[fluent(display = "std")] // set display impl to `std::fmt::Display`
#[fluent(keys = ["Description", "Label"])] // generates specialized names
#[fluent(derive(Clone))] // custom derives you'd define
#[fluent(this)] // describes the item ident (name) as a fn, `{}::this_ftl()`
pub struct Address {
    pub street: String,
    pub postal_code: String,
}
```

this will expand to

```rs
impl Address {
  pub fn this_ftl() -> String {/* */}
}

#[derive(EsFluent, Clone)]
#[fluent(display = "std")]
pub enum AddressLabelFtl {
    Street,
    PostalCode,
}

impl AddressLabelFtl {
  pub fn this_ftl() -> String {/* */}
}

#[derive(EsFluent, Clone)]
#[fluent(display = "std")]
pub enum AddressDescriptionFtl {
    Street,
    PostalCode,
}

impl AddressDescriptionFtl {
  pub fn this_ftl() -> String {/* */}
}

```

if no keys are provided, this will expand to

```rs
pub enum AddressFtl {
    Street,
    PostalCode,
}
```

## Simple examples

see the [examples](examples) dir to see a bunch of ways to use this crate. where i18n resources are placed in the [i18n](examples/i18n) dir.

## [Gpui](https://gpui.rs) example

<https://github.com/stayhydated/gpui-form/tree/master/examples>

## Prototyping tools

[Build.rs script](crates/es-fluent-build/README.md)

[Cli](crates/es-fluent-cli/README.md)

## Notes

- `es-fluent` expects you to provide a `fl!` macro, accessible via `crate::fl`
- This macro was designed around [i18n-embed-fl](https://github.com/kellpossible/cargo-i18n/tree/master/i18n-embed-fl) and [i18n-embed](https://github.com/kellpossible/cargo-i18n/tree/master/i18n-embed). Decoupling from it is planned in the future. For now, this satisfies my use case with [gpui](https://gpui.rs). Feel free to contribute to the project!

## Derive Macro Supported kinds

### Enums

- enum_unit
- enum_named
- enum_tuple

### Structs

- struct_named

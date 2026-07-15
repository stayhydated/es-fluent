# Deriving Messages

The `EsFluent` derive macro turns a struct or enum into a localizable message. Each type maps to one or more keys in your `.ftl` files, and fields become Fluent arguments.

- **Enums**: Each variant becomes a message ID (e.g., `MyEnum::Variant` → `my_enum-Variant`).
- **Structs**: The struct itself becomes the message ID (e.g., `MyStruct` → `my_struct`).
- **Fields**: Fields are automatically exposed as arguments to the Fluent message.

```rust
use es_fluent::EsFluent;

#[derive(EsFluent)]
pub enum LoginError {
    InvalidPassword,                   // no params
    UserNotFound { username: String }, // exposed as $username in the ftl file
    Something(String, String, String), // exposed as $f0, $f1, $f2 in the ftl file
    SomethingArgNamed(
        #[fluent(arg = "input")] String,
        #[fluent(arg = "expected")] String,
        #[fluent(arg = "details")] String,
    ), // exposed as $input, $expected, $details
}

#[derive(EsFluent)]
pub struct WelcomeMessage<'a> {
    pub name: &'a str, // exposed as $name in the ftl file
    pub count: i32,    // exposed as $count in the ftl file
}
```

The CLI generates the following FTL entries for these types:

```ftl
## LoginError

login_error-InvalidPassword = Invalid Password
login_error-Something = Something { $f0 } { $f1 } { $f2 }
login_error-SomethingArgNamed = Something Arg Named { $input } { $expected } { $details }
login_error-UserNotFound = User Not Found { $username }

## WelcomeMessage

welcome_message = Welcome Message { $name } { $count }
```

At runtime, call `i18n.localize_message(&value)` on an explicit manager to resolve translations:

```rust
let _ = i18n.localize_message(&LoginError::InvalidPassword);
let _ = i18n.localize_message(&LoginError::UserNotFound { username: "john".to_string() });
let _ = i18n.localize_message(&LoginError::Something("a".to_string(), "b".to_string(), "c".to_string()));
let _ = i18n.localize_message(&LoginError::SomethingArgNamed("a".to_string(), "b".to_string(), "c".to_string()));

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

## Localized Temporal Arguments

Enable the feature for the date/time library used by your message fields:

```toml
[dependencies]
es-fluent = { version = "*", features = ["chrono"] }
chrono = "0.4"
```

Temporal fields work like other derived arguments, including borrowed fields,
`Option<T>`, and values returned by `#[fluent(value = ...)]`:

```rust
use chrono::{DateTime, Utc};
use es_fluent::EsFluent;

#[derive(EsFluent)]
pub struct EventStartsAt {
    pub starts_at: DateTime<Utc>,
}
```

The generated argument can be interpolated directly in FTL:

```ftl
event_starts_at = Starts { $starts_at }
```

| Feature | Supported field types |
| --- | --- |
| `icu-datetime` | ICU4X `Date<Gregorian>`, `Time`, `DateTime<Gregorian>`, and `ZonedDateTime<Gregorian, TimeZoneInfo<AtTime>>` |
| `chrono` | `NaiveDate`, `NaiveTime`, `NaiveDateTime`, and `DateTime<Tz>` for any `Tz: TimeZone` |
| `jiff` | `civil::Date`, `civil::Time`, `civil::DateTime`, `Timestamp`, `Zoned`, `Span`, and `SignedDuration` |

Calendar, time, instant, and zoned values use ICU4X's medium localized formats
for the manager's active Fluent locale. Zoned values include a localized short
UTC offset; Jiff `Timestamp` values are rendered in UTC. Jiff `Span` and
`SignedDuration` arguments use Jiff's friendly duration format.

Skipped single-field enum variants:

`#[fluent(skip)]` on a single-field enum variant suppresses that variant's own
key and delegates context-bound rendering to the wrapped value. This is useful for
transparent wrapper enums.

```rust
use es_fluent::{EsFluent, FluentMessage};

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

## Using Choices

Choices allow an enum to be used _inside_ another message as a Fluent selector (e.g., for gender or category). Unit-only enums that derive `EsFluent` infer `EsFluentChoice` automatically.
Variants serialize as kebab-case by default, so `GenderChoice::Male` becomes
`male` and a compound variant like `VeryFriendly` becomes `very-friendly`.
Derived choice values are emitted as validated `StaticFluentVariantKey` values.
Use `#[fluent_choice(rename_all = "...")]` on the same enum to change selector casing. Styles that generate invalid selector values, such as values containing spaces, are rejected at compile time. Use standalone `#[derive(EsFluentChoice)]` only for selector enums that should not also be registered as messages.

```rust
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
```

In the FTL file, the selector field can drive a selector:

```ftl
greeting = { $gender ->
    [male] Welcome Mr. { $name }
    [female] Welcome Ms. { $name }
   *[other] Welcome { $name }
}
```

```rust
use es_fluent::FluentMessage;
let greeting = Greeting { name: "John", gender: Some(&GenderChoice::Male) };
let _ = i18n.localize_message(&greeting);
```

## Generating Variants

`EsFluentVariants` generates key-value pair enums for struct fields or enum
variants. This is useful for generating UI labels, placeholders, or
descriptions for a form object, and it can also expose enum variants as
localizable keys.

```rust
use es_fluent::{EsFluent, EsFluentVariants};

#[derive(EsFluentVariants)]
#[fluent_variants(keys = ["label", "description"])]
pub struct LoginFormVariants {
    pub username: String,
    pub password: String,
}

#[derive(EsFluent)]
pub struct ActiveFormField {
    #[fluent(selector)]
    pub field: LoginFormVariantsLabelVariants,
}
```

This generates two enums with corresponding FTL entries:

```ftl
## LoginFormVariantsLabelVariants

login_form_variants_label_variants-password = Password
login_form_variants_label_variants-username = Username

## LoginFormVariantsDescriptionVariants

login_form_variants_description_variants-password = Password
login_form_variants_description_variants-username = Username
```

```rust
use es_fluent::FluentMessage;
let _ = i18n.localize_message(&LoginFormVariantsLabelVariants::Username);
let _ = i18n.localize_message(&ActiveFormField {
    field: LoginFormVariantsLabelVariants::Username,
});
```

Generated variant enums also implement `EsFluentChoice`, so they can drive
selector fields:

```ftl
active_form_field =
    { $field ->
        [username] Editing username
       *[password] Editing password
    }
```

Enums are supported too. In that case, the derive generates a single
`...Variants` enum over the original variants:

```rust
use es_fluent::EsFluentVariants;

#[derive(EsFluentVariants)]
pub enum SettingsTab {
    General,
    Notifications,
    Privacy,
}
```

```ftl
## SettingsTabVariants

settings_tab_variants-General = General
settings_tab_variants-Notifications = Notifications
settings_tab_variants-Privacy = Privacy
```

```rust
use es_fluent::FluentMessage;
let _ = i18n.localize_message(&SettingsTabVariants::Notifications);
```

`keys = [...]` values must be lowercase snake_case. Use
`#[fluent_variants(skip)]` to omit a struct field or enum variant from the
generated enums. Generated enums derive `Clone`, `Copy`, `Debug`, `Eq`, `Hash`,
and `PartialEq` automatically and implement `EsFluentChoice`, so they can be
used directly in `#[fluent(selector)]` fields. Use `derive(...)` inside
`#[fluent_variants(...)]` for additional traits; `EsFluentChoice` is already
inferred.

## Type-level Labels

`EsFluentLabel` generates a `FluentLabel` implementation that registers the type's _name_ as a key. Where `EsFluentVariants` registers individual fields, `EsFluentLabel` registers the parent type itself.

### Type Label

`#[derive(EsFluentLabel)]` creates a single key for the type. The derive is
enough to register the type label; no additional `#[fluent_label(...)]` flag is
needed for the parent type itself.

```rust
use es_fluent::EsFluentLabel;

#[derive(EsFluentLabel)]
pub enum GenderLabelOnly {
    Male,
    Female,
    Other,
}
```

```ftl
gender_label_only_label = Gender Label Only
```

```rust
use es_fluent::FluentLabel;
let _ = GenderLabelOnly::localize_label(&i18n);
let _ = GenderLabelOnly::try_localize_label(&i18n);
let _ = GenderLabelOnly::fluent_label_id();
```

### Combined with Generated Variant Labels

`#[derive(EsFluentVariants)]` also gives each generated variant enum a
type-level label key inferred from the generated enum name:

```rust
use es_fluent::{EsFluentLabel, EsFluentVariants};

#[derive(EsFluentLabel, EsFluentVariants)]
#[fluent_variants(keys = ["label", "description"])]
pub struct LoginFormCombined {
    pub username: String,
    pub password: String,
}
```

```ftl
login_form_combined_label_variants_label = Login Form Combined Label Variants
login_form_combined_description_variants_label = Login Form Combined Description Variants
```

```rust
use es_fluent::FluentLabel;
let _ = LoginFormCombinedDescriptionVariants::localize_label(&i18n);
```

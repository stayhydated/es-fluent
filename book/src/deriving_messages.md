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
        #[fluent(arg_name = "input")] String,
        #[fluent(arg_name = "expected")] String,
        #[fluent(arg_name = "details")] String,
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

Choices allow an enum to be used _inside_ another message as a Fluent selector (e.g., for gender or category). Derive `EsFluentChoice` alongside `EsFluent` on the selector enum.

```rust
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
```

In the FTL file, the choice field can drive a selector:

```ftl
greeting = { $gender ->
    [male] Welcome Mr. { $name }
    [female] Welcome Ms. { $name }
   *[other] Welcome { $name }
}
```

```rust
use es_fluent::FluentMessage;
let greeting = Greeting { name: "John", gender: &GenderChoice::Male };
let _ = i18n.localize_message(&greeting);
```

## Generating Variants

`EsFluentVariants` generates key-value pair enums for struct fields or enum
variants. This is useful for generating UI labels, placeholders, or
descriptions for a form object, and it can also expose enum variants as
localizable keys.

```rust
use es_fluent::EsFluentVariants;

#[derive(EsFluentVariants)]
#[fluent_variants(keys = ["label", "description"])]
pub struct LoginFormVariants {
    pub username: String,
    pub password: String,
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
generated enums. Use `derive(Debug, Clone)` inside `#[fluent_variants(...)]` to
add derives to the generated enums.

## Type-level Labels

`EsFluentLabel` generates a `FluentLabel` implementation that registers the type's _name_ as a key. Where `EsFluentVariants` registers individual fields, `EsFluentLabel` registers the parent type itself.

### Origin Only

`#[fluent_label(origin)]` creates a single key for the type:

```rust
use es_fluent::EsFluentLabel;

#[derive(EsFluentLabel)]
#[fluent_label(origin)]
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
```

### Combined with Variants

`#[fluent_label(variants)]` can be combined with `EsFluentVariants` to generate type-level keys for each generated variant enum:

```rust
use es_fluent::{EsFluentLabel, EsFluentVariants};

#[derive(EsFluentLabel, EsFluentVariants)]
#[fluent_label(origin, variants)]
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

#![doc = include_str!("../README.md")]

use proc_macro_error2::proc_macro_error;

mod macros;

/// Turns an enum or struct into a localizable message.
///
/// - **Enums**: Each variant becomes a message ID (e.g., `MyEnum::Variant` -> `my_enum-Variant`).
/// - **Structs**: The struct itself becomes the message ID (e.g., `MyStruct` -> `my_struct`).
/// - **Fields**: Fields are automatically exposed as arguments to the Fluent message.
///
/// # Example
///
/// ```ignore
/// use es_fluent::EsFluent;
///
/// #[derive(EsFluent)]
/// pub enum LoginError {
///     InvalidPassword, // no params
///     UserNotFound { username: String }, // exposed as $username in the ftl file
///     Something(String, String, String), // exposed as $f1, $f2, $f3 in the ftl file
/// }
///
/// #[derive(EsFluent)]
/// pub struct UserProfile<'a> {
///     pub name: &'a str, // exposed as $name in the ftl file
///     pub gender: &'a str, // exposed as $gender in the ftl file
/// }
/// ```
///
/// # Field Attributes
///
/// - `#[fluent(choice)]`: Marks a field as a selector for Fluent's select expression.
#[proc_macro_derive(EsFluent, attributes(fluent))]
#[proc_macro_error]
pub fn derive_es_fluent(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    macros::derive_es_fluent::from(input)
}

/// Generates key-value pair enums for struct fields.
///
/// This is perfect for generating UI labels, placeholders, or descriptions for a form object.
///
/// # Example
///
/// ```ignore
/// use es_fluent::EsFluentKv;
///
/// #[derive(EsFluentKv)]
/// #[fluent_kv(keys = ["label", "description"])]
/// pub struct LoginForm {
///     pub username: String,
///     pub password: String,
/// }
///
/// // Generates enums -> keys:
/// // LoginFormLabelKvFtl::{Variants} -> (login_form_label_kv_ftl-{variant})
/// // LoginFormDescriptionKvFtl::{Variants} -> (login_form_description_kv_ftl-{variant})
/// ```
///
/// # Container Attributes
///
/// - `#[fluent_kv(keys = ["label", "description"])]`: Specifies which key variants to generate.
#[proc_macro_derive(EsFluentKv, attributes(fluent_kv))]
#[proc_macro_error]
pub fn derive_es_fluent_kv(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    macros::derive_es_fluent_kv::from(input)
}

/// Allows an enum to be used inside another message as a selector (e.g., for gender or status).
///
/// # Example
///
/// ```ignore
/// use es_fluent::{EsFluent, EsFluentChoice};
///
/// #[derive(EsFluent, EsFluentChoice)]
/// #[fluent_choice(serialize_all = "snake_case")]
/// pub enum Gender {
///     Male,
///     Female,
///     Other,
/// }
///
/// #[derive(EsFluent)]
/// pub struct UserProfile<'a> {
///     pub name: &'a str,
///     #[fluent(choice)] // Matches $gender -> [male]...
///     pub gender: &'a Gender,
/// }
/// ```
///
/// # Container Attributes
///
/// - `#[fluent_choice(serialize_all = "...")]`: Controls variant name serialization (e.g., `"snake_case"`).
#[proc_macro_derive(EsFluentChoice, attributes(fluent_choice))]
#[proc_macro_error]
pub fn derive_fluent_choice(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    macros::derive_fluent_choice::from(input)
}

/// Generates a helper implementation of the `ThisFtl` trait and registers the type's name as a key.
///
/// This is similar to `EsFluentKv` (which registers fields), but for the parent type itself.
///
/// # Example
///
/// ```ignore
/// use es_fluent::EsFluentThis;
///
/// #[derive(EsFluentThis)]
/// #[fluent_this(origin)]
/// pub enum Gender {
///     Male,
///     Female,
///     Other,
/// }
///
/// // Generates key: (gender_this)
/// // Usage: Gender::this_ftl()
/// ```
///
/// # Attributes
///
/// - `#[fluent_this(origin)]`: Generates an implementation where `this_ftl()` returns the base key for the type.
/// - `#[fluent_this(members)]`: Can be combined with `Kv` derives to generate keys for the generated member enums.
/// - `#[fluent_this(origin, members)]`: Combines both behaviors.
#[proc_macro_derive(EsFluentThis, attributes(fluent_this))]
#[proc_macro_error]
pub fn derive_es_fluent_this(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    macros::derive_es_fluent_this::from(input)
}

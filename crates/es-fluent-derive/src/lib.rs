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
///     Something(String, String, String), // exposed as $f0, $f1, $f2 in the ftl file
///     SomethingArgNamed(
///         #[fluent(arg_name = "input")] String,
///         #[fluent(arg_name = "expected")] String,
///         #[fluent(arg_name = "details")] String,
///     ), // exposed as $input, $expected, $details
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
/// - `#[fluent(arg_name = "value")]`: Overrides a field argument name on any exposed message field (struct fields, enum named fields, or enum tuple fields). On tuple variants, this can also be used at variant level as a single-field shorthand.
#[proc_macro_derive(EsFluent, attributes(fluent))]
#[proc_macro_error]
pub fn derive_es_fluent(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    macros::derive_es_fluent::from(input)
}

/// Generates variant enums for struct fields.
///
/// This is perfect for generating UI labels, placeholders, or descriptions for a form object.
///
/// # Example
///
/// ```ignore
/// use es_fluent::EsFluentVariants;
///
/// #[derive(EsFluentVariants)]
/// #[fluent_variants(keys = ["label", "description"])]
/// pub struct LoginForm {
///     pub username: String,
///     pub password: String,
/// }
///
/// // Generates enums -> keys:
/// // LoginFormLabelVariants::{Variants} -> (login_form_label_variants-{variant})
/// // LoginFormDescriptionVariants::{Variants} -> (login_form_description_variants-{variant})
/// ```
///
/// # Container Attributes
///
/// - `#[fluent_variants(keys = ["label", "description"])]`: Specifies which key variants to generate.
/// - `#[fluent(namespace = "...")]`: Routes generated registrations to a namespaced FTL file.
#[proc_macro_derive(EsFluentVariants, attributes(fluent_variants, fluent))]
#[proc_macro_error]
pub fn derive_es_fluent_variants(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    macros::derive_es_fluent_variants::from(input)
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
/// This is similar to `EsFluentVariants` (which registers fields), but for the parent type itself.
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
/// - `#[fluent_this(variants)]`: Can be combined with `EsFluentVariants` derives to generate keys for the generated variant enums.
/// - `#[fluent_this(origin, variants)]`: Combines both behaviors.
/// - `#[fluent(namespace = "...")]`: Routes generated registrations to a namespaced FTL file.
#[proc_macro_derive(EsFluentThis, attributes(fluent_this, fluent))]
#[proc_macro_error]
pub fn derive_es_fluent_this(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    macros::derive_es_fluent_this::from(input)
}

#![doc = include_str!("../README.md")]

use proc_macro_error2::proc_macro_error;

mod macros;
#[cfg(test)]
mod snapshot_support;

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
/// - `#[fluent(arg_name = "value")]`: On a field, renames that exposed Fluent argument (works on struct fields, enum named fields, and enum tuple fields).
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
#[proc_macro_derive(EsFluentVariants, attributes(fluent_variants, fluent_label, fluent))]
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

/// Generates a helper implementation of the `FluentLabel` trait and registers the type's name as a key.
///
/// This is similar to `EsFluentVariants` (which registers fields), but for the parent type itself.
///
/// # Example
///
/// ```ignore
/// use es_fluent::{EsFluentLabel, FluentLabel as _};
///
/// #[derive(EsFluentLabel)]
/// pub enum Gender {
///     Male,
///     Female,
///     Other,
/// }
///
/// // Generates key: (gender_label)
/// // Usage: Gender::localize_label(&i18n)
/// ```
///
/// # Attributes
///
/// - `origin`: Enabled by default. `#[derive(EsFluentLabel)]` and `#[derive(EsFluentLabel)] #[fluent_label(origin)]` both generate the type-level label. Use `#[fluent_label(origin = false)]` when deriving only variant labels through `EsFluentVariants`.
/// - `#[fluent_label(origin)]`: Explicitly generates an implementation where `localize_label(localizer)` returns the base key for the type.
/// - `#[fluent_label(variants)]`: Can be combined with `EsFluentVariants` derives to generate keys for the generated variant enums.
/// - `#[fluent_label(origin, variants)]`: Combines both behaviors.
/// - `#[fluent(namespace = "...")]`: Routes generated registrations to a namespaced FTL file.
#[proc_macro_derive(EsFluentLabel, attributes(fluent_label, fluent))]
#[proc_macro_error]
pub fn derive_es_fluent_label(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    macros::derive_es_fluent_label::from(input)
}

use crate::registry::StaticFluentVariantKey;

/// Converts an enum into a validated Fluent select variant key.
///
/// Unit-only enums that derive `EsFluent` implement this trait automatically.
/// Implement it manually, or derive `EsFluentChoice`, for selector-only enums
/// that should not also be registered as localized messages.
///
/// # Example
///
/// ```rs
/// use es_fluent::{registry::StaticFluentVariantKey, EsFluentChoice};
///
/// enum MyEnum {
///     Variant1,
///     Variant2,
/// }
///
/// impl EsFluentChoice for MyEnum {
///     fn as_fluent_choice(&self) -> StaticFluentVariantKey {
///         match self {
///             MyEnum::Variant1 => StaticFluentVariantKey::try_new("variant1").expect("valid choice"),
///             MyEnum::Variant2 => StaticFluentVariantKey::try_new("variant2").expect("valid choice"),
///         }
///     }
/// }
///
/// let my_enum = MyEnum::Variant1;
/// assert_eq!(my_enum.as_fluent_choice(), "variant1");
/// ```
pub trait EsFluentChoice {
    fn as_fluent_choice(&self) -> StaticFluentVariantKey;
}

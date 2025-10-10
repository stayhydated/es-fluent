/// A trait for converting a type into a Fluent choice.
///
/// This trait is used to convert an enum into a string that can be used as a
/// Fluent choice.
///
/// # Example
///
/// ```rust
/// use es_fluent::EsFluentChoice;
///
/// enum MyEnum {
///     Variant1,
///     Variant2,
/// }
///
/// impl EsFluentChoice for MyEnum {
///     fn as_fluent_choice(&self) -> &'static str {
///         match self {
///             MyEnum::Variant1 => "variant1",
///             MyEnum::Variant2 => "variant2",
///         }
///     }
/// }
///
/// let my_enum = MyEnum::Variant1;
/// assert_eq!(my_enum.as_fluent_choice(), "variant1");
/// ```
pub trait EsFluentChoice {
    /// Converts the type into a Fluent choice.
    fn as_fluent_choice(&self) -> &'static str;
}

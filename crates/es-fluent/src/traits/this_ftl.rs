/// A trait for types that have a "this" fluent key representing the type itself.
///
/// This trait is automatically implemented by `#[derive(EsFluentThis)]`, or by
/// `#[derive(EsFluentVariants)]` when combined with `#[fluent_this(members)]`.
///
/// The `this_ftl()` method returns the localized string for the type's own
/// fluent key, without any variant or field suffix.
pub trait ThisFtl {
    /// Returns the localized string for this type's fluent key.
    fn this_ftl() -> String;
}

use super::{FluentLocalizer, FluentLocalizerExt};

/// A trait for types that have a "this" Fluent key representing the type itself.
///
/// This trait is automatically implemented by `#[derive(EsFluentThis)]`, or by
/// `#[derive(EsFluentVariants)]` when combined with `#[fluent_this(variants)]`.
pub trait ThisFtl {
    /// Returns the localized string for this type's own Fluent key using an
    /// explicit localization context.
    fn this_ftl<L: FluentLocalizer + ?Sized>(localizer: &L) -> String;
}

#[doc(hidden)]
pub fn localize_this<L: FluentLocalizer + ?Sized>(localizer: &L, domain: &str, id: &str) -> String {
    localizer.localize_in_domain_or_id(domain, id, None)
}

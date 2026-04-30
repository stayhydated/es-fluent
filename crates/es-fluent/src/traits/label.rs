use super::FluentLocalizer;

/// A trait for types that have a Fluent label key representing the type itself.
///
/// This trait is automatically implemented by `#[derive(EsFluentLabel)]`, or by
/// `#[derive(EsFluentVariants)]` when combined with `#[fluent_label(variants)]`.
pub trait FluentLabel {
    /// Returns the localized label for this type using an explicit localization
    /// context.
    fn localize_label<L: FluentLocalizer + ?Sized>(localizer: &L) -> String;
}

#[doc(hidden)]
pub fn localize_label<L: FluentLocalizer + ?Sized>(
    localizer: &L,
    domain: &str,
    id: &str,
) -> String {
    localizer
        .localize_in_domain(domain, id, None)
        .unwrap_or_else(|| {
            tracing::warn!(domain, message_id = id, "missing Fluent label");
            id.to_string()
        })
}

#[cfg(test)]
mod tests;

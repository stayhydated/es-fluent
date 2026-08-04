use super::FluentLocalizer;
use crate::registry::StaticFluentMessageKey;

/// A trait for types that have a Fluent label key representing the type itself.
///
/// This trait is automatically implemented by `#[derive(EsFluentLabel)]` for
/// source types, and by `#[derive(EsFluentVariants)]` for each generated
/// variant enum.
pub trait FluentLabel {
    /// Returns the fully scoped static key for this type-level label.
    fn fluent_label_key() -> StaticFluentMessageKey;

    /// Attempts to return the localized label for this type using an explicit
    /// localization context.
    fn try_localize_label<L: FluentLocalizer + ?Sized>(localizer: &L) -> Option<String> {
        try_localize_label(localizer, Self::fluent_label_key())
    }

    /// Returns the localized label for this type using an explicit localization
    /// context.
    fn localize_label<L: FluentLocalizer + ?Sized>(localizer: &L) -> String {
        localize_label(localizer, Self::fluent_label_key())
    }
}

#[doc(hidden)]
pub fn try_localize_label<L: FluentLocalizer + ?Sized>(
    localizer: &L,
    key: StaticFluentMessageKey,
) -> Option<String> {
    localizer.localize(key, None)
}

#[doc(hidden)]
pub fn localize_label<L: FluentLocalizer + ?Sized>(
    localizer: &L,
    key: StaticFluentMessageKey,
) -> String {
    localizer.localize(key, None).unwrap_or_else(|| {
        panic!(
            "missing Fluent label `{}` in domain `{}` owned by `{}`",
            key.id().as_str(),
            key.domain().as_str(),
            key.owner().as_str(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FluentArgs;

    fn static_key(id: &'static str) -> StaticFluentMessageKey {
        crate::registry::__macro::static_message_key(
            "label-owner",
            crate::registry::__macro::static_domain("label-domain"),
            crate::registry::__macro::static_entry_id(id),
        )
    }

    struct LabelLocalizer;

    impl FluentLocalizer for LabelLocalizer {
        fn localize<'a>(
            &self,
            key: StaticFluentMessageKey,
            _args: Option<&FluentArgs<'a>>,
        ) -> Option<String> {
            (key.owner() == "label-owner"
                && key.domain() == "label-domain"
                && key.id() == "label-id")
                .then(|| "Label".to_string())
        }
    }

    struct TestLabel;

    impl FluentLabel for TestLabel {
        fn fluent_label_key() -> StaticFluentMessageKey {
            static_key("label-id")
        }
    }

    #[test]
    fn label_trait_exposes_typed_metadata_and_localizes() {
        let localizer = LabelLocalizer;

        assert_eq!(TestLabel::fluent_label_key().owner(), "label-owner");
        assert_eq!(TestLabel::fluent_label_key().domain(), "label-domain");
        assert_eq!(TestLabel::fluent_label_key().id(), "label-id");
        assert_eq!(
            TestLabel::try_localize_label(&localizer),
            Some("Label".into())
        );
        assert_eq!(TestLabel::localize_label(&localizer), "Label");
    }

    #[test]
    fn localize_label_helpers_return_localized_values_or_explicitly_report_missing_values() {
        let localizer = LabelLocalizer;

        assert_eq!(
            try_localize_label(&localizer, static_key("label-id")),
            Some("Label".into())
        );
        assert_eq!(
            try_localize_label(&localizer, static_key("missing-id")),
            None
        );
        assert_eq!(localize_label(&localizer, static_key("label-id")), "Label");
    }

    #[test]
    #[should_panic(
        expected = "missing Fluent label `missing-id` in domain `label-domain` owned by `label-owner`"
    )]
    fn localize_label_panics_when_the_typed_label_is_missing() {
        let _ = localize_label(&LabelLocalizer, static_key("missing-id"));
    }
}

use super::FluentLocalizer;
use crate::registry::{StaticFluentDomain, StaticFluentEntryId};

/// A trait for types that have a Fluent label key representing the type itself.
///
/// This trait is automatically implemented by `#[derive(EsFluentLabel)]` for
/// source types, and by `#[derive(EsFluentVariants)]` for each generated
/// variant enum.
pub trait FluentLabel {
    /// Returns the validated static domain for this type-level label.
    fn fluent_label_domain() -> StaticFluentDomain;

    /// Returns the validated static message id for this type-level label.
    fn fluent_label_id() -> StaticFluentEntryId;

    /// Attempts to return the localized label for this type using an explicit
    /// localization context.
    fn try_localize_label<L: FluentLocalizer + ?Sized>(localizer: &L) -> Option<String> {
        try_localize_label(
            localizer,
            Self::fluent_label_domain(),
            Self::fluent_label_id(),
        )
    }

    /// Returns the localized label for this type using an explicit localization
    /// context.
    fn localize_label<L: FluentLocalizer + ?Sized>(localizer: &L) -> String {
        localize_label(
            localizer,
            Self::fluent_label_domain(),
            Self::fluent_label_id(),
        )
    }
}

#[doc(hidden)]
pub fn try_localize_label<L: FluentLocalizer + ?Sized>(
    localizer: &L,
    domain: StaticFluentDomain,
    id: StaticFluentEntryId,
) -> Option<String> {
    localizer.localize_in_domain(domain, id, None)
}

#[doc(hidden)]
pub fn localize_label<L: FluentLocalizer + ?Sized>(
    localizer: &L,
    domain: StaticFluentDomain,
    id: StaticFluentEntryId,
) -> String {
    localizer
        .localize_in_domain(domain, id, None)
        .unwrap_or_else(|| {
            panic!(
                "missing Fluent label `{}` in domain `{}`",
                id.as_str(),
                domain.as_str(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FluentArgs;

    fn static_domain(value: &'static str) -> StaticFluentDomain {
        StaticFluentDomain::try_new(value).expect("valid test domain")
    }

    fn static_entry(value: &'static str) -> StaticFluentEntryId {
        StaticFluentEntryId::try_new(value).expect("valid test message id")
    }

    struct LabelLocalizer;

    impl FluentLocalizer for LabelLocalizer {
        fn localize<'a>(
            &self,
            id: StaticFluentEntryId,
            _args: Option<&FluentArgs<'a>>,
        ) -> Option<String> {
            (id == "label-id").then(|| "Label".to_string())
        }

        fn localize_in_domain<'a>(
            &self,
            domain: StaticFluentDomain,
            id: StaticFluentEntryId,
            args: Option<&FluentArgs<'a>>,
        ) -> Option<String> {
            (domain == "label-domain")
                .then(|| self.localize(id, args))
                .flatten()
        }
    }

    struct TestLabel;

    impl FluentLabel for TestLabel {
        fn fluent_label_domain() -> StaticFluentDomain {
            static_domain("label-domain")
        }

        fn fluent_label_id() -> StaticFluentEntryId {
            static_entry("label-id")
        }
    }

    #[test]
    fn label_trait_exposes_typed_metadata_and_localizes() {
        let localizer = LabelLocalizer;

        assert_eq!(TestLabel::fluent_label_domain(), "label-domain");
        assert_eq!(TestLabel::fluent_label_id(), "label-id");
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
            try_localize_label(
                &localizer,
                static_domain("label-domain"),
                static_entry("label-id")
            ),
            Some("Label".into())
        );
        assert_eq!(
            try_localize_label(
                &localizer,
                static_domain("label-domain"),
                static_entry("missing-id")
            ),
            None
        );
        assert_eq!(
            localize_label(
                &localizer,
                static_domain("label-domain"),
                static_entry("label-id")
            ),
            "Label"
        );
    }

    #[test]
    #[should_panic(expected = "missing Fluent label `missing-id` in domain `label-domain`")]
    fn localize_label_panics_when_the_typed_label_is_missing() {
        let _ = localize_label(
            &LabelLocalizer,
            static_domain("label-domain"),
            static_entry("missing-id"),
        );
    }
}

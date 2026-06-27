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

    /// Returns deterministic fallback text for this type label without a
    /// runtime localization context.
    ///
    /// Prefer [`Self::localize_label`] when UI code has a runtime manager.
    /// This helper is intended for generated metadata, tests, and integration
    /// scaffolding that cannot access app state.
    fn fallback_label() -> String {
        fallback_label::<Self>()
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
            tracing::warn!(
                domain = domain.as_str(),
                message_id = id.as_str(),
                "missing Fluent label"
            );
            id.as_str().to_string()
        })
}

/// Returns deterministic fallback text for a typed label without a runtime
/// localization context.
///
/// The fallback is derived from the generated label id. Prefer
/// [`FluentLabel::localize_label`] for user-facing UI that should follow the
/// active locale.
pub fn fallback_label<T: FluentLabel + ?Sized>() -> String {
    humanize_fluent_entry_id(T::fluent_label_id())
}

/// Converts a validated static Fluent entry id into readable fallback text.
///
/// This strips a trailing `_label`, splits on `_` and `-`, drops empty
/// segments, and uppercases the first character of each remaining segment.
pub fn humanize_fluent_entry_id(id: StaticFluentEntryId) -> String {
    humanize_fluent_entry_key(id.as_str())
}

fn humanize_fluent_entry_key(id: &str) -> String {
    let id = id.strip_suffix("_label").unwrap_or(id);
    id.split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
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
    fn label_trait_exposes_typed_metadata_and_localizes_with_fallback() {
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
    fn localize_label_helpers_return_localized_value_or_id_fallback() {
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
        assert_eq!(
            localize_label(
                &localizer,
                static_domain("label-domain"),
                static_entry("missing-id")
            ),
            "missing-id"
        );
    }

    #[test]
    fn fallback_label_helpers_humanize_typed_label_ids_without_a_localizer() {
        assert_eq!(TestLabel::fallback_label(), "Label Id");
        assert_eq!(fallback_label::<TestLabel>(), "Label Id");
        assert_eq!(
            humanize_fluent_entry_id(static_entry("sales-order_status_label")),
            "Sales Order Status"
        );
    }
}

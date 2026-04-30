use super::*;
use crate::FluentValue;
use std::collections::HashMap;

struct LabelLocalizer;

impl FluentLocalizer for LabelLocalizer {
    fn localize<'a>(
        &self,
        id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        (id == "label-id").then(|| "Label".to_string())
    }

    fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        (domain == "label-domain")
            .then(|| self.localize(id, args))
            .flatten()
    }
}

struct TestLabel;

impl FluentLabel for TestLabel {
    fn localize_label<L: FluentLocalizer + ?Sized>(localizer: &L) -> String {
        localize_label(localizer, "label-domain", "label-id")
    }
}

#[test]
fn localize_label_returns_localized_value_or_id_fallback() {
    let localizer = LabelLocalizer;

    assert_eq!(
        localize_label(&localizer, "label-domain", "label-id"),
        "Label"
    );
    assert_eq!(
        localize_label(&localizer, "label-domain", "missing-id"),
        "missing-id"
    );
    assert_eq!(TestLabel::localize_label(&localizer), "Label");
}

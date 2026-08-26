mod fluent_choice;
mod fluent_message;
mod label;

use crate::registry::StaticFluentMessageKey;

fn missing_fluent_value(key: StaticFluentMessageKey, kind: &str) -> String {
    if let Some(fallback) = key.fallback() {
        return fallback.to_string();
    }

    panic!(
        "missing Fluent {} `{}` in domain `{}` owned by `{}`",
        kind,
        key.id().as_str(),
        key.domain().as_str(),
        key.owner().as_str(),
    )
}

pub use fluent_choice::EsFluentChoice;
pub use fluent_message::{
    FluentArgs, FluentArgumentValue, FluentBorrowedArgumentValue, FluentLocalizer,
    FluentLocalizerExt, FluentLocalizerLookup, FluentMessage, FluentMessageLookup,
    FluentOptionalArgumentValue, IntoFluentArgumentValue, IntoFluentValue,
};
pub use label::{FluentLabel, localize_label};

#[cfg(test)]
mod tests {
    use super::missing_fluent_value;
    use crate::registry::{StaticFluentDomain, StaticFluentEntryId, StaticFluentMessageKey};

    fn key(fallback: Option<&'static str>) -> StaticFluentMessageKey {
        let owner = StaticFluentDomain::try_new("test-app").expect("owner");
        let id = StaticFluentEntryId::try_new("missing_value").expect("id");
        match fallback {
            Some(fallback) => StaticFluentMessageKey::with_fallback(owner, owner, id, fallback),
            None => StaticFluentMessageKey::new(owner, owner, id),
        }
    }

    #[test]
    fn missing_value_behavior_is_carried_by_each_key() {
        assert_eq!(
            missing_fluent_value(key(Some("missing_value")), "message"),
            "missing_value"
        );
        assert!(std::panic::catch_unwind(|| missing_fluent_value(key(None), "message")).is_err());
    }
}

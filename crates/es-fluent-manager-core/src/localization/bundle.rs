use es_fluent_shared::EsFluentError;
use es_fluent_shared::registry::StaticFluentEntryId;
use fluent_bundle::{FluentError, FluentResource, bundle::FluentBundle, memoizer::MemoizerKind};
use std::borrow::Borrow;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

use super::FluentArgumentMap;

pub type LocalizationError = EsFluentError;
pub type SyncFluentBundle =
    FluentBundle<Arc<FluentResource>, intl_memoizer::concurrent::IntlLangMemoizer>;

/// Adds resources to a bundle and returns all resource-add errors.
pub fn add_resources_to_bundle<R, M>(
    bundle: &mut FluentBundle<R, M>,
    resources: impl IntoIterator<Item = R>,
) -> Vec<Vec<FluentError>>
where
    R: Borrow<FluentResource>,
    M: MemoizerKind,
{
    let mut add_errors = Vec::new();
    for resource in resources {
        if let Err(errors) = bundle.add_resource(resource) {
            add_errors.push(errors);
        }
    }
    add_errors
}

/// Builds a concurrent `FluentBundle` from a locale and resources.
///
/// Fluent takes plural rules from a bundle's first locale and does not read
/// the others, so the bundle lists only `lang`. Message fallback across
/// locales chains separate bundles; see [`localize_with_bundles`].
pub fn build_sync_bundle(
    lang: &LanguageIdentifier,
    resources: impl IntoIterator<Item = Arc<FluentResource>>,
) -> (SyncFluentBundle, Vec<Vec<FluentError>>) {
    let mut bundle = FluentBundle::new_concurrent(vec![lang.clone()]);
    let add_errors = add_resources_to_bundle(&mut bundle, resources);
    (bundle, add_errors)
}

/// Localizes a message from an already-built Fluent bundle.
///
/// Returns `None` when the message or value is missing.
/// Returns the formatted value and collected formatting errors otherwise.
fn localize_with_bundle<'a, R, M>(
    bundle: &FluentBundle<R, M>,
    id: StaticFluentEntryId,
    args: Option<&FluentArgumentMap<'a>>,
) -> Option<(String, Vec<FluentError>)>
where
    R: Borrow<FluentResource>,
    M: MemoizerKind,
{
    let message = bundle.get_message(id.as_str())?;
    let pattern = message.value()?;
    let mut errors = Vec::new();
    let value = bundle.format_pattern(pattern, args, &mut errors);
    Some((value.into_owned(), errors))
}

/// Localizes a message from the first bundle that defines a value for it.
///
/// Pass bundles in fallback order. Returns `None` when no bundle has the
/// message value, and the formatted value with any formatting errors otherwise.
pub fn localize_with_bundles<'b, 'a, R, M>(
    bundles: impl IntoIterator<Item = &'b FluentBundle<R, M>>,
    id: StaticFluentEntryId,
    args: Option<&FluentArgumentMap<'a>>,
) -> Option<(String, Vec<FluentError>)>
where
    R: Borrow<FluentResource> + 'b,
    M: MemoizerKind + 'b,
{
    bundles
        .into_iter()
        .find_map(|bundle| localize_with_bundle(bundle, id, args))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_bundle::FluentValue;
    use unic_langid::langid;

    fn static_entry(value: &'static str) -> StaticFluentEntryId {
        crate::__macro::static_entry_id(value)
    }

    fn resource(source: &str) -> Arc<FluentResource> {
        Arc::new(FluentResource::try_new(source.to_string()).expect("test FTL should parse"))
    }

    #[test]
    fn bundle_helpers_localize_values_and_report_errors() {
        let lang = langid!("en-US");
        let first = resource(
            "hello = Hello { $name }\nneeds-name = Missing { $missing }\nattr-only =\n    .label = Label",
        );
        let duplicate = resource("hello = Duplicate");
        let mut bundle = FluentBundle::new_concurrent(vec![lang.clone()]);

        let add_errors = add_resources_to_bundle(&mut bundle, vec![first, duplicate]);
        assert!(!add_errors.is_empty());

        let mut args = FluentArgumentMap::default();
        args.set("name", FluentValue::from("Mark"));

        let (value, errors) = localize_with_bundle(&bundle, static_entry("hello"), Some(&args))
            .expect("message should be present");
        assert!(value.contains("Hello"));
        assert!(value.contains("Mark"));
        assert!(errors.is_empty());

        let (_value, errors) = localize_with_bundle(&bundle, static_entry("needs-name"), None)
            .expect("message should be present");
        assert!(!errors.is_empty());
        assert!(localize_with_bundle(&bundle, static_entry("missing"), None).is_none());
        assert!(localize_with_bundle(&bundle, static_entry("attr-only"), None).is_none());

        let (sync_bundle, sync_errors) = build_sync_bundle(&lang, vec![resource("sync = Sync")]);
        assert!(sync_errors.is_empty());
        assert_eq!(sync_bundle.locales, vec![langid!("en-US")]);
    }

    #[test]
    fn bundle_chain_localization_uses_the_first_bundle_defining_the_message() {
        let mut args = FluentArgumentMap::default();
        args.set("name", FluentValue::from("Mark"));
        let (en_us, _) = build_sync_bundle(
            &langid!("en-US"),
            vec![resource(
                "hello = Howdy { $name }\nattr-only =\n    .label = Label",
            )],
        );
        let (en, _) = build_sync_bundle(
            &langid!("en"),
            vec![resource(
                "hello = Hello\nfallback-only = Fallback\nattr-only = Value",
            )],
        );
        let chain = [en_us, en];

        let (value, errors) =
            localize_with_bundles(chain.iter(), static_entry("hello"), Some(&args))
                .expect("localized value should be present");
        assert!(value.contains("Howdy"));
        assert!(value.contains("Mark"));
        assert!(errors.is_empty());

        assert_eq!(
            localize_with_bundles(chain.iter(), static_entry("fallback-only"), None)
                .map(|(value, _)| value),
            Some("Fallback".to_string())
        );
        assert_eq!(
            localize_with_bundles(chain.iter(), static_entry("attr-only"), None)
                .map(|(value, _)| value),
            Some("Value".to_string()),
            "a message without a value falls through to the next bundle"
        );
        assert!(localize_with_bundles(chain.iter(), static_entry("missing"), None).is_none());
        assert!(
            localize_with_bundles(
                std::iter::empty::<&SyncFluentBundle>(),
                static_entry("hello"),
                None
            )
            .is_none()
        );
    }
}

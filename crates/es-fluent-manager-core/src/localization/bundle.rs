use es_fluent_shared::EsFluentError;
use fluent_bundle::{
    FluentArgs, FluentError, FluentResource, FluentValue, bundle::FluentBundle,
    memoizer::MemoizerKind,
};
use fluent_fallback::{
    Localization, LocalizationError as FallbackLocalizationError,
    env::LocalesProvider,
    generator::{BundleGenerator, FluentBundleResult},
};
use futures::stream::{self, Empty};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

pub type LocalizationError = EsFluentError;
pub type SyncFluentBundle =
    FluentBundle<Arc<FluentResource>, intl_memoizer::concurrent::IntlLangMemoizer>;
type FallbackFluentBundle = fluent_bundle::FluentBundle<Arc<FluentResource>>;

#[derive(Clone)]
struct OrderedLocales(Vec<LanguageIdentifier>);

impl LocalesProvider for OrderedLocales {
    type Iter = std::vec::IntoIter<LanguageIdentifier>;

    fn locales(&self) -> Self::Iter {
        self.0.clone().into_iter()
    }
}

#[derive(Clone, Default)]
struct StaticBundleGenerator {
    resources_by_locale: HashMap<LanguageIdentifier, Vec<Arc<FluentResource>>>,
}

impl StaticBundleGenerator {
    fn new(locale_resources: &[(LanguageIdentifier, Vec<Arc<FluentResource>>)]) -> Self {
        Self {
            resources_by_locale: locale_resources.iter().cloned().collect(),
        }
    }

    fn build_bundle(
        &self,
        locale: &LanguageIdentifier,
    ) -> Option<FluentBundleResult<Arc<FluentResource>>> {
        let resources = self.resources_by_locale.get(locale)?.clone();
        let mut bundle = FallbackFluentBundle::new(crate::fallback::locale_candidates(locale));
        let mut errors = Vec::new();

        for resource in resources {
            if let Err(bundle_errors) = bundle.add_resource(resource) {
                errors.extend(bundle_errors);
            }
        }

        Some(if errors.is_empty() {
            Ok(bundle)
        } else {
            Err((bundle, errors))
        })
    }
}

impl BundleGenerator for StaticBundleGenerator {
    type Resource = Arc<FluentResource>;
    type LocalesIter = std::vec::IntoIter<LanguageIdentifier>;
    type Iter = std::vec::IntoIter<FluentBundleResult<Self::Resource>>;
    type Stream = Empty<FluentBundleResult<Self::Resource>>;

    fn bundles_iter(
        &self,
        locales: Self::LocalesIter,
        _res_ids: rustc_hash::FxHashSet<fluent_fallback::types::ResourceId>,
    ) -> Self::Iter {
        locales
            .filter_map(|locale| self.build_bundle(&locale))
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn bundles_stream(
        &self,
        _locales: Self::LocalesIter,
        _res_ids: rustc_hash::FxHashSet<fluent_fallback::types::ResourceId>,
    ) -> Self::Stream {
        stream::empty()
    }
}

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
pub fn build_sync_bundle(
    lang: &LanguageIdentifier,
    resources: impl IntoIterator<Item = Arc<FluentResource>>,
) -> (SyncFluentBundle, Vec<Vec<FluentError>>) {
    let mut bundle = FluentBundle::new_concurrent(crate::fallback::locale_candidates(lang));
    let add_errors = add_resources_to_bundle(&mut bundle, resources);
    (bundle, add_errors)
}

/// Converts hash-map arguments into `FluentArgs`.
pub fn build_fluent_args<'a>(
    args: Option<&HashMap<&str, FluentValue<'a>>>,
) -> Option<FluentArgs<'a>> {
    args.map(|args| {
        let mut fluent_args = FluentArgs::new();
        for (key, value) in args {
            fluent_args.set((*key).to_string(), value.clone());
        }
        fluent_args
    })
}

/// Localizes a message from an already-built Fluent bundle.
///
/// Returns `None` when the message or value is missing.
/// Returns the formatted value and collected formatting errors otherwise.
pub fn localize_with_bundle<'a, R, M>(
    bundle: &FluentBundle<R, M>,
    id: &str,
    args: Option<&HashMap<&str, FluentValue<'a>>>,
) -> Option<(String, Vec<FluentError>)>
where
    R: Borrow<FluentResource>,
    M: MemoizerKind,
{
    let message = bundle.get_message(id)?;
    let pattern = message.value()?;
    let fluent_args = build_fluent_args(args);
    let mut errors = Vec::new();
    let value = bundle.format_pattern(pattern, fluent_args.as_ref(), &mut errors);
    Some((value.into_owned(), errors))
}

#[doc(hidden)]
pub fn localize_with_fallback_resources<'a>(
    locale_resources: &[(LanguageIdentifier, Vec<Arc<FluentResource>>)],
    id: &str,
    args: Option<&HashMap<&str, FluentValue<'a>>>,
) -> (Option<String>, Vec<FallbackLocalizationError>) {
    if locale_resources.is_empty() {
        return (None, Vec::new());
    }

    let provider = OrderedLocales(
        locale_resources
            .iter()
            .map(|(locale, _)| locale.clone())
            .collect(),
    );
    let generator = StaticBundleGenerator::new(locale_resources);
    let localization = Localization::with_env(
        Vec::<fluent_fallback::types::ResourceId>::new(),
        true,
        provider,
        generator,
    );
    let fluent_args = build_fluent_args(args);
    let mut errors = Vec::new();

    let value =
        match localization
            .bundles()
            .format_value_sync(id, fluent_args.as_ref(), &mut errors)
        {
            Ok(value) => value.map(|value| value.into_owned()),
            Err(error) => {
                errors.push(error);
                None
            },
        };

    (value, errors)
}

#[doc(hidden)]
pub fn fallback_errors_are_fatal(errors: &[FallbackLocalizationError]) -> bool {
    errors.iter().any(|error| {
        matches!(
            error,
            FallbackLocalizationError::Bundle { .. } | FallbackLocalizationError::Resolver { .. }
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_fallback::{env::LocalesProvider, generator::BundleGenerator};
    use rustc_hash::FxHashSet;
    use unic_langid::langid;

    fn resource(source: &str) -> Arc<FluentResource> {
        Arc::new(FluentResource::try_new(source.to_string()).expect("test FTL should parse"))
    }

    fn empty_resource_ids() -> FxHashSet<fluent_fallback::types::ResourceId> {
        FxHashSet::default()
    }

    #[test]
    fn ordered_locales_returns_a_fresh_iterator_each_time() {
        let locales = OrderedLocales(vec![langid!("fr-CA"), langid!("fr"), langid!("en")]);

        assert_eq!(
            locales.locales().collect::<Vec<_>>(),
            vec![langid!("fr-CA"), langid!("fr"), langid!("en")]
        );
        assert_eq!(
            locales.locales().collect::<Vec<_>>(),
            vec![langid!("fr-CA"), langid!("fr"), langid!("en")]
        );
    }

    #[test]
    fn static_bundle_generator_builds_success_error_and_iter_results() {
        let en = langid!("en");
        let fr = langid!("fr");
        let generator = StaticBundleGenerator::new(&[
            (en.clone(), vec![resource("hello = Hello")]),
            (
                fr.clone(),
                vec![resource("dupe = first"), resource("dupe = second")],
            ),
        ]);

        assert!(
            generator
                .build_bundle(&en)
                .expect("en should be present")
                .is_ok()
        );
        assert!(
            generator
                .build_bundle(&fr)
                .expect("fr should be present")
                .is_err()
        );
        assert!(generator.build_bundle(&langid!("de")).is_none());

        let iter_results = generator
            .bundles_iter(
                vec![langid!("de"), en.clone(), fr.clone()].into_iter(),
                empty_resource_ids(),
            )
            .collect::<Vec<_>>();
        assert_eq!(iter_results.len(), 2);
        assert!(iter_results[0].is_ok());
        assert!(iter_results[1].is_err());

        let _empty_stream = generator.bundles_stream(Vec::new().into_iter(), empty_resource_ids());
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

        let mut args = HashMap::new();
        args.insert("name", FluentValue::from("Mark"));
        assert!(build_fluent_args(None).is_none());
        assert!(build_fluent_args(Some(&args)).is_some());

        let (value, errors) =
            localize_with_bundle(&bundle, "hello", Some(&args)).expect("message should be present");
        assert!(value.contains("Hello"));
        assert!(value.contains("Mark"));
        assert!(errors.is_empty());

        let (_value, errors) =
            localize_with_bundle(&bundle, "needs-name", None).expect("message should be present");
        assert!(!errors.is_empty());
        assert!(localize_with_bundle(&bundle, "missing", None).is_none());
        assert!(localize_with_bundle(&bundle, "attr-only", None).is_none());

        let (sync_bundle, sync_errors) = build_sync_bundle(&lang, vec![resource("sync = Sync")]);
        assert!(sync_errors.is_empty());
        assert_eq!(sync_bundle.locales, vec![langid!("en-US"), langid!("en")]);
    }

    #[test]
    fn fallback_resource_localization_uses_ordered_locale_resources() {
        let mut args = HashMap::new();
        args.insert("name", FluentValue::from("Mark"));
        let locale_resources = vec![
            (langid!("en-US"), vec![resource("hello = Howdy { $name }")]),
            (langid!("en"), vec![resource("fallback-only = Fallback")]),
        ];

        let (value, errors) =
            localize_with_fallback_resources(&locale_resources, "hello", Some(&args));
        let value = value.expect("localized value should be present");
        assert!(value.contains("Howdy"));
        assert!(value.contains("Mark"));
        assert!(!fallback_errors_are_fatal(&errors));

        let (fallback, errors) =
            localize_with_fallback_resources(&locale_resources, "fallback-only", None);
        assert_eq!(fallback, Some("Fallback".to_string()));
        assert!(!fallback_errors_are_fatal(&errors));

        let (missing, errors) =
            localize_with_fallback_resources(&locale_resources, "missing", None);
        assert_eq!(missing, None);
        assert!(!fallback_errors_are_fatal(&errors));

        let (empty, errors) = localize_with_fallback_resources(&[], "hello", None);
        assert_eq!(empty, None);
        assert!(errors.is_empty());
        assert!(!fallback_errors_are_fatal(&[]));
    }
}

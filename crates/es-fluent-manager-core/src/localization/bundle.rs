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

use crate::fallback::locale_candidates;

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
        let mut bundle = FallbackFluentBundle::new(locale_candidates(locale));
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
    let mut bundle = FluentBundle::new_concurrent(locale_candidates(lang));
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

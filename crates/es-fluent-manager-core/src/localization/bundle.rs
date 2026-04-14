use es_fluent_shared::EsFluentError;
use fluent_bundle::{
    FluentArgs, FluentError, FluentResource, FluentValue, bundle::FluentBundle,
    memoizer::MemoizerKind,
};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

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
pub fn build_sync_bundle(
    lang: &LanguageIdentifier,
    resources: impl IntoIterator<Item = Arc<FluentResource>>,
) -> (SyncFluentBundle, Vec<Vec<FluentError>>) {
    let mut bundle = FluentBundle::new_concurrent(vec![lang.clone()]);
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

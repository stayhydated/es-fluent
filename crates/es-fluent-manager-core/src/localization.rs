//! This module provides the core types for managing translations.

use crate::asset_localization::{
    I18nModuleDescriptor, ModuleData, StaticModuleDescriptor, validate_module_registry,
};
use es_fluent_derive_core::EsFluentError;
use fluent_bundle::{
    FluentArgs, FluentError, FluentResource, FluentValue, bundle::FluentBundle,
    memoizer::MemoizerKind,
};
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
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

pub trait Localizer: Send + Sync {
    /// Selects a language for the localizer.
    fn select_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> es_fluent_derive_core::EsFluentResult<()>;
    /// Localizes a message by its ID.
    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String>;
}

/// Unified inventory contract for all module registrations.
///
/// Backends that only provide metadata (for example Bevy asset-driven loading)
/// can return `None` from `create_localizer`.
pub trait I18nModuleRegistration: I18nModuleDescriptor {
    /// Creates a localizer when the registration supports runtime localization.
    fn create_localizer(&self) -> Option<Box<dyn Localizer>> {
        None
    }
}

pub trait I18nModule: I18nModuleDescriptor {
    /// Creates a localizer for the module.
    fn create_localizer(&self) -> Box<dyn Localizer>;
}

impl<T: I18nModule> I18nModuleRegistration for T {
    fn create_localizer(&self) -> Option<Box<dyn Localizer>> {
        Some(I18nModule::create_localizer(self))
    }
}

impl I18nModuleRegistration for StaticModuleDescriptor {}

inventory::collect!(&'static dyn I18nModuleRegistration);

/// A manager for Fluent translations.
#[derive(Default)]
pub struct FluentManager {
    localizers: Vec<(&'static ModuleData, Box<dyn Localizer>)>,
}

impl FluentManager {
    /// Creates a new `FluentManager` with discovered i18n modules.
    pub fn new_with_discovered_modules() -> Self {
        let discovered_modules: Vec<_> =
            inventory::iter::<&'static dyn I18nModuleRegistration>().collect();
        let discovered_data = discovered_modules
            .iter()
            .map(|module| module.data())
            .collect::<Vec<_>>();

        if let Err(errors) = validate_module_registry(discovered_data.iter().copied()) {
            for error in errors {
                tracing::error!("Invalid i18n module registry entry: {}", error);
            }
        }

        let mut manager = Self::default();
        let mut seen_module_names = HashSet::new();
        let mut seen_domains = HashSet::new();

        for module in discovered_modules {
            let data = module.data();
            if data.name.trim().is_empty() || data.domain.trim().is_empty() {
                tracing::warn!(
                    "Skipping i18n module with invalid metadata: name='{}', domain='{}'",
                    data.name,
                    data.domain
                );
                continue;
            }
            if !seen_module_names.insert(data.name) {
                tracing::warn!(
                    "Skipping duplicate i18n module name '{}' (domain '{}')",
                    data.name,
                    data.domain
                );
                continue;
            }
            if !seen_domains.insert(data.domain) {
                tracing::warn!(
                    "Skipping duplicate i18n domain '{}' from module '{}'",
                    data.domain,
                    data.name
                );
                continue;
            }
            tracing::info!("Discovered and loading i18n module: {}", data.name);
            if let Some(localizer) = module.create_localizer() {
                manager.localizers.push((data, localizer));
            } else {
                tracing::debug!(
                    "Skipping metadata-only i18n module '{}' for FluentManager runtime localization",
                    data.name
                );
            }
        }
        manager
    }

    /// Selects a language for all localizers.
    pub fn select_language(&self, lang: &LanguageIdentifier) {
        let mut any_selected = false;

        for (data, localizer) in &self.localizers {
            match localizer.select_language(lang) {
                Ok(()) => {
                    any_selected = true;
                },
                Err(e) => {
                    tracing::debug!(
                        "Module '{}' failed to set language '{}': {}",
                        data.name,
                        lang,
                        e
                    );
                },
            }
        }

        if !any_selected {
            tracing::warn!("No i18n modules support language '{}'", lang);
        }
    }

    /// Localizes a message by its ID.
    pub fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        for (_, localizer) in &self.localizers {
            if let Some(message) = localizer.localize(id, args) {
                return Some(message);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_bundle::FluentResource;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use unic_langid::langid;

    static SELECT_OK_CALLS: AtomicUsize = AtomicUsize::new(0);
    static SELECT_ERR_CALLS: AtomicUsize = AtomicUsize::new(0);
    static MODULE_OK_DATA: ModuleData = ModuleData {
        name: "module-ok",
        domain: "module-ok",
        supported_languages: &[],
        namespaces: &[],
    };
    static MODULE_ERR_DATA: ModuleData = ModuleData {
        name: "module-err",
        domain: "module-err",
        supported_languages: &[],
        namespaces: &[],
    };

    struct ModuleOk;
    struct ModuleErr;

    struct LocalizerOk;
    struct LocalizerErr;

    impl Localizer for LocalizerOk {
        fn select_language(&self, _lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
            SELECT_OK_CALLS.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        fn localize<'a>(
            &self,
            id: &str,
            _args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            match id {
                "from-ok" => Some("ok-value".to_string()),
                _ => None,
            }
        }
    }

    impl Localizer for LocalizerErr {
        fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
            SELECT_ERR_CALLS.fetch_add(1, Ordering::Relaxed);
            Err(LocalizationError::LanguageNotSupported(lang.clone()))
        }

        fn localize<'a>(
            &self,
            id: &str,
            _args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            if id == "from-err" {
                Some("err-value".to_string())
            } else {
                None
            }
        }
    }

    impl I18nModuleDescriptor for ModuleOk {
        fn data(&self) -> &'static ModuleData {
            &MODULE_OK_DATA
        }
    }

    impl I18nModule for ModuleOk {
        fn create_localizer(&self) -> Box<dyn Localizer> {
            Box::new(LocalizerOk)
        }
    }

    impl I18nModuleDescriptor for ModuleErr {
        fn data(&self) -> &'static ModuleData {
            &MODULE_ERR_DATA
        }
    }

    impl I18nModule for ModuleErr {
        fn create_localizer(&self) -> Box<dyn Localizer> {
            Box::new(LocalizerErr)
        }
    }

    static MODULE_OK: ModuleOk = ModuleOk;
    static MODULE_ERR: ModuleErr = ModuleErr;

    inventory::submit! {
        &MODULE_OK as &dyn I18nModuleRegistration
    }

    inventory::submit! {
        &MODULE_ERR as &dyn I18nModuleRegistration
    }

    #[test]
    fn manager_select_language_calls_all_localizers() {
        SELECT_OK_CALLS.store(0, Ordering::Relaxed);
        SELECT_ERR_CALLS.store(0, Ordering::Relaxed);

        let manager = FluentManager::new_with_discovered_modules();
        manager.select_language(&langid!("en-US"));

        assert!(SELECT_OK_CALLS.load(Ordering::Relaxed) >= 1);
        assert!(SELECT_ERR_CALLS.load(Ordering::Relaxed) >= 1);
    }

    #[test]
    fn manager_localize_returns_first_matching_message() {
        let manager = FluentManager::new_with_discovered_modules();
        assert_eq!(
            manager.localize("from-ok", None),
            Some("ok-value".to_string())
        );
        assert_eq!(
            manager.localize("from-err", None),
            Some("err-value".to_string())
        );
        assert_eq!(manager.localize("missing", None), None);
    }

    #[test]
    fn manager_select_language_with_only_failing_localizers_covers_warn_path() {
        SELECT_ERR_CALLS.store(0, Ordering::Relaxed);

        let manager = FluentManager {
            localizers: vec![(&MODULE_ERR_DATA, Box::new(LocalizerErr))],
        };
        manager.select_language(&langid!("en-US"));

        assert_eq!(SELECT_ERR_CALLS.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn build_sync_bundle_reports_resource_add_errors() {
        let lang = langid!("en-US");
        let first =
            Arc::new(FluentResource::try_new("hello = first".to_string()).expect("valid ftl"));
        let duplicate =
            Arc::new(FluentResource::try_new("hello = second".to_string()).expect("valid ftl"));

        let (bundle, add_errors) = build_sync_bundle(&lang, vec![first, duplicate]);
        assert!(!add_errors.is_empty());

        let (localized, _format_errors) =
            localize_with_bundle(&bundle, "hello", None).expect("message should exist");
        assert_eq!(localized, "first");
    }
}

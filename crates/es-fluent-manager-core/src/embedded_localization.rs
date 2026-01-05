//! This module provides types for managing embedded translations.

use crate::localization::{I18nModule, LocalizationError, Localizer};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use rust_embed::RustEmbed;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use unic_langid::LanguageIdentifier;

pub trait EmbeddedAssets: RustEmbed + Send + Sync + 'static {
    fn domain() -> &'static str;
}

#[derive(Debug)]
pub struct EmbeddedModuleData {
    /// The name of the module.
    pub name: &'static str,
    /// The domain of the module.
    pub domain: &'static str,
    /// The supported languages of the module.
    pub supported_languages: &'static [LanguageIdentifier],
}

#[derive(Debug)]
pub struct EmbeddedLocalizer<T: EmbeddedAssets> {
    data: &'static EmbeddedModuleData,
    current_resource: RwLock<Option<Arc<FluentResource>>>,
    current_lang: RwLock<Option<LanguageIdentifier>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: EmbeddedAssets> EmbeddedLocalizer<T> {
    pub fn new(data: &'static EmbeddedModuleData) -> Self {
        Self {
            data,
            current_resource: RwLock::new(None),
            current_lang: RwLock::new(None),
            _phantom: std::marker::PhantomData,
        }
    }

    fn load_resource_for_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> Result<FluentResource, LocalizationError> {
        let file_name = format!("{}.ftl", self.data.domain);
        let file_path = format!("{}/{}", lang, file_name);

        if let Some(file_data) = T::get(&file_path) {
            let content = String::from_utf8(file_data.data.to_vec()).map_err(|e| {
                LocalizationError::BackendError(anyhow::anyhow!(
                    "Invalid UTF-8 in embedded file '{}': {}",
                    file_path,
                    e
                ))
            })?;

            FluentResource::try_new(content).map_err(|(_, errs)| {
                LocalizationError::BackendError(anyhow::anyhow!(
                    "Failed to parse fluent resource from '{}': {:?}",
                    file_path,
                    errs
                ))
            })
        } else {
            Err(LocalizationError::LanguageNotSupported(lang.clone()))
        }
    }
}

impl<T: EmbeddedAssets> Localizer for EmbeddedLocalizer<T> {
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        let mut current_lang_guard = self.current_lang.write().unwrap();
        if Some(lang) == current_lang_guard.as_ref() {
            return Ok(());
        }

        if let Ok(resource) = self.load_resource_for_language(lang) {
            *self.current_resource.write().unwrap() = Some(Arc::new(resource));
            *current_lang_guard = Some(lang.clone());
            return Ok(());
        }

        for supported_lang in self.data.supported_languages {
            if lang.matches(supported_lang, true, true)
                && let Ok(resource) = self.load_resource_for_language(supported_lang)
            {
                *self.current_resource.write().unwrap() = Some(Arc::new(resource));
                *current_lang_guard = Some(lang.clone());
                return Ok(());
            }
        }

        Err(LocalizationError::LanguageNotSupported(lang.clone()))
    }

    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let resource_arc = self.current_resource.read().unwrap();
        let resource = resource_arc.as_ref()?;

        let lang_guard = self.current_lang.read().unwrap();
        let lang = lang_guard
            .as_ref()
            .expect("Language not selected before localization");

        let mut bundle = FluentBundle::new(vec![lang.clone()]);
        bundle
            .add_resource(resource.clone())
            .expect("Failed to add resource");

        let message = bundle.get_message(id)?;
        let pattern = message.value()?;

        let fluent_args = args.map(|args| {
            let mut fa = FluentArgs::new();
            for (key, value) in args {
                fa.set(*key, value.clone());
            }
            fa
        });

        let mut errors = Vec::new();
        let value = bundle.format_pattern(pattern, fluent_args.as_ref(), &mut errors);

        if !errors.is_empty() {
            tracing::error!("Fluent formatting errors for id '{}': {:?}", id, errors);
            return None;
        }

        Some(value.into_owned())
    }
}

pub struct EmbeddedI18nModule<T: EmbeddedAssets> {
    data: &'static EmbeddedModuleData,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: EmbeddedAssets> EmbeddedI18nModule<T> {
    pub const fn new(data: &'static EmbeddedModuleData) -> Self {
        Self {
            data,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn discover_languages() -> Vec<LanguageIdentifier> {
        let domain = T::domain();
        let file_name = format!("{}.ftl", domain);
        let mut languages = Vec::new();

        for file_path in T::iter() {
            if file_path.ends_with(&file_name)
                && let Some(lang_part) = file_path.strip_suffix(&format!("/{}", file_name))
                && let Ok(lang_id) = lang_part.parse::<LanguageIdentifier>()
            {
                languages.push(lang_id);
            }
        }

        languages.sort_by_key(|a| a.to_string());
        languages
    }
}

impl<T: EmbeddedAssets> I18nModule for EmbeddedI18nModule<T> {
    fn name(&self) -> &'static str {
        self.data.name
    }

    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(EmbeddedLocalizer::<T>::new(self.data))
    }
}

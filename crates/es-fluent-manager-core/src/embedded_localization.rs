use crate::localization::{I18nModule, LocalizationError, Localizer};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use rust_embed::RustEmbed;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use unic_langid::LanguageIdentifier;

/// Trait for embedded asset collections that can be used for i18n.
/// This should be derived using `#[derive(RustEmbed)]` on a struct.
pub trait EmbeddedAssets: RustEmbed + Send + Sync + 'static {
    /// Get the domain name for this embedded asset collection.
    /// This is used to match FTL files (e.g., "my-app" matches "my-app.ftl").
    fn domain() -> &'static str;
}

/// Data for an embedded i18n module that uses rust-embed.
/// This struct is intended to be created by a macro at compile time.
#[derive(Debug)]
pub struct EmbeddedModuleData {
    /// The name of the module (typically the crate name).
    pub name: &'static str,
    /// The domain name used for asset file names (e.g., "my-app" -> "my-app.ftl").
    pub domain: &'static str,
    /// Languages discovered from the embedded files.
    pub supported_languages: &'static [LanguageIdentifier],
}

/// A `Localizer` implementation that uses rust-embed for embedded assets.
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

    /// Load the FTL resource for a given language from embedded assets.
    fn load_resource_for_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> Result<FluentResource, LocalizationError> {
        let file_name = format!("{}.ftl", self.data.domain);
        let file_path = format!("{}/{}", lang.to_string(), file_name);

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

        // Try exact match first
        if let Ok(resource) = self.load_resource_for_language(lang) {
            *self.current_resource.write().unwrap() = Some(Arc::new(resource));
            *current_lang_guard = Some(lang.clone());
            return Ok(());
        }

        // Try matching against supported languages with language matching
        for supported_lang in self.data.supported_languages {
            if lang.matches(supported_lang, true, true) {
                if let Ok(resource) = self.load_resource_for_language(supported_lang) {
                    *self.current_resource.write().unwrap() = Some(Arc::new(resource));
                    *current_lang_guard = Some(lang.clone());
                    return Ok(());
                }
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
        if let Some(resource) = resource_arc.as_ref() {
            let lang_guard = self.current_lang.read().unwrap();
            let lang = lang_guard
                .as_ref()
                .expect("Language not selected before localization");

            let mut bundle = FluentBundle::new(vec![lang.clone()]);
            bundle
                .add_resource(resource.clone())
                .expect("Failed to add resource");

            if let Some(message) = bundle.get_message(id) {
                if let Some(pattern) = message.value() {
                    let mut errors = Vec::new();
                    let fluent_args = args.map(|args| {
                        let mut fa = FluentArgs::new();
                        for (key, value) in args {
                            fa.set(*key, value.clone());
                        }
                        fa
                    });
                    let value = bundle.format_pattern(pattern, fluent_args.as_ref(), &mut errors);
                    if errors.is_empty() {
                        return Some(value.into_owned());
                    } else {
                        log::error!("Fluent formatting errors for id '{}': {:?}", id, errors);
                    }
                }
            }
        }
        None
    }
}

/// An `I18nModule` implementation that uses rust-embed for embedded assets.
///
/// This struct is intended to be created by a macro at compile time and
/// registered with the inventory system for automatic discovery.
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

    /// Discover available languages from embedded files.
    /// This scans the embedded assets to find directories that contain the expected FTL file.
    pub fn discover_languages() -> Vec<LanguageIdentifier> {
        let domain = T::domain();
        let file_name = format!("{}.ftl", domain);
        let mut languages = Vec::new();

        for file_path in T::iter() {
            // Parse paths like "en/my-app.ftl", "fr/my-app.ftl", etc.
            if file_path.ends_with(&file_name) {
                if let Some(lang_part) = file_path.strip_suffix(&format!("/{}", file_name)) {
                    if let Ok(lang_id) = lang_part.parse::<LanguageIdentifier>() {
                        languages.push(lang_id);
                    }
                }
            }
        }

        languages.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
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

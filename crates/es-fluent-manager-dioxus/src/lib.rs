use es_fluent_manager_core::{
    FluentManager, I18nAssetModule, I18nModule, LocalizationError, Localizer,
};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
use unic_langid::LanguageIdentifier;

/// Data for a Dioxus i18n module with embedded content
#[derive(Debug)]
pub struct DioxusModuleData {
    /// The name of the module (typically the crate name)
    pub name: &'static str,
    /// The domain name used for asset file names (e.g., "my-app" -> "my-app.ftl")
    pub domain: &'static str,
    /// Languages and their FTL content
    pub content: &'static [(&'static str, &'static str)], // (language_code, ftl_content)
}

/// A Localizer implementation for Dioxus with embedded FTL content
#[derive(Debug)]
pub struct DioxusLocalizer {
    data: &'static DioxusModuleData,
    current_resource: RwLock<Option<Arc<FluentResource>>>,
    current_lang: RwLock<Option<LanguageIdentifier>>,
}

impl DioxusLocalizer {
    pub fn new(data: &'static DioxusModuleData) -> Self {
        Self {
            data,
            current_resource: RwLock::new(None),
            current_lang: RwLock::new(None),
        }
    }

    /// Load the FTL resource for a given language from embedded content
    fn load_resource_for_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> Result<FluentResource, LocalizationError> {
        let lang_code = lang.to_string();

        // Find the content for this language
        for &(code, content) in self.data.content {
            if code == lang_code {
                return FluentResource::try_new(content.to_string()).map_err(|(_, errs)| {
                    LocalizationError::BackendError(anyhow::anyhow!(
                        "Failed to parse fluent resource for language '{}': {:?}",
                        lang_code,
                        errs
                    ))
                });
            }
        }

        Err(LocalizationError::LanguageNotSupported(lang.clone()))
    }
}

impl Localizer for DioxusLocalizer {
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

        // Try partial matching
        for &(code, _) in self.data.content {
            if let Ok(supported_lang) = code.parse::<LanguageIdentifier>() {
                if lang.matches(&supported_lang, true, true) {
                    if let Ok(resource) = self.load_resource_for_language(&supported_lang) {
                        *self.current_resource.write().unwrap() = Some(Arc::new(resource));
                        *current_lang_guard = Some(lang.clone());
                        return Ok(());
                    }
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

/// An I18nModule implementation for Dioxus with embedded content
pub struct DioxusI18nModule {
    data: &'static DioxusModuleData,
}

impl DioxusI18nModule {
    pub const fn new(data: &'static DioxusModuleData) -> Self {
        Self { data }
    }
}

impl I18nModule for DioxusI18nModule {
    fn name(&self) -> &'static str {
        self.data.name
    }

    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(DioxusLocalizer::new(self.data))
    }
}

#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::define_dioxus_i18n_module as define_i18n_module;

/// Re-export commonly used types
pub use unic_langid::langid;

/// Configuration for the Dioxus i18n system
#[derive(Debug, Clone, PartialEq)]
pub struct DioxusI18nConfig {
    pub initial_language: LanguageIdentifier,
    /// Enable debug logging
    pub debug: bool,
}

impl Default for DioxusI18nConfig {
    fn default() -> Self {
        Self {
            initial_language: langid!("en"),
            debug: false,
        }
    }
}

/// Global state for derive macro integration
static DIOXUS_I18N_MANAGER: OnceLock<Arc<RwLock<FluentManager>>> = OnceLock::new();
static DIOXUS_I18N_CONFIG: OnceLock<DioxusI18nConfig> = OnceLock::new();

/// Initialize the global Dioxus i18n state for derive macro integration
pub fn init_global_state(config: DioxusI18nConfig) {
    let manager = FluentManager::new_with_discovered_modules();
    DIOXUS_I18N_MANAGER
        .set(Arc::new(RwLock::new(manager)))
        .map_err(|_| "Manager already initialized")
        .expect("Failed to initialize global Dioxus i18n manager");

    DIOXUS_I18N_CONFIG
        .set(config)
        .map_err(|_| "Config already initialized")
        .expect("Failed to initialize global Dioxus i18n config");

    // Set up custom localizer for derive macros
    es_fluent::set_custom_localizer(dioxus_custom_localizer);
}

/// Update the global manager
pub fn update_global_manager<F>(updater: F)
where
    F: FnOnce(&mut FluentManager),
{
    if let Some(manager_arc) = DIOXUS_I18N_MANAGER.get() {
        if let Ok(mut manager) = manager_arc.write() {
            updater(&mut manager);
        }
    }
}

/// Get a reference to the global manager
pub fn with_global_manager<F, R>(reader: F) -> Option<R>
where
    F: FnOnce(&FluentManager) -> R,
{
    if let Some(manager_arc) = DIOXUS_I18N_MANAGER.get() {
        if let Ok(manager) = manager_arc.read() {
            return Some(reader(&manager));
        }
    }
    None
}

/// Custom localizer function for derive macros
fn dioxus_custom_localizer<'a>(id: &str, args: Option<&HashMap<&str, FluentValue<'a>>>) -> String {
    let manager_arc = DIOXUS_I18N_MANAGER
        .get()
        .expect("Dioxus i18n manager not initialized. Call init_global_state() first.");

    let manager = manager_arc
        .read()
        .expect("Failed to acquire read lock on Dioxus i18n manager");

    manager.localize(id, args).unwrap_or_else(|| {
        if let Some(config) = DIOXUS_I18N_CONFIG.get() {
            if config.debug {
                log::warn!("Translation for '{}' not found", id);
            }
        }
        id.to_string()
    })
}

/// Change the current language globally
pub fn set_language(lang: LanguageIdentifier) {
    update_global_manager(|manager| {
        manager.select_language(&lang);
    });
}

/// Get the current language (placeholder - not easily available with FluentManager)
pub fn current_language() -> Option<LanguageIdentifier> {
    // With FluentManager, we don't track current language explicitly
    None
}

/// Get available languages (placeholder - would need to be discovered from modules)
pub fn available_languages() -> Vec<LanguageIdentifier> {
    // With embedded content, languages are determined at compile time
    Vec::new()
}

/// Check if a language is loaded (always true with embedded content)
pub fn is_language_loaded(_lang: &LanguageIdentifier) -> bool {
    // With embedded content, languages are always loaded
    true
}

/// Localize a message using the global manager
pub fn localize<'a>(id: &str, args: Option<&HashMap<&str, FluentValue<'a>>>) -> Option<String> {
    with_global_manager(|manager| manager.localize(id, args)).flatten()
}

/// Convenience function for simple localization
pub fn t(id: &str) -> String {
    localize(id, None).unwrap_or_else(|| id.to_string())
}

/// Load FTL content (not needed with embedded content, but kept for compatibility)
pub fn load_ftl_content(
    _lang: &LanguageIdentifier,
    _domain: &str,
    _content: String,
) -> Result<(), String> {
    // With embedded content, this is not needed
    Ok(())
}

/// Initialize the Dioxus i18n system
pub fn init(config: DioxusI18nConfig) {
    let initial_language = config.initial_language.clone();
    init_global_state(config);

    // Select the initial language
    set_language(initial_language);
}

/// Simple setup function for common use cases
pub fn setup_i18n(initial_language: LanguageIdentifier) -> DioxusI18nConfig {
    DioxusI18nConfig {
        initial_language,
        ..Default::default()
    }
}

/// Setup with debug logging enabled
pub fn setup_i18n_debug(initial_language: LanguageIdentifier) -> DioxusI18nConfig {
    DioxusI18nConfig {
        initial_language,
        debug: true,
        ..Default::default()
    }
}

/// Load FTL content from a string (for manual asset loading)
/// This is the preferred way to load assets in Dioxus - load them manually
/// using Dioxus's asset system and then call this function.
pub fn load_language_from_content(
    lang: &LanguageIdentifier,
    domain: &str,
    content: String,
) -> Result<(), String> {
    load_ftl_content(lang, domain, content)
}

/// Get asset information (simplified for embedded content)
pub fn get_asset_info() -> Vec<(LanguageIdentifier, String, String)> {
    // With embedded content, asset info is not applicable
    Vec::new()
}

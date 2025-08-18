use es_fluent_manager_core::I18nAssetModule;
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
use unic_langid::LanguageIdentifier;

#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::define_bevy_i18n_module as define_dioxus_i18n_module;

/// Re-export commonly used types
pub use unic_langid::langid;

/// Configuration for the Dioxus i18n system
#[derive(Debug, Clone, PartialEq)]
pub struct DioxusI18nConfig {
    pub initial_language: LanguageIdentifier,
    pub asset_path: String,
    /// Optional manual domains - if empty, will auto-discover via inventory
    pub domains: Vec<String>,
    /// Optional manual languages - if empty, will auto-discover via inventory
    pub supported_languages: Vec<LanguageIdentifier>,
    /// Enable debug logging
    pub debug: bool,
}

impl Default for DioxusI18nConfig {
    fn default() -> Self {
        Self {
            initial_language: langid!("en"),
            asset_path: "i18n".to_string(),
            domains: vec![],             // Auto-discover
            supported_languages: vec![], // Auto-discover
            debug: false,
        }
    }
}

/// Asset handle for a Fluent resource
#[derive(Debug, Clone)]
pub struct FluentAssetHandle {
    pub language: LanguageIdentifier,
    pub domain: String,
    pub asset_path: String,
    pub loaded_resource: Option<Arc<FluentResource>>,
}

/// The main i18n state for Dioxus applications
#[derive(Debug, Clone)]
pub struct DioxusI18nState {
    pub current_language: LanguageIdentifier,
    pub available_languages: Vec<LanguageIdentifier>,
    pub asset_handles: Vec<FluentAssetHandle>,
    pub config: DioxusI18nConfig,
}

impl DioxusI18nState {
    pub fn new(config: DioxusI18nConfig) -> Self {
        let mut available_languages = config.supported_languages.clone();
        let mut domains = config.domains.clone();

        // Auto-discover languages and domains if not manually specified
        if available_languages.is_empty() || domains.is_empty() {
            let mut discovered_languages = std::collections::HashSet::new();
            let mut discovered_domains = std::collections::HashSet::new();

            for module in inventory::iter::<&'static dyn I18nAssetModule>() {
                let data = module.data();
                discovered_domains.insert(data.domain.to_string());
                for lang in data.supported_languages {
                    discovered_languages.insert(lang.clone());
                }
                if config.debug {
                    log::info!(
                        "Discovered Dioxus i18n module: {} with domain: {}",
                        data.name,
                        data.domain
                    );
                }
            }

            if available_languages.is_empty() {
                available_languages = discovered_languages.into_iter().collect();
                available_languages.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
            }

            if domains.is_empty() {
                domains = discovered_domains.into_iter().collect();
                domains.sort();
            }
        }

        // Create asset handles for all language/domain combinations
        let mut asset_handles = Vec::new();
        for lang in &available_languages {
            for domain in &domains {
                let asset_path = format!("/{}/{}/{}.ftl", config.asset_path, lang, domain);

                asset_handles.push(FluentAssetHandle {
                    language: lang.clone(),
                    domain: domain.clone(),
                    asset_path,
                    loaded_resource: None,
                });
            }
        }

        if config.debug {
            log::info!(
                "Initialized Dioxus i18n with {} languages and {} domains",
                available_languages.len(),
                domains.len()
            );
        }

        Self {
            current_language: config.initial_language.clone(),
            available_languages,
            asset_handles,
            config,
        }
    }

    /// Check if a language is fully loaded (all domains have their resources loaded)
    pub fn is_language_loaded(&self, lang: &LanguageIdentifier) -> bool {
        let lang_handles: Vec<_> = self
            .asset_handles
            .iter()
            .filter(|handle| &handle.language == lang)
            .collect();

        !lang_handles.is_empty()
            && lang_handles
                .iter()
                .all(|handle| handle.loaded_resource.is_some())
    }

    /// Get all loaded resources for a specific language
    pub fn get_language_resources(&self, lang: &LanguageIdentifier) -> Vec<Arc<FluentResource>> {
        self.asset_handles
            .iter()
            .filter_map(|handle| {
                if &handle.language == lang {
                    handle.loaded_resource.clone()
                } else {
                    None
                }
            })
            .collect()
    }

    /// Load resource content for a specific handle
    pub fn load_resource_for_handle(
        &mut self,
        lang: &LanguageIdentifier,
        domain: &str,
        content: String,
    ) -> Result<(), String> {
        if let Some(handle) = self
            .asset_handles
            .iter_mut()
            .find(|h| &h.language == lang && h.domain == domain)
        {
            match FluentResource::try_new(content) {
                Ok(resource) => {
                    handle.loaded_resource = Some(Arc::new(resource));
                    if self.config.debug {
                        log::info!("Loaded Fluent resource for {}/{}", lang, domain);
                    }
                    Ok(())
                },
                Err((_, errors)) => {
                    let error_msg = format!(
                        "Failed to parse Fluent resource for {}/{}: {:?}",
                        lang, domain, errors
                    );
                    if self.config.debug {
                        log::error!("{}", error_msg);
                    }
                    Err(error_msg)
                },
            }
        } else {
            Err(format!("No handle found for {}/{}", lang, domain))
        }
    }

    /// Localize a message using available resources
    pub fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        if !self.is_language_loaded(&self.current_language) {
            if self.config.debug {
                log::debug!(
                    "Language {} not fully loaded for localization",
                    self.current_language
                );
            }
            return None;
        }

        let resources = self.get_language_resources(&self.current_language);
        if resources.is_empty() {
            return None;
        }

        // Create bundle and add all resources
        let mut bundle = FluentBundle::new(vec![self.current_language.clone()]);
        for resource in resources {
            if let Err(e) = bundle.add_resource(resource) {
                if self.config.debug {
                    log::error!("Failed to add resource to bundle: {:?}", e);
                }
                continue;
            }
        }

        // Format message
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
        let result = bundle.format_pattern(pattern, fluent_args.as_ref(), &mut errors);

        if !errors.is_empty() && self.config.debug {
            log::warn!("Fluent formatting errors for '{}': {:?}", id, errors);
        }

        Some(result.into_owned())
    }

    /// Change the current language
    pub fn set_language(&mut self, lang: LanguageIdentifier) {
        self.current_language = lang;
    }
}

/// Global state for derive macro integration
static DIOXUS_I18N_STATE: OnceLock<Arc<RwLock<DioxusI18nState>>> = OnceLock::new();

/// Initialize the global Dioxus i18n state for derive macro integration
pub fn init_global_state(config: DioxusI18nConfig) {
    let state = DioxusI18nState::new(config);
    DIOXUS_I18N_STATE
        .set(Arc::new(RwLock::new(state)))
        .map_err(|_| "State already initialized")
        .expect("Failed to initialize global Dioxus i18n state");

    // Set up custom localizer for derive macros
    es_fluent::set_custom_localizer(dioxus_custom_localizer);
}

/// Update the global state
pub fn update_global_state<F>(updater: F)
where
    F: FnOnce(&mut DioxusI18nState),
{
    if let Some(state_arc) = DIOXUS_I18N_STATE.get() {
        if let Ok(mut state) = state_arc.write() {
            updater(&mut state);
        }
    }
}

/// Get a reference to the global state
pub fn with_global_state<F, R>(reader: F) -> Option<R>
where
    F: FnOnce(&DioxusI18nState) -> R,
{
    if let Some(state_arc) = DIOXUS_I18N_STATE.get() {
        if let Ok(state) = state_arc.read() {
            return Some(reader(&state));
        }
    }
    None
}

/// Custom localizer function for derive macros
fn dioxus_custom_localizer<'a>(id: &str, args: Option<&HashMap<&str, FluentValue<'a>>>) -> String {
    let state_arc = DIOXUS_I18N_STATE
        .get()
        .expect("Dioxus i18n state not initialized. Call init_global_state() first.");

    let state = state_arc
        .read()
        .expect("Failed to acquire read lock on Dioxus i18n state");

    state.localize(id, args).unwrap_or_else(|| {
        if state.config.debug {
            log::warn!("Translation for '{}' not found", id);
        }
        id.to_string()
    })
}

/// Initialize the Dioxus i18n system
pub fn init(config: DioxusI18nConfig) {
    init_global_state(config);
}

/// Change the current language globally
pub fn set_language(lang: LanguageIdentifier) {
    update_global_state(|state| {
        state.set_language(lang);
    });
}

/// Get the current language
pub fn current_language() -> Option<LanguageIdentifier> {
    with_global_state(|state| state.current_language.clone())
}

/// Get available languages
pub fn available_languages() -> Vec<LanguageIdentifier> {
    with_global_state(|state| state.available_languages.clone()).unwrap_or_default()
}

/// Check if a language is loaded
pub fn is_language_loaded(lang: &LanguageIdentifier) -> bool {
    with_global_state(|state| state.is_language_loaded(lang)).unwrap_or(false)
}

/// Localize a message using the global state
pub fn localize<'a>(id: &str, args: Option<&HashMap<&str, FluentValue<'a>>>) -> Option<String> {
    with_global_state(|state| state.localize(id, args)).flatten()
}

/// Convenience function for simple localization
pub fn t(id: &str) -> String {
    localize(id, None).unwrap_or_else(|| id.to_string())
}

/// Load FTL content for a specific language and domain
pub fn load_ftl_content(
    lang: &LanguageIdentifier,
    domain: &str,
    content: String,
) -> Result<(), String> {
    update_global_state(|state| {
        if let Err(e) = state.load_resource_for_handle(lang, domain, content) {
            if state.config.debug {
                log::error!("Failed to load FTL content for {}/{}: {}", lang, domain, e);
            }
        }
    });
    Ok(())
}

/// Simple setup function for common use cases with auto-discovery
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

/// Setup with manual language and domain specification
pub fn setup_i18n_manual(
    initial_language: LanguageIdentifier,
    supported_languages: Vec<LanguageIdentifier>,
    domains: Vec<String>,
) -> DioxusI18nConfig {
    DioxusI18nConfig {
        initial_language,
        supported_languages,
        domains,
        ..Default::default()
    }
}

/// Get asset information for implementing custom asset loading
pub fn get_asset_info() -> Vec<(LanguageIdentifier, String, String)> {
    with_global_state(|state| {
        state
            .asset_handles
            .iter()
            .map(|handle| {
                (
                    handle.language.clone(),
                    handle.domain.clone(),
                    handle.asset_path.clone(),
                )
            })
            .collect()
    })
    .unwrap_or_default()
}

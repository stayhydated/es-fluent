#![doc = include_str!("../README.md")]

#[cfg(feature = "derive")]
pub use es_fluent_derive::{EsFluent, EsFluentChoice, EsFluentKv, EsFluentThis};

#[doc(hidden)]
pub use es_fluent_manager_core::{FluentManager, I18nModule, LocalizationError, Localizer};

#[doc(hidden)]
pub use fluent_bundle::FluentValue;

#[doc(hidden)]
pub use inventory as __inventory;

#[doc(hidden)]
pub use rust_embed as __rust_embed;

#[doc(hidden)]
pub use es_fluent_manager_core as __manager_core;

#[doc(hidden)]
pub use es_fluent_core as __core;

#[doc(hidden)]
pub use unic_langid;

mod traits;
pub use traits::{EsFluentChoice, FluentDisplay, ThisFtl, ToFluentString};

use std::sync::{Arc, OnceLock, RwLock};

#[doc(hidden)]
static CONTEXT: OnceLock<Arc<RwLock<FluentManager>>> = OnceLock::new();

#[doc(hidden)]
static CUSTOM_LOCALIZER: OnceLock<
    Box<
        dyn Fn(&str, Option<&std::collections::HashMap<&str, FluentValue>>) -> Option<String>
            + Send
            + Sync,
    >,
> = OnceLock::new();

/// Sets the global `FluentManager` context.
///
/// This function should be called once at the beginning of your application's
/// lifecycle.
///
/// # Panics
///
/// This function will panic if the context has already been set.
#[doc(hidden)]
pub fn set_context(manager: FluentManager) {
    CONTEXT
        .set(Arc::new(RwLock::new(manager)))
        .map_err(|_| "Context already set")
        .expect("Failed to set context");
}

/// Sets the global `FluentManager` context with a shared `Arc<RwLock<FluentManager>>`.
///
/// This function is useful when you want to share the `FluentManager` between
/// multiple threads.
///
/// # Panics
///
/// This function will panic if the context has already been set.
#[doc(hidden)]
pub fn set_shared_context(manager: Arc<RwLock<FluentManager>>) {
    CONTEXT
        .set(manager)
        .map_err(|_| "Context already set")
        .expect("Failed to set shared context");
}

/// Sets a custom localizer function.
///
/// The custom localizer will be called before the global context's `localize`
/// method. If the custom localizer returns `Some(message)`, the message will be
/// returned. Otherwise, the global context will be used.
///
/// # Panics
///
/// This function will panic if the custom localizer has already been set.
#[doc(hidden)]
pub fn set_custom_localizer<F>(localizer: F)
where
    F: Fn(&str, Option<&std::collections::HashMap<&str, FluentValue>>) -> Option<String>
        + Send
        + Sync
        + 'static,
{
    CUSTOM_LOCALIZER
        .set(Box::new(localizer))
        .map_err(|_| "Custom localizer already set")
        .expect("Failed to set custom localizer");
}

/// Updates the global `FluentManager` context.
#[doc(hidden)]
pub fn update_context<F>(f: F)
where
    F: FnOnce(&mut FluentManager),
{
    if let Some(context_arc) = CONTEXT.get() {
        let mut context = context_arc
            .write()
            .expect("Failed to acquire write lock on context");
        f(&mut context);
    }
}

/// Localizes a message by its ID.
///
/// This function will first try to use the custom localizer if it has been set.
/// If the custom localizer returns `None`, it will then try to use the global
/// context.
///
/// If the message is not found, a warning will be logged and the ID will be
/// returned as the message.
#[doc(hidden)]
pub fn localize<'a>(
    id: &str,
    args: Option<&std::collections::HashMap<&str, FluentValue<'a>>>,
) -> String {
    if let Some(custom_localizer) = CUSTOM_LOCALIZER.get()
        && let Some(message) = custom_localizer(id, args)
    {
        return message;
    }

    if let Some(context_arc) = CONTEXT.get() {
        let context = context_arc
            .read()
            .expect("Failed to acquire read lock on context");

        if let Some(message) = context.localize(id, args) {
            return message;
        }
    }

    log::warn!("Translation for '{}' not found or context not set.", id);
    id.to_string()
}

// FTL file generation support (requires "generate" feature)
#[cfg(feature = "generate")]
mod generate {
    use std::path::PathBuf;

    pub use es_fluent_generate::FluentParseMode;
    pub use es_fluent_generate::error::FluentGenerateError;

    /// Error type for FTL generation.
    #[derive(Debug)]
    pub enum GeneratorError {
        /// Failed to read i18n.toml configuration.
        Config(es_fluent_toml::I18nConfigError),
        /// Failed to detect crate name.
        CrateName(String),
        /// Failed to generate FTL files.
        Generate(FluentGenerateError),
    }

    impl std::fmt::Display for GeneratorError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Config(e) => write!(f, "Configuration error: {}", e),
                Self::CrateName(e) => write!(f, "Failed to detect crate name: {}", e),
                Self::Generate(e) => write!(f, "Generation error: {}", e),
            }
        }
    }

    impl std::error::Error for GeneratorError {}

    impl From<es_fluent_toml::I18nConfigError> for GeneratorError {
        fn from(e: es_fluent_toml::I18nConfigError) -> Self {
            Self::Config(e)
        }
    }

    impl From<FluentGenerateError> for GeneratorError {
        fn from(e: FluentGenerateError) -> Self {
            Self::Generate(e)
        }
    }

    /// Builder for generating FTL files from registered types.
    ///
    /// Uses the `inventory` crate to collect all types registered via
    /// `#[derive(EsFluent)]`, `#[derive(EsFluentKv)]`, or `#[derive(EsFluentThis)]`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use es_fluent::EsFluentGenerator;
    ///
    /// fn main() {
    ///     // Uses defaults from i18n.toml and auto-detects crate name
    ///     EsFluentGenerator::builder()
    ///         .build()
    ///         .generate()
    ///         .expect("Failed to generate FTL files");
    ///
    ///     // Or with custom settings
    ///     EsFluentGenerator::builder()
    ///         .mode(es_fluent::FluentParseMode::Aggressive)
    ///         .output_path("custom/path")
    ///         .build()
    ///         .generate()
    ///         .expect("Failed to generate FTL files");
    /// }
    /// ```
    #[derive(bon::Builder)]
    pub struct EsFluentGenerator {
        /// The parse mode (Conservative preserves existing translations, Aggressive overwrites).
        /// Defaults to Conservative.
        #[builder(default)]
        mode: FluentParseMode,

        /// Override the crate name (defaults to auto-detect from Cargo.toml).
        #[builder(into)]
        crate_name: Option<String>,

        /// Override the output path (defaults to reading from i18n.toml).
        #[builder(into)]
        output_path: Option<PathBuf>,

        /// Override the crate root directory for filtering source files.
        /// If not provided, defaults to the current crate's src/ directory logic.
        #[builder(into)]
        crate_root: Option<PathBuf>,
    }

    impl EsFluentGenerator {
        /// Generates FTL files from all registered types.
        pub fn generate(&self) -> Result<(), GeneratorError> {
            let crate_name = match &self.crate_name {
                Some(name) => name.clone(),
                None => Self::detect_crate_name()?,
            };

            let output_path = match &self.output_path {
                Some(path) => path.clone(),
                None => {
                    let config = es_fluent_toml::I18nConfig::read_from_manifest_dir()?;
                    config.assets_dir.join(&config.fallback_language)
                },
            };

            let type_infos = if let Some(root) = &self.crate_root {
                // Filter by explicitly provided root
                crate::__core::registry::get_all_ftl_type_infos()
                    .into_iter()
                    .filter(|info| {
                        info.file_path
                            .as_ref()
                            .is_some_and(|path| path.starts_with(root.to_str().unwrap_or_default()))
                    })
                    .collect::<Vec<_>>()
            } else {
                // Get the current crate's src directory to filter types
                // file!() returns paths relative to workspace root, so we need to get the relative path
                let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| {
                    GeneratorError::CrateName("CARGO_MANIFEST_DIR not set".to_string())
                })?;

                // Get workspace root and compute relative path
                let src_prefix = cargo_metadata::MetadataCommand::new()
                    .exec()
                    .ok()
                    .and_then(|metadata| {
                        let workspace_root = metadata.workspace_root.as_std_path();
                        let manifest_dir_path = std::path::Path::new(&manifest_dir);
                        manifest_dir_path
                            .strip_prefix(workspace_root)
                            .ok()
                            .map(|rel| format!("{}/src/", rel.display()))
                    })
                    .unwrap_or_else(|| "src/".to_string());

                crate::__core::registry::get_all_ftl_type_infos()
                    .into_iter()
                    .filter(|info| {
                        info.file_path
                            .as_ref()
                            .is_some_and(|path| path.starts_with(&src_prefix))
                    })
                    .collect()
            };

            log::info!(
                "Generating FTL files for {} types in crate '{}'",
                type_infos.len(),
                crate_name
            );

            es_fluent_generate::generate(&crate_name, output_path, type_infos, self.mode.clone())?;

            Ok(())
        }

        /// Auto-detects the crate name from Cargo.toml using cargo_metadata.
        fn detect_crate_name() -> Result<String, GeneratorError> {
            let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
                .map_err(|_| GeneratorError::CrateName("CARGO_MANIFEST_DIR not set".to_string()))?;
            let manifest_path = std::path::PathBuf::from(&manifest_dir).join("Cargo.toml");

            cargo_metadata::MetadataCommand::new()
                .exec()
                .ok()
                .and_then(|metadata| {
                    metadata
                        .packages
                        .iter()
                        .find(|pkg| pkg.manifest_path == manifest_path)
                        .map(|pkg| pkg.name.to_string())
                })
                .or_else(|| std::env::var("CARGO_PKG_NAME").ok())
                .ok_or_else(|| {
                    GeneratorError::CrateName("Could not determine crate name".to_string())
                })
        }
    }
}

#[cfg(feature = "generate")]
pub use generate::{EsFluentGenerator, FluentParseMode, GeneratorError};

#[cfg(feature = "generate")]
pub use es_fluent_toml;

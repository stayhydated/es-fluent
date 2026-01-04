//! FTL file generation functionality.

use std::path::PathBuf;

pub use es_fluent_generate::FluentParseMode;
pub use es_fluent_generate::error::FluentGenerateError;

/// Error type for FTL generation.
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    /// Failed to read i18n.toml configuration.
    #[error("Configuration error: {0}")]
    Config(#[from] es_fluent_toml::I18nConfigError),

    /// Failed to detect crate name.
    #[error("Failed to detect crate name: {0}")]
    CrateName(String),

    /// Failed to generate FTL files.
    #[error("Generation error: {0}")]
    Generate(#[from] FluentGenerateError),
}

/// Builder for generating FTL files from registered types.
///
/// Uses the `inventory` crate to collect all types registered via
/// `#[derive(EsFluent)]`, `#[derive(EsFluentKv)]`, or `#[derive(EsFluentThis)]`.
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

    /// Override the assets directory (defaults to reading from i18n.toml).
    #[builder(into)]
    assets_dir: Option<PathBuf>,

    /// Dry run (don't write changes).
    #[builder(default)]
    dry_run: bool,
}

/// Command line arguments for the generator.
#[derive(clap::Parser)]
pub struct GeneratorArgs {
    #[command(subcommand)]
    action: Action,
}

#[derive(clap::Subcommand)]
enum Action {
    /// Generate FTL files
    Generate {
        /// Parse mode
        #[arg(long, default_value_t = FluentParseMode::default())]
        mode: FluentParseMode,
        /// Dry run (don't write changes)
        #[arg(long)]
        dry_run: bool,
    },
    /// Clean FTL files (remove orphans)
    Clean {
        /// Clean all locales
        #[arg(long)]
        all: bool,
        /// Dry run (don't write changes)
        #[arg(long)]
        dry_run: bool,
    },
}

impl EsFluentGenerator {
    /// Runs the generator based on command line arguments.
    pub fn run_cli(self) -> Result<bool, GeneratorError> {
        use clap::Parser;
        let args = GeneratorArgs::parse();

        match args.action {
            Action::Generate { mode, dry_run } => {
                let mut generator = self;
                generator.mode = mode;
                generator.dry_run = dry_run;
                generator.generate()
            },
            Action::Clean { all, dry_run } => self.clean(all, dry_run),
        }
    }

    /// Generates FTL files from all registered types.
    pub fn generate(&self) -> Result<bool, GeneratorError> {
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

        let crate_ident = crate_name.replace('-', "_");
        let type_infos = es_fluent_core::registry::get_all_ftl_type_infos()
            .into_iter()
            .filter(|info| {
                info.module_path == crate_ident
                    || info.module_path.starts_with(&format!("{}::", crate_ident))
            })
            .collect::<Vec<_>>();

        tracing::info!(
            "Generating FTL files for {} types in crate '{}'",
            type_infos.len(),
            crate_name
        );

        let changed = es_fluent_generate::generate(
            &crate_name,
            output_path,
            type_infos,
            self.mode.clone(),
            self.dry_run,
        )?;

        Ok(changed)
    }

    /// Cleans FTL files by removing orphan keys while preserving existing translations.
    pub fn clean(&self, all_locales: bool, dry_run: bool) -> Result<bool, GeneratorError> {
        let crate_name = match &self.crate_name {
            Some(name) => name.clone(),
            None => Self::detect_crate_name()?,
        };

        // Determine assets_dir
        let assets_dir = if let Some(path) = &self.assets_dir {
            path.clone()
        } else {
            es_fluent_toml::I18nConfig::read_from_manifest_dir()?.assets_dir
        };

        // Determine which paths to clean
        let paths = if all_locales {
            match std::fs::read_dir(&assets_dir) {
                Ok(entries) => entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .map(|e| e.path())
                    .collect(),
                Err(_) => {
                    // If output_path is set, fallback to it?
                    if let Some(path) = &self.output_path {
                        vec![path.clone()]
                    } else {
                        // Can't do much if assets_dir fails and no output_path
                        vec![]
                    }
                },
            }
        } else if let Some(path) = &self.output_path {
            vec![path.clone()]
        } else {
            // Fallback to reading config again? We already read it if assets_dir was missing.
            // Ideally we keep config if read.
            // For now, simple re-read is fine or minimal logic.
            let config = es_fluent_toml::I18nConfig::read_from_manifest_dir()?;
            vec![config.assets_dir.join(&config.fallback_language)]
        };

        let crate_ident = crate_name.replace('-', "_");
        let type_infos = es_fluent_core::registry::get_all_ftl_type_infos()
            .into_iter()
            .filter(|info| {
                info.module_path == crate_ident
                    || info.module_path.starts_with(&format!("{}::", crate_ident))
            })
            .collect::<Vec<_>>();

        let mut any_changed = false;
        for output_path in paths {
            if !dry_run {
                tracing::info!(
                    "Cleaning FTL files for {} types in crate '{}' at {}",
                    type_infos.len(),
                    crate_name,
                    output_path.display()
                );
            }

            if es_fluent_generate::clean::clean(
                &crate_name,
                output_path,
                type_infos.clone(),
                dry_run,
            )? {
                any_changed = true;
            }
        }

        Ok(any_changed)
    }

    /// Auto-detects the crate name from Cargo.toml using cargo_metadata.
    fn detect_crate_name() -> Result<String, GeneratorError> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map_err(|_| GeneratorError::CrateName("CARGO_MANIFEST_DIR not set".to_string()))?;
        let manifest_path = PathBuf::from(&manifest_dir).join("Cargo.toml");

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
            .ok_or_else(|| GeneratorError::CrateName("Could not determine crate name".to_string()))
    }
}

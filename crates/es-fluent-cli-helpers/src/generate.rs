//! FTL file generation functionality.

use es_fluent::registry::FtlTypeInfo;
use std::path::{Path, PathBuf};

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

    /// Invalid namespace used (not in allowed list).
    #[error(
        "Invalid namespace '{namespace}' for type '{type_name}'. Allowed namespaces: {allowed:?}"
    )]
    InvalidNamespace {
        namespace: String,
        type_name: String,
        allowed: Vec<String>,
    },
}

/// Builder for generating FTL files from registered types.
///
/// Uses the `inventory` crate to collect all types registered via
/// `#[derive(EsFluent)]`, `#[derive(EsFluentVariants)]`, or `#[derive(EsFluentThis)]`.
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

    /// Override the manifest directory for namespace resolution.
    #[builder(into)]
    manifest_dir: Option<PathBuf>,

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

    // --- Resolution helpers (DRY) ---

    /// Resolve the crate name, using override or auto-detection.
    fn resolve_crate_name(&self) -> Result<String, GeneratorError> {
        self.crate_name
            .clone()
            .map_or_else(Self::detect_crate_name, Ok)
    }

    /// Resolve the output path for the fallback locale.
    fn resolve_output_path(&self) -> Result<PathBuf, GeneratorError> {
        if let Some(path) = &self.output_path {
            return Ok(path.clone());
        }
        let manifest_dir = self.resolve_manifest_dir()?;
        Ok(es_fluent_toml::I18nConfig::output_dir_from_manifest_dir(
            &manifest_dir,
        )?)
    }

    /// Resolve the assets directory.
    fn resolve_assets_dir(&self) -> Result<PathBuf, GeneratorError> {
        if let Some(path) = &self.assets_dir {
            return Ok(path.clone());
        }
        let manifest_dir = self.resolve_manifest_dir()?;
        Ok(es_fluent_toml::I18nConfig::assets_dir_from_manifest_dir(
            &manifest_dir,
        )?)
    }

    /// Resolve the manifest directory for namespace resolution.
    fn resolve_manifest_dir(&self) -> Result<PathBuf, GeneratorError> {
        if let Some(path) = &self.manifest_dir {
            return Ok(path.clone());
        }

        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map_err(|_| GeneratorError::CrateName("CARGO_MANIFEST_DIR not set".to_string()))?;
        Ok(PathBuf::from(manifest_dir))
    }

    /// Resolve the paths to clean based on configuration.
    fn resolve_clean_paths(&self, all_locales: bool) -> Result<Vec<PathBuf>, GeneratorError> {
        if !all_locales {
            return Ok(vec![self.resolve_output_path()?]);
        }

        let assets_dir = self.resolve_assets_dir()?;
        let mut paths: Vec<PathBuf> = std::fs::read_dir(&assets_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .map(|e| e.path())
                    .collect()
            })
            .unwrap_or_else(|| self.output_path.clone().into_iter().collect());

        // Sort paths to ensure deterministic ordering across filesystems
        paths.sort();

        Ok(paths)
    }

    /// Generates FTL files from all registered types.
    pub fn generate(&self) -> Result<bool, GeneratorError> {
        let crate_name = self.resolve_crate_name()?;
        let output_path = self.resolve_output_path()?;
        let manifest_dir = self.resolve_manifest_dir()?;
        let type_infos = collect_type_infos(&crate_name);

        // Validate namespaces against allowed list if configured
        self.validate_namespaces(&type_infos, &manifest_dir)?;

        tracing::info!(
            "Generating FTL files for {} types in crate '{}'",
            type_infos.len(),
            crate_name
        );

        let changed = es_fluent_generate::generate(
            &crate_name,
            output_path,
            &manifest_dir,
            &type_infos,
            self.mode.clone(),
            self.dry_run,
        )?;

        Ok(changed)
    }

    /// Validates that all namespaces in the type infos are allowed by the config.
    fn validate_namespaces(
        &self,
        type_infos: &[&'static FtlTypeInfo],
        manifest_dir: &Path,
    ) -> Result<(), GeneratorError> {
        let config = es_fluent_toml::I18nConfig::from_manifest_dir(manifest_dir).ok();
        let allowed = config.as_ref().and_then(|c| c.namespaces.as_ref());

        if let Some(allowed_namespaces) = allowed {
            for info in type_infos {
                if let Some(ns) = info.resolved_namespace(manifest_dir)
                    && !allowed_namespaces.contains(&ns)
                {
                    return Err(GeneratorError::InvalidNamespace {
                        namespace: ns,
                        type_name: info.type_name.to_string(),
                        allowed: allowed_namespaces.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Cleans FTL files by removing orphan keys while preserving existing translations.
    pub fn clean(&self, all_locales: bool, dry_run: bool) -> Result<bool, GeneratorError> {
        let crate_name = self.resolve_crate_name()?;
        let paths = self.resolve_clean_paths(all_locales)?;
        let manifest_dir = self.resolve_manifest_dir()?;
        let type_infos = collect_type_infos(&crate_name);

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
                &manifest_dir,
                &type_infos,
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

/// Collect all registered type infos for a given crate.
fn collect_type_infos(crate_name: &str) -> Vec<&'static FtlTypeInfo> {
    let crate_ident = crate_name.replace('-', "_");
    es_fluent::registry::get_all_ftl_type_infos()
        .filter(|info| {
            info.module_path == crate_ident
                || info.module_path.starts_with(&format!("{}::", crate_ident))
        })
        .collect()
}

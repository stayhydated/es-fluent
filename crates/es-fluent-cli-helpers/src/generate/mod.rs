//! FTL file generation functionality.

mod args;
mod error;
mod inventory;

#[cfg(test)]
mod tests;

use self::args::Action;
use self::inventory::{collect_type_infos, validate_namespaces};

pub use self::args::GeneratorArgs;
pub use self::error::GeneratorError;
pub use es_fluent_generate::FluentParseMode;
use es_fluent_toml::ResolvedI18nLayout;
use std::path::{Path, PathBuf};

/// Builder for generating FTL files from registered types.
///
/// Uses the `inventory` crate to collect all types registered via
/// `#[derive(EsFluent)]`, `#[derive(EsFluentVariants)]`, or `#[derive(EsFluentLabel)]`.
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

impl EsFluentGenerator {
    /// Runs the generator based on command line arguments.
    pub fn run_cli(self) -> Result<bool, GeneratorError> {
        use clap::Parser as _;
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

    fn resolve_crate_name(&self) -> Result<String, GeneratorError> {
        self.crate_name
            .clone()
            .map_or_else(Self::detect_crate_name, Ok)
    }

    fn resolve_output_path(&self) -> Result<PathBuf, GeneratorError> {
        if let Some(path) = &self.output_path {
            return Ok(path.clone());
        }

        Ok(self.resolve_layout()?.output_dir)
    }

    #[cfg(test)]
    fn resolve_assets_dir(&self) -> Result<PathBuf, GeneratorError> {
        if let Some(path) = &self.assets_dir {
            return Ok(path.clone());
        }

        Ok(self.resolve_layout()?.assets_dir)
    }

    fn resolve_manifest_dir(&self) -> Result<PathBuf, GeneratorError> {
        if let Some(path) = &self.manifest_dir {
            return Ok(path.clone());
        }

        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map_err(|_| GeneratorError::CrateName("CARGO_MANIFEST_DIR not set".to_string()))?;
        Ok(PathBuf::from(manifest_dir))
    }

    fn resolve_layout(&self) -> Result<ResolvedI18nLayout, GeneratorError> {
        let manifest_dir = self.resolve_manifest_dir()?;
        Ok(ResolvedI18nLayout::from_manifest_dir(&manifest_dir)?)
    }

    fn resolve_clean_paths(&self, all_locales: bool) -> Result<Vec<PathBuf>, GeneratorError> {
        if !all_locales {
            return Ok(vec![self.resolve_output_path()?]);
        }

        let mut paths = if let Some(assets_dir) = &self.assets_dir {
            Self::resolve_clean_locale_dirs(assets_dir)?
        } else {
            let layout = self.resolve_layout()?;
            layout
                .available_locale_names()?
                .into_iter()
                .map(|locale| layout.locale_dir(&locale))
                .collect()
        };

        if paths.is_empty() {
            return Ok(vec![self.resolve_output_path()?]);
        }

        paths.sort();
        Ok(paths)
    }

    /// Generates FTL files from all registered types.
    pub fn generate(&self) -> Result<bool, GeneratorError> {
        let crate_name = self.resolve_crate_name()?;
        let output_path = self.resolve_output_path()?;
        let manifest_dir = self.resolve_manifest_dir()?;
        let type_infos = collect_type_infos(&crate_name);

        validate_namespaces(&type_infos, &manifest_dir)?;

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

    fn resolve_clean_locale_dirs(assets_dir: &Path) -> Result<Vec<PathBuf>, GeneratorError> {
        let config = es_fluent_toml::I18nConfig {
            fallback_language: "en".to_string(),
            assets_dir: assets_dir.to_path_buf(),
            fluent_feature: None,
            namespaces: None,
        };

        Ok(config
            .available_locale_names_from_base(Some(Path::new("")))?
            .into_iter()
            .map(|locale| assets_dir.join(locale))
            .collect())
    }

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

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

#[cfg(test)]
mod tests {
    use super::*;
    use es_fluent::registry::{FtlVariant, NamespaceRule};
    use es_fluent_derive_core::meta::TypeKind;
    use std::sync::{LazyLock, Mutex};
    use tempfile::tempdir;

    static EMPTY_VARIANTS: &[FtlVariant] = &[];
    static ALLOWED_INFO: FtlTypeInfo = FtlTypeInfo {
        type_kind: TypeKind::Struct,
        type_name: "AllowedType",
        variants: EMPTY_VARIANTS,
        file_path: "src/lib.rs",
        module_path: "test_crate",
        namespace: Some(NamespaceRule::Literal("ui")),
    };
    static DISALLOWED_INFO: FtlTypeInfo = FtlTypeInfo {
        type_kind: TypeKind::Struct,
        type_name: "DisallowedType",
        variants: EMPTY_VARIANTS,
        file_path: "src/lib.rs",
        module_path: "test_crate",
        namespace: Some(NamespaceRule::Literal("errors")),
    };
    static CLEAN_VARIANTS: &[FtlVariant] = &[FtlVariant {
        name: "Key1",
        ftl_key: "group_a-Key1",
        args: &[],
        module_path: "test",
        line: 0,
    }];
    static CLEAN_INFO: FtlTypeInfo = FtlTypeInfo {
        type_kind: TypeKind::Enum,
        type_name: "GroupA",
        variants: CLEAN_VARIANTS,
        file_path: "src/lib.rs",
        module_path: "coverage_test_crate",
        namespace: None,
    };
    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    es_fluent::__inventory::submit! {
        es_fluent::registry::RegisteredFtlType(&CLEAN_INFO)
    }

    fn with_env_var<T>(key: &str, value: Option<&str>, f: impl FnOnce() -> T) -> T {
        let _guard = ENV_LOCK.lock().expect("lock poisoned");
        let previous = std::env::var_os(key);

        match value {
            Some(value) => {
                // SAFETY: tests serialize environment updates with a global lock.
                unsafe { std::env::set_var(key, value) };
            },
            None => {
                // SAFETY: tests serialize environment updates with a global lock.
                unsafe { std::env::remove_var(key) };
            },
        }

        let result = f();

        match previous {
            Some(previous) => {
                // SAFETY: tests serialize environment updates with a global lock.
                unsafe { std::env::set_var(key, previous) };
            },
            None => {
                // SAFETY: tests serialize environment updates with a global lock.
                unsafe { std::env::remove_var(key) };
            },
        }

        result
    }

    fn with_env_vars<T>(vars: &[(&str, Option<&str>)], f: impl FnOnce() -> T) -> T {
        let _guard = ENV_LOCK.lock().expect("lock poisoned");
        let previous: Vec<(String, Option<std::ffi::OsString>)> = vars
            .iter()
            .map(|(key, _)| ((*key).to_string(), std::env::var_os(key)))
            .collect();

        for (key, value) in vars {
            match value {
                Some(value) => {
                    // SAFETY: tests serialize environment updates with a global lock.
                    unsafe { std::env::set_var(key, value) };
                },
                None => {
                    // SAFETY: tests serialize environment updates with a global lock.
                    unsafe { std::env::remove_var(key) };
                },
            }
        }

        let result = f();

        for (key, value) in previous {
            match value {
                Some(value) => {
                    // SAFETY: tests serialize environment updates with a global lock.
                    unsafe { std::env::set_var(&key, value) };
                },
                None => {
                    // SAFETY: tests serialize environment updates with a global lock.
                    unsafe { std::env::remove_var(&key) };
                },
            }
        }

        result
    }

    fn write_basic_i18n_config(manifest_dir: &Path) {
        std::fs::create_dir_all(manifest_dir.join("i18n/en-US")).expect("mkdir en-US");
        std::fs::create_dir_all(manifest_dir.join("i18n/fr")).expect("mkdir fr");
        std::fs::write(
            manifest_dir.join("i18n.toml"),
            "fallback_language = \"en-US\"\nassets_dir = \"i18n\"\nnamespaces = [\"ui\"]\n",
        )
        .expect("write i18n.toml");
    }

    #[test]
    fn resolve_helpers_use_overrides_and_config_defaults() {
        let temp = tempdir().expect("tempdir");
        write_basic_i18n_config(temp.path());

        let output_override = temp.path().join("custom-output");
        let assets_override = temp.path().join("custom-assets");
        let generator = EsFluentGenerator::builder()
            .crate_name("my-crate")
            .output_path(&output_override)
            .assets_dir(&assets_override)
            .manifest_dir(temp.path())
            .build();

        assert_eq!(
            generator.resolve_crate_name().expect("crate name"),
            "my-crate"
        );
        assert_eq!(
            generator.resolve_output_path().expect("output"),
            output_override
        );
        assert_eq!(
            generator.resolve_assets_dir().expect("assets"),
            assets_override
        );
        assert_eq!(
            generator.resolve_manifest_dir().expect("manifest"),
            temp.path()
        );
    }

    #[test]
    fn resolve_helpers_can_load_defaults_from_manifest_environment() {
        let temp = tempdir().expect("tempdir");
        write_basic_i18n_config(temp.path());

        with_env_var("CARGO_MANIFEST_DIR", temp.path().to_str(), || {
            let generator = EsFluentGenerator::builder()
                .crate_name("missing-crate")
                .build();
            assert_eq!(
                generator.resolve_output_path().expect("output path"),
                temp.path().join("i18n/en-US")
            );
            assert_eq!(
                generator.resolve_assets_dir().expect("assets path"),
                temp.path().join("i18n")
            );
            assert_eq!(
                generator.resolve_manifest_dir().expect("manifest path"),
                temp.path()
            );
        });
    }

    #[test]
    fn resolve_manifest_dir_reports_missing_environment() {
        let generator = EsFluentGenerator::builder()
            .crate_name("missing-crate")
            .build();

        with_env_var("CARGO_MANIFEST_DIR", None, || {
            let err = generator
                .resolve_manifest_dir()
                .expect_err("missing env should fail");
            assert!(
                matches!(err, GeneratorError::CrateName(message) if message.contains("CARGO_MANIFEST_DIR not set"))
            );
        });
    }

    #[test]
    fn resolve_helpers_report_config_errors_when_manifest_lacks_i18n_toml() {
        let temp = tempdir().expect("tempdir");
        let generator = EsFluentGenerator::builder()
            .crate_name("missing-crate")
            .manifest_dir(temp.path())
            .build();

        let output_err = generator
            .resolve_output_path()
            .expect_err("missing config should fail");
        assert!(matches!(output_err, GeneratorError::Config(_)));

        let assets_err = generator
            .resolve_assets_dir()
            .expect_err("missing config should fail");
        assert!(matches!(assets_err, GeneratorError::Config(_)));
    }

    #[test]
    fn resolve_clean_paths_supports_single_or_all_locales() {
        let temp = tempdir().expect("tempdir");
        write_basic_i18n_config(temp.path());

        let generator = EsFluentGenerator::builder()
            .crate_name("missing-crate")
            .manifest_dir(temp.path())
            .build();

        let single = generator
            .resolve_clean_paths(false)
            .expect("single clean path");
        assert_eq!(single, vec![temp.path().join("i18n/en-US")]);

        let all = generator
            .resolve_clean_paths(true)
            .expect("all clean paths");
        assert_eq!(
            all,
            vec![temp.path().join("i18n/en-US"), temp.path().join("i18n/fr")]
        );
    }

    #[test]
    fn resolve_clean_paths_falls_back_to_output_override_when_assets_dir_missing() {
        let temp = tempdir().expect("tempdir");
        let fallback_output = temp.path().join("fallback-output");
        let generator = EsFluentGenerator::builder()
            .crate_name("missing-crate")
            .manifest_dir(temp.path())
            .output_path(&fallback_output)
            .assets_dir(temp.path().join("missing-assets"))
            .build();

        let paths = generator
            .resolve_clean_paths(true)
            .expect("resolve clean paths");
        assert_eq!(paths, vec![fallback_output]);
    }

    #[test]
    fn validate_namespaces_allows_configured_namespaces_only() {
        let temp = tempdir().expect("tempdir");
        write_basic_i18n_config(temp.path());

        let generator = EsFluentGenerator::builder()
            .crate_name("missing-crate")
            .manifest_dir(temp.path())
            .build();

        generator
            .validate_namespaces(&[&ALLOWED_INFO], temp.path())
            .expect("allowed namespace should pass");

        let err = generator
            .validate_namespaces(&[&DISALLOWED_INFO], temp.path())
            .expect_err("disallowed namespace should fail");
        assert!(matches!(
            err,
            GeneratorError::InvalidNamespace {
                namespace,
                type_name,
                ..
            } if namespace == "errors" && type_name == "DisallowedType"
        ));
    }

    #[test]
    fn generate_and_clean_handle_empty_inventory() {
        let temp = tempdir().expect("tempdir");
        write_basic_i18n_config(temp.path());

        let generator = EsFluentGenerator::builder()
            .crate_name("missing-crate")
            .manifest_dir(temp.path())
            .build();

        let generate_changed = generator.generate().expect("generate");
        assert!(!generate_changed);

        let clean_changed = generator.clean(false, false).expect("clean");
        assert!(!clean_changed);

        let clean_all_changed = generator.clean(true, true).expect("clean all");
        assert!(!clean_all_changed);
    }

    #[test]
    fn clean_marks_changes_when_cleaner_rewrites_files() {
        let temp = tempdir().expect("tempdir");
        write_basic_i18n_config(temp.path());

        let target_file = temp.path().join("i18n/en-US/coverage-test-crate.ftl");
        std::fs::write(
            &target_file,
            "## GroupA\n\ngroup_a-Key1 = Keep\norphan-Old = stale value\n",
        )
        .expect("write stale ftl");

        let generator = EsFluentGenerator::builder()
            .crate_name("coverage-test-crate")
            .manifest_dir(temp.path())
            .build();

        let changed = generator.clean(false, false).expect("clean");
        assert!(changed);
    }

    #[test]
    fn detect_crate_name_works_in_test_environment() {
        with_env_vars(
            &[
                ("CARGO_MANIFEST_DIR", Some(env!("CARGO_MANIFEST_DIR"))),
                ("CARGO_PKG_NAME", Some(env!("CARGO_PKG_NAME"))),
            ],
            || {
                let crate_name = EsFluentGenerator::detect_crate_name().expect("crate name");
                assert_eq!(crate_name, env!("CARGO_PKG_NAME"));
            },
        );
    }

    #[test]
    fn detect_crate_name_uses_env_fallback_or_errors_when_unavailable() {
        let temp = tempdir().expect("tempdir");

        with_env_vars(
            &[
                ("CARGO_MANIFEST_DIR", temp.path().to_str()),
                ("CARGO_PKG_NAME", Some("env-fallback-crate")),
            ],
            || {
                let crate_name = EsFluentGenerator::detect_crate_name().expect("crate name");
                assert_eq!(crate_name, "env-fallback-crate");
            },
        );

        with_env_vars(
            &[
                ("CARGO_MANIFEST_DIR", temp.path().to_str()),
                ("CARGO_PKG_NAME", None),
            ],
            || {
                let err = EsFluentGenerator::detect_crate_name().expect_err("should fail");
                assert!(
                    matches!(err, GeneratorError::CrateName(message) if message.contains("Could not determine crate name"))
                );
            },
        );

        with_env_var("CARGO_MANIFEST_DIR", None, || {
            let err = EsFluentGenerator::detect_crate_name().expect_err("missing env should fail");
            assert!(
                matches!(err, GeneratorError::CrateName(message) if message.contains("CARGO_MANIFEST_DIR not set"))
            );
        });
    }

    #[test]
    fn env_helpers_restore_unset_variables() {
        let key = format!("ES_FLUENT_TEST_UNSET_{}_A", std::process::id());
        with_env_var(&key, Some("value"), || {
            assert_eq!(std::env::var(&key).expect("set"), "value");
        });
        assert!(std::env::var(&key).is_err());

        let key_a = format!("ES_FLUENT_TEST_UNSET_{}_B", std::process::id());
        let key_b = format!("ES_FLUENT_TEST_UNSET_{}_C", std::process::id());
        with_env_vars(
            &[(key_a.as_str(), Some("first")), (key_b.as_str(), None)],
            || {
                assert_eq!(std::env::var(&key_a).expect("set"), "first");
                assert!(std::env::var(&key_b).is_err());
            },
        );
        assert!(std::env::var(&key_a).is_err());
        assert!(std::env::var(&key_b).is_err());
    }

    #[test]
    fn collect_type_infos_returns_empty_for_unknown_crate() {
        let infos = collect_type_infos("definitely_unknown_crate_name");
        assert!(infos.is_empty());
    }
}

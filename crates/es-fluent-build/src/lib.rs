#![doc = include_str!("../README.md")]

use std::env;
use std::path::{Path, PathBuf};
mod error;
use cargo_metadata::PackageName;
use error::FluentBuildError;
pub use es_fluent_generate::FluentParseMode;
use state_shift::{impl_state, type_state};

#[type_state(
    states = (Unset, ModeSet),
    slots = (Unset)
)]
pub struct FluentBuilder {
    mode: FluentParseMode,
}

#[impl_state]
impl FluentBuilder {
    /// Creates a new `FluentBuilder`.
    #[require(Unset)]
    pub fn new() -> FluentBuilder {
        FluentBuilder {
            mode: FluentParseMode::default(),
        }
    }

    /// Sets the `FluentParseMode`.
    #[require(Unset)]
    #[switch_to(ModeSet)]
    pub fn mode(self, mode: FluentParseMode) -> FluentBuilder {
        FluentBuilder { mode }
    }

    #[require(A)]
    pub fn build(self) -> Result<(), FluentBuildError> {
        println!("cargo:rerun-if-env-changed=ES_FLUENT_SKIP_BUILD");

        // Allow consumers to skip FTL generation (e.g. during `cargo publish`)
        // Example:
        //   ES_FLUENT_SKIP_BUILD cargo publish --workspace --dry-run
        let skip_build = env::var("ES_FLUENT_SKIP_BUILD")
            .ok()
            .map(|value| {
                let normalized = value.trim().to_ascii_lowercase();
                normalized.is_empty()
                    || !(normalized == "0" || normalized == "false" || normalized == "no")
            })
            .unwrap_or(false);

        if skip_build {
            println!(
                "cargo:warning=es-fluent-build: skipping FTL generation because ES_FLUENT_SKIP_BUILD is set"
            );
            return Ok(());
        }

        let crate_name = {
            let manifest_dir =
                std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
            let manifest_path = PathBuf::from(&manifest_dir).join("Cargo.toml");

            cargo_metadata::MetadataCommand::new()
                .exec()
                .ok()
                .and_then(|metadata| {
                    metadata
                        .packages
                        .iter()
                        .find(|pkg| pkg.manifest_path == manifest_path)
                        .map(|pkg| pkg.name.clone())
                })
                .unwrap_or_else(|| {
                    let crate_name = std::env::var("CARGO_PKG_NAME")
                        .expect("Error fetching `CARGO_PKG_NAME` env");
                    PackageName::new(crate_name)
                })
        };

        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let src_dir = Path::new(&manifest_dir).join("src");

        let i18n_config_file = PathBuf::from("i18n.toml");
        let i18n_config = es_fluent_toml::I18nConfig::read_from_path(&i18n_config_file)?;

        let i18n_output_path = i18n_config.assets_dir.join(&i18n_config.fallback_language);

        println!("cargo:rerun-if-changed={}", src_dir.display());
        println!("cargo:rerun-if-changed=i18n.toml");
        println!("cargo:rerun-if-env-changed=CARGO_PKG_NAME");

        let file_path = i18n_output_path.join(format!("{}.ftl", &crate_name));
        println!("cargo:rerun-if-changed={}", file_path.display());

        let data = es_fluent_sc_parser::parse_directory(&src_dir)?;
        es_fluent_generate::generate(&crate_name, i18n_output_path, data, self.mode)?;
        Ok(())
    }
}

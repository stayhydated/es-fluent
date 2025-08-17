use std::path::{Path, PathBuf};
mod error;
use error::FluentBuildError;
use state_shift::{impl_state, type_state};

pub use es_fluent_generate::FluentParseMode;

#[type_state(
    states = (Unset, ModeSet),
    slots = (Unset)
)]
pub struct FluentBuilder {
    mode: FluentParseMode,
}

#[impl_state]
impl FluentBuilder {
    #[require(Unset)]
    pub fn new() -> FluentBuilder {
        FluentBuilder {
            mode: FluentParseMode::default(),
        }
    }

    #[require(Unset)]
    #[switch_to(ModeSet)]
    pub fn mode(self, mode: FluentParseMode) -> FluentBuilder {
        FluentBuilder { mode }
    }

    #[require(A)]
    pub fn build(self) -> Result<(), FluentBuildError> {
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
                    std::env::var("CARGO_PKG_NAME").expect("Error fetching `CARGO_PKG_NAME` env")
                })
        };

        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let src_dir = Path::new(&manifest_dir).join("src");

        let i18n_config_file = PathBuf::from("i18n.toml");
        let i18n_config = i18n_config::I18nConfig::from_file(&i18n_config_file)?;

        let i18n_output_path = match i18n_config.fluent {
            Some(fluent_config) => fluent_config
                .assets_dir
                .join(i18n_config.fallback_language.to_string()),
            None => {
                return Err(FluentBuildError::NoI18nConfig(i18n_config_file));
            },
        };

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

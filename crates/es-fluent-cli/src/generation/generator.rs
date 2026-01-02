use crate::core::{CrateInfo, FluentParseMode, FluentParseModeExt as _};
use crate::generation::{
    CargoTomlTemplate, MainRsTemplate, create_temp_dir, get_es_fluent_dep, run_cargo,
    write_cargo_toml, write_main_rs,
};
use anyhow::{Result, bail};
use askama::Template as _;

const TEMP_CRATE_NAME: &str = "es-fluent-gen";

/// Generates FTL files for a crate using the CrateInfo struct.
pub fn generate_for_crate(krate: &CrateInfo, mode: &FluentParseMode) -> Result<()> {
    if !krate.has_lib_rs {
        bail!(
            "Crate '{}' has no lib.rs - inventory requires a library target for linking",
            krate.name
        );
    }

    let temp_dir = create_temp_dir(krate)?;

    let crate_ident = krate.name.replace('-', "_");
    let manifest_path = krate.manifest_dir.join("Cargo.toml");
    let es_fluent_dep = get_es_fluent_dep(&manifest_path, "generate");

    let cargo_toml = CargoTomlTemplate {
        crate_name: TEMP_CRATE_NAME,
        parent_crate_name: &krate.name,
        es_fluent_dep: &es_fluent_dep,
        has_fluent_features: !krate.fluent_features.is_empty(),
        fluent_features: &krate.fluent_features,
    };
    write_cargo_toml(&temp_dir, &cargo_toml.render().unwrap())?;

    let i18n_toml_path_str = krate.i18n_config_path.display().to_string();
    let main_rs = MainRsTemplate {
        crate_ident: &crate_ident,
        i18n_toml_path: &i18n_toml_path_str,
        parse_mode: mode.as_code(),
        crate_name: &krate.name,
    };
    write_main_rs(&temp_dir, &main_rs.render().unwrap())?;

    run_cargo(&temp_dir)
}

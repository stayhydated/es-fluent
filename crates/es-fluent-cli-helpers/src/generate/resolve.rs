use super::{EsFluentGenerator, GeneratorError};
use es_fluent_toml::ResolvedI18nLayout;
use std::path::PathBuf;

pub(super) fn resolve_crate_name(generator: &EsFluentGenerator) -> Result<String, GeneratorError> {
    generator
        .crate_name
        .clone()
        .map_or_else(detect_crate_name, Ok)
}

pub(super) fn resolve_output_path(
    generator: &EsFluentGenerator,
) -> Result<PathBuf, GeneratorError> {
    if let Some(path) = &generator.output_path {
        return Ok(path.clone());
    }

    Ok(resolve_layout(generator)?.output_dir)
}

pub(super) fn resolve_assets_dir(generator: &EsFluentGenerator) -> Result<PathBuf, GeneratorError> {
    if let Some(path) = &generator.assets_dir {
        return Ok(path.clone());
    }

    Ok(resolve_layout(generator)?.assets_dir)
}

pub(super) fn resolve_manifest_dir(
    generator: &EsFluentGenerator,
) -> Result<PathBuf, GeneratorError> {
    if let Some(path) = &generator.manifest_dir {
        return Ok(path.clone());
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| GeneratorError::CrateName("CARGO_MANIFEST_DIR not set".to_string()))?;
    Ok(PathBuf::from(manifest_dir))
}

pub(super) fn resolve_layout(
    generator: &EsFluentGenerator,
) -> Result<ResolvedI18nLayout, GeneratorError> {
    let manifest_dir = resolve_manifest_dir(generator)?;
    Ok(ResolvedI18nLayout::from_manifest_dir(&manifest_dir)?)
}

pub(super) fn resolve_clean_paths(
    generator: &EsFluentGenerator,
    all_locales: bool,
) -> Result<Vec<PathBuf>, GeneratorError> {
    if !all_locales {
        return Ok(vec![resolve_output_path(generator)?]);
    }

    let mut paths = if let Ok(layout) = resolve_layout(generator) {
        layout
            .available_locale_names()?
            .into_iter()
            .map(|locale| layout.locale_dir(&locale))
            .collect::<Vec<_>>()
    } else if let Ok(assets_dir) = resolve_assets_dir(generator) {
        es_fluent_runner::get_all_locales(&assets_dir)?
            .into_iter()
            .map(|locale| assets_dir.join(locale))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    if paths.is_empty() {
        return Ok(vec![resolve_output_path(generator)?]);
    }

    paths.sort();
    Ok(paths)
}

pub(super) fn detect_crate_name() -> Result<String, GeneratorError> {
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

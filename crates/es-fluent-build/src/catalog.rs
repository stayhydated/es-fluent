use std::path::{Path, PathBuf};

use es_fluent_shared::fluent::FluentDomain;
use es_fluent_shared::resource::{FALLBACK_CATALOG_FILE_NAME, FallbackCatalog, ResourcePlan};
use es_fluent_toml::ResolvedI18nLayout;

pub(super) fn write_fallback_catalog(
    layout: &ResolvedI18nLayout,
    package_name: &str,
    out_dir: &Path,
) -> Result<(), String> {
    layout
        .config
        .validate_for_package(package_name)
        .map_err(|error| error.to_string())?;
    let mut domains =
        vec![FluentDomain::try_new(package_name.to_string()).map_err(|error| error.to_string())?];
    domains.extend(layout.config.domains.iter().cloned());

    let mut catalog = FallbackCatalog::default();
    let crate_root_assets = assets_dir_is_manifest_root(layout);
    for domain in domains {
        let paths = if crate_root_assets {
            fallback_root_resource_paths(layout, &domain)?
        } else {
            validate_sparse_catalog_inputs(layout, &domain)?;
            let plans = ResourcePlan::sparse_from_assets(domain.as_str(), &layout.assets_dir)
                .map_err(|error| error.to_string())?;
            let Some((_, resources)) = plans
                .resource_specs_by_language()
                .iter()
                .find(|(language, _)| language == &layout.config.fallback_language)
            else {
                continue;
            };

            resources
                .iter()
                .map(|resource| {
                    layout
                        .output_dir
                        .join(resource.locale_relative_path.as_str())
                })
                .collect()
        };

        for path in paths {
            validate_catalog_resource_path(&layout.assets_dir, &path)?;
            let source = std::fs::read_to_string(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            catalog.insert_source(&domain, source).map_err(|error| {
                format!(
                    "failed to catalog fallback resource {}: {error}",
                    path.display()
                )
            })?;
        }
    }

    let path = out_dir.join(FALLBACK_CATALOG_FILE_NAME);
    std::fs::write(&path, catalog.encode())
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn assets_dir_is_manifest_root(layout: &ResolvedI18nLayout) -> bool {
    match (
        layout.manifest_dir.canonicalize(),
        layout.assets_dir.canonicalize(),
    ) {
        (Ok(manifest_dir), Ok(assets_dir)) => manifest_dir == assets_dir,
        _ => false,
    }
}

/// Validates sparse locale assets before resource-plan discovery.
///
/// This is shared with CLI diagnostics so build-time and diagnostic input
/// acceptance stay synchronized.
#[doc(hidden)]
pub fn validate_sparse_catalog_inputs(
    layout: &ResolvedI18nLayout,
    domain: &FluentDomain,
) -> Result<(), String> {
    let entries = std::fs::read_dir(&layout.assets_dir).map_err(|error| {
        format!(
            "failed to read locale assets directory {}: {error}",
            layout.assets_dir.display()
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "failed to read directory entry in {}: {error}",
                layout.assets_dir.display()
            )
        })?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path).map_err(|error| {
            format!("failed to inspect locale asset {}: {error}", path.display())
        })?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "locale asset entries must not be symlinks: {}",
                path.display()
            ));
        }
        if metadata.is_dir() {
            catalog_resource_paths_for_locale(domain, &path)?;
        }
    }

    Ok(())
}

fn fallback_root_resource_paths(
    layout: &ResolvedI18nLayout,
    domain: &FluentDomain,
) -> Result<Vec<PathBuf>, String> {
    let locales = layout
        .available_locale_names()
        .map_err(|error| error.to_string())?;

    let mut fallback_paths = Vec::new();
    for locale in locales {
        let locale_dir = layout.assets_dir.join(&locale);
        let paths = catalog_resource_paths_for_locale(domain, &locale_dir)?;
        if locale == layout.fallback_language {
            fallback_paths.extend(paths);
        }
    }

    fallback_paths.sort();
    Ok(fallback_paths)
}

fn catalog_resource_paths_for_locale(
    domain: &FluentDomain,
    locale_dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    let locale_metadata = std::fs::symlink_metadata(locale_dir).map_err(|error| {
        format!(
            "failed to inspect locale directory {}: {error}",
            locale_dir.display()
        )
    })?;
    if locale_metadata.file_type().is_symlink() {
        return Err(format!(
            "locale directory must be a real directory, not a symlink: {}",
            locale_dir.display()
        ));
    }

    let mut paths = Vec::new();
    let base_path = locale_dir.join(format!("{}.ftl", domain.as_str()));
    match std::fs::symlink_metadata(&base_path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!(
                "Fluent resource must be a real file, not a symlink: {}",
                base_path.display()
            ));
        },
        Ok(_) => paths.push(base_path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
        Err(error) => {
            return Err(format!(
                "failed to inspect Fluent resource {}: {error}",
                base_path.display()
            ));
        },
    }

    let namespace_root = locale_dir.join(domain.as_str());
    match std::fs::symlink_metadata(&namespace_root) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!(
                "Fluent namespace must be a real directory, not a symlink: {}",
                namespace_root.display()
            ));
        },
        Ok(metadata) if metadata.is_dir() => {
            paths.extend(discover_namespace_paths(domain, &namespace_root)?);
        },
        Ok(_) => {},
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
        Err(error) => {
            return Err(format!(
                "failed to inspect Fluent namespace {}: {error}",
                namespace_root.display()
            ));
        },
    }

    Ok(paths)
}

fn validate_catalog_resource_path(assets_dir: &Path, path: &Path) -> Result<(), String> {
    let relative = path.strip_prefix(assets_dir).map_err(|error| {
        format!(
            "failed to validate catalog resource {} relative to {}: {error}",
            path.display(),
            assets_dir.display()
        )
    })?;
    let mut current = assets_dir.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        let metadata = std::fs::symlink_metadata(&current).map_err(|error| {
            format!(
                "failed to inspect catalog resource component {}: {error}",
                current.display()
            )
        })?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "catalog resource paths must not contain symlinks: {}",
                current.display()
            ));
        }
    }
    Ok(())
}

fn discover_namespace_paths(
    domain: &FluentDomain,
    namespace_root: &Path,
) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    let mut pending = vec![namespace_root.to_path_buf()];

    while let Some(current_dir) = pending.pop() {
        let entries = std::fs::read_dir(&current_dir)
            .map_err(|error| format!("failed to read {}: {error}", current_dir.display()))?;

        for entry in entries {
            let entry = entry.map_err(|error| {
                format!(
                    "failed to read directory entry in {}: {error}",
                    current_dir.display()
                )
            })?;
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path).map_err(|error| {
                format!(
                    "failed to inspect fallback asset {}: {error}",
                    path.display()
                )
            })?;

            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "fallback Fluent namespace entries must not be symlinks: {}",
                    path.display()
                ));
            }

            if metadata.is_dir() {
                pending.push(path);
                continue;
            }

            if !metadata.is_file() {
                continue;
            }

            if path.extension().and_then(|extension| extension.to_str()) != Some("ftl") {
                continue;
            }

            let relative_path = path.strip_prefix(namespace_root).map_err(|error| {
                format!(
                    "failed to derive namespace for asset {} relative to {}: {error}",
                    path.display(),
                    namespace_root.display()
                )
            })?;
            let relative_without_extension = relative_path.with_extension("");
            let mut components = Vec::new();
            for component in relative_without_extension.components() {
                let component = component.as_os_str().to_str().ok_or_else(|| {
                    format!(
                        "namespace path {} contains non-UTF-8 components",
                        relative_without_extension.display()
                    )
                })?;
                components.push(component);
            }

            if components.is_empty() {
                continue;
            }

            let namespace = components.join("/");
            es_fluent_shared::namespace::ResolvedNamespace::new(namespace.clone()).map_err(
                |error| {
                    format!(
                        "discovered invalid namespace '{namespace}' in assets for crate '{}': {error}",
                        domain.as_str()
                    )
                },
            )?;
            paths.push(path);
        }
    }

    Ok(paths)
}

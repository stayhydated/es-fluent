use es_fluent_shared::{
    fluent::FluentDomain,
    namespace::ResolvedNamespace,
    resource::{FallbackCatalog, ResourcePlan},
};
use es_fluent_toml::ResolvedI18nLayout;
use std::path::Path;

pub(super) fn fallback_catalog_inputs(
    layout: &ResolvedI18nLayout,
    package: &str,
) -> Result<usize, String> {
    let mut domains = vec![
        FluentDomain::try_new(package.to_string())
            .map_err(|error| format!("invalid package domain `{package}`: {error}"))?,
    ];
    domains.extend(layout.config.domains.iter().cloned());
    let mut catalog = FallbackCatalog::default();
    let mut resource_count = 0;

    for domain in domains {
        let paths = if assets_dir_is_manifest_root(layout) {
            let locales = layout
                .available_locale_names()
                .map_err(|error| error.to_string())?;
            let domain_names = [domain.as_str().to_string()];
            let mut fallback_paths = Vec::new();
            for locale in locales {
                let resources = crate::ftl::discover_domain_ftl_files_in_locale_dir(
                    &layout.assets_dir.join(&locale),
                    &domain_names,
                )
                .map_err(|error| error.to_string())?;
                for resource in &resources {
                    validate_discovered_namespace(&resource.relative_path, &domain)?;
                }
                if locale == layout.fallback_language() {
                    fallback_paths.extend(resources.into_iter().map(|resource| resource.abs_path));
                }
            }
            fallback_paths
        } else {
            es_fluent_build::validate_sparse_catalog_inputs(layout, &domain)?;
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
                .collect::<Vec<_>>()
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
            resource_count += 1;
        }
    }

    Ok(resource_count)
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

fn validate_discovered_namespace(
    locale_relative_path: &Path,
    domain: &FluentDomain,
) -> Result<(), String> {
    let Ok(namespaced_path) = locale_relative_path.strip_prefix(domain.as_str()) else {
        return Ok(());
    };
    let namespace_path = namespaced_path.with_extension("");
    let namespace = namespace_path
        .components()
        .map(|component| {
            component.as_os_str().to_str().ok_or_else(|| {
                format!(
                    "namespace path {} contains non-UTF-8 components",
                    namespace_path.display()
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?
        .join("/");
    ResolvedNamespace::new(namespace.clone()).map_err(|error| {
        format!(
            "discovered invalid namespace '{namespace}' in locale resource {} for domain '{}': {error}",
            locale_relative_path.display(),
            domain.as_str()
        )
    })?;
    Ok(())
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

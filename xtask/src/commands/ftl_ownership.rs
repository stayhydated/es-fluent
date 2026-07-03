use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    path::{Component, Path, PathBuf},
};

use anyhow::{Context as _, bail};
use cargo_metadata::MetadataCommand;
use es_fluent_runner::{PackageName, RunnerMetadataStore};
use es_fluent_shared::resource::ResourcePlan;

#[derive(Debug)]
struct I18nPackage {
    name: String,
    assets_dir: PathBuf,
    owned_domains: BTreeSet<String>,
    owned_locale_relative_paths: BTreeSet<String>,
    inventory_locale_relative_paths: BTreeSet<String>,
}

pub fn run() -> anyhow::Result<()> {
    let workspace_root = stayhydated_xtask::workspace_root_from_xtask_manifest()?;
    let packages = discover_i18n_packages(&workspace_root)?;
    let owner_by_domain = owner_by_domain(&packages)?;
    let mut diagnostics = Vec::new();

    validate_inventory_resource_claims(&packages, &mut diagnostics);
    validate_no_duplicate_domain_files(
        &workspace_root,
        &packages,
        &owner_by_domain,
        &mut diagnostics,
    )?;

    if !diagnostics.is_empty() {
        let details = diagnostics
            .into_iter()
            .map(|diagnostic| format!("- {diagnostic}"))
            .collect::<Vec<_>>()
            .join("\n");
        bail!("FTL ownership check failed:\n{details}");
    }

    println!(
        "FTL ownership check passed for {} i18n packages",
        packages.len()
    );
    Ok(())
}

fn discover_i18n_packages(workspace_root: &Path) -> anyhow::Result<Vec<I18nPackage>> {
    let metadata = MetadataCommand::new()
        .manifest_path(workspace_root.join("Cargo.toml"))
        .exec()
        .context("failed to read workspace metadata")?;
    let workspace_members = metadata.workspace_members.iter().collect::<HashSet<_>>();
    let metadata_store = RunnerMetadataStore::new(workspace_root.join(".es-fluent"));
    let mut packages = Vec::new();

    for package in metadata.packages {
        if !workspace_members.contains(&package.id) {
            continue;
        }

        let manifest_dir = package
            .manifest_path
            .as_std_path()
            .parent()
            .context("workspace package manifest path has no parent")?
            .to_path_buf();
        if !manifest_dir.join("i18n.toml").is_file() {
            continue;
        }

        let layout = es_fluent_toml::ResolvedI18nLayout::from_manifest_dir(&manifest_dir)
            .with_context(|| {
                format!(
                    "failed to resolve i18n layout for package '{}'",
                    package.name
                )
            })?;
        let sparse_plan =
            ResourcePlan::sparse_from_assets(package.name.as_str(), &layout.assets_dir)
                .with_context(|| {
                    format!(
                        "failed to build sparse resource plan for package '{}'",
                        package.name
                    )
                })?;

        let (_, _, specs_by_language) = sparse_plan.into_parts();
        let owned_locale_relative_paths = specs_by_language
            .iter()
            .flat_map(|(_, specs)| {
                specs
                    .iter()
                    .map(|spec| spec.locale_relative_path.to_string())
            })
            .collect::<BTreeSet<_>>();
        let inventory_locale_relative_paths =
            read_inventory_locale_relative_paths(&metadata_store, package.name.as_str())?;
        let mut owned_domains = BTreeSet::from([package.name.to_string()]);
        owned_domains.extend(
            inventory_locale_relative_paths
                .iter()
                .filter_map(|path| domain_from_locale_relative_path(path))
                .map(str::to_string),
        );

        packages.push(I18nPackage {
            name: package.name.to_string(),
            assets_dir: layout.assets_dir,
            owned_domains,
            owned_locale_relative_paths,
            inventory_locale_relative_paths,
        });
    }

    packages.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(packages)
}

fn read_inventory_locale_relative_paths(
    store: &RunnerMetadataStore,
    package_name: &str,
) -> anyhow::Result<BTreeSet<String>> {
    let package = PackageName::try_new(package_name.to_string())
        .with_context(|| format!("invalid package name '{package_name}'"))?;
    let inventory_path = store.inventory_path(&package);
    if !inventory_path.is_file() {
        return Ok(BTreeSet::new());
    }

    let inventory = store
        .read_inventory(&package)
        .with_context(|| format!("failed to read {}", inventory_path.display()))?;
    Ok(inventory
        .expected_keys
        .into_iter()
        .filter_map(|key| key.resource)
        .map(|resource| resource.locale_relative_path.to_string())
        .collect())
}

fn owner_by_domain(packages: &[I18nPackage]) -> anyhow::Result<BTreeMap<String, String>> {
    let mut owner_by_domain = BTreeMap::new();
    let mut diagnostics = Vec::new();

    for package in packages {
        for domain in &package.owned_domains {
            if let Some(existing_owner) = owner_by_domain.get(domain) {
                if existing_owner != &package.name {
                    diagnostics.push(format!(
                        "domain '{domain}' is claimed by both '{existing_owner}' and '{}'",
                        package.name
                    ));
                }
            } else {
                owner_by_domain.insert(domain.clone(), package.name.clone());
            }
        }
    }

    if diagnostics.is_empty() {
        Ok(owner_by_domain)
    } else {
        bail!("{}", diagnostics.join("\n"));
    }
}

fn validate_inventory_resource_claims(packages: &[I18nPackage], diagnostics: &mut Vec<String>) {
    for package in packages {
        for path in &package.inventory_locale_relative_paths {
            if !package.owned_locale_relative_paths.contains(path) {
                diagnostics.push(format!(
                    "package '{}' has generated inventory for '{}' but no owner-side FTL file for that locale-relative path under {}",
                    package.name,
                    path,
                    package.assets_dir.display()
                ));
            }
        }
    }
}

fn validate_no_duplicate_domain_files(
    workspace_root: &Path,
    packages: &[I18nPackage],
    owner_by_domain: &BTreeMap<String, String>,
    diagnostics: &mut Vec<String>,
) -> anyhow::Result<()> {
    for package in packages {
        for file in collect_ftl_files(&package.assets_dir)? {
            let Some((locale, locale_relative_path)) =
                locale_and_relative_path(&package.assets_dir, &file)
            else {
                continue;
            };
            let Some(domain) = domain_from_locale_relative_path(&locale_relative_path) else {
                continue;
            };
            let Some(owner) = owner_by_domain.get(domain) else {
                continue;
            };

            if owner == &package.name {
                continue;
            }

            diagnostics.push(format!(
                "registered owner crate: '{owner}'; duplicate file: {}; locale: '{locale}'; locale-relative path: '{locale_relative_path}'. Fix: remove this duplicate and load the owner module through inventory, or move app-specific text into the '{}' domain.",
                display_path(workspace_root, &file),
                package.name
            ));
        }
    }

    Ok(())
}

fn collect_ftl_files(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();

    while let Some(dir) = pending.pop() {
        for entry in
            std::fs::read_dir(&dir).with_context(|| format!("failed to read {}", dir.display()))?
        {
            let entry =
                entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("ftl") {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

fn locale_and_relative_path(assets_dir: &Path, file: &Path) -> Option<(String, String)> {
    let relative = file.strip_prefix(assets_dir).ok()?;
    let mut components = normal_components(relative)?;
    if components.len() < 2 {
        return None;
    }

    let locale = components.remove(0);
    Some((locale, components.join("/")))
}

fn domain_from_locale_relative_path(path: &str) -> Option<&str> {
    let first_segment = path.split('/').next()?;
    first_segment
        .strip_suffix(".ftl")
        .or(Some(first_segment))
        .filter(|domain| !domain.is_empty())
}

fn normal_components(path: &Path) -> Option<Vec<String>> {
    path.components()
        .map(|component| match component {
            Component::Normal(value) => value.to_str().map(str::to_string),
            _ => None,
        })
        .collect()
}

fn display_path(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_from_locale_relative_path_handles_base_and_namespaced_resources() {
        assert_eq!(
            domain_from_locale_relative_path("example-shared-lib.ftl"),
            Some("example-shared-lib")
        );
        assert_eq!(
            domain_from_locale_relative_path("bevy-example/ui.ftl"),
            Some("bevy-example")
        );
        assert_eq!(domain_from_locale_relative_path(""), None);
    }

    #[test]
    fn locale_and_relative_path_splits_locale_root_from_resource_path() {
        let assets = Path::new("assets/i18n");
        let file = assets.join("fr-FR/readme/namespaces/user.ftl");

        assert_eq!(
            locale_and_relative_path(assets, &file),
            Some((
                "fr-FR".to_string(),
                "readme/namespaces/user.ftl".to_string()
            ))
        );
    }
}

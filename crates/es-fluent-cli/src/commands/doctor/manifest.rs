use std::path::Path;

#[derive(Clone, Debug, Default)]
pub(super) struct DependencySpec {
    pub(super) alias: String,
    pub(super) package: String,
    pub(super) features: Vec<String>,
    pub(super) optional: bool,
}

#[derive(Debug, Default)]
pub(super) struct ManifestSummary {
    pub(super) build_helpers: Vec<DependencySpec>,
    pub(super) conditional_build_helpers: Vec<ConditionalDependency>,
    pub(super) managers: Vec<DependencySpec>,
}

#[derive(Clone, Debug)]
pub(super) struct ConditionalDependency {
    pub(super) target: String,
    pub(super) package: String,
}

pub(super) fn manifest_summary(
    path: &Path,
    workspace_root: &Path,
) -> Result<ManifestSummary, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let manifest: toml::Value = toml::from_str(&source)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    let workspace_manifest = std::fs::read_to_string(workspace_root.join("Cargo.toml"))
        .ok()
        .and_then(|source| toml::from_str::<toml::Value>(&source).ok());
    let workspace_dependencies = workspace_manifest
        .as_ref()
        .and_then(|manifest| manifest.get("workspace"))
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(toml::Value::as_table);
    let normal_dependencies =
        dependency_specs_for_target(&manifest, "dependencies", workspace_dependencies).0;
    let (build_dependencies, conditional_build_helpers) =
        dependency_specs_for_target(&manifest, "build-dependencies", workspace_dependencies);
    let managers = normal_dependencies
        .iter()
        .filter(|dependency| {
            matches!(
                dependency.package.as_str(),
                "es-fluent-manager-embedded"
                    | "es-fluent-manager-dioxus"
                    | "es-fluent-manager-bevy"
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    Ok(ManifestSummary {
        build_helpers: build_dependencies
            .iter()
            .filter(|dependency| dependency.package == "es-fluent-build")
            .cloned()
            .collect(),
        conditional_build_helpers: conditional_build_helpers
            .into_iter()
            .filter(|dependency| dependency.package == "es-fluent-build")
            .collect(),
        managers,
    })
}

#[cfg(test)]
pub(super) fn dependency_specs(
    manifest: &toml::Value,
    table_name: &str,
    workspace_dependencies: Option<&toml::Table>,
) -> Vec<DependencySpec> {
    dependency_specs_for_target(manifest, table_name, workspace_dependencies).0
}

fn dependency_specs_for_target(
    manifest: &toml::Value,
    table_name: &str,
    workspace_dependencies: Option<&toml::Table>,
) -> (Vec<DependencySpec>, Vec<ConditionalDependency>) {
    let mut specs = Vec::new();
    let mut conditional = Vec::new();
    collect_dependency_table(manifest.get(table_name), workspace_dependencies, &mut specs);
    if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
        for (target_name, target) in targets {
            let mut target_specs = Vec::new();
            collect_dependency_table(
                target.get(table_name),
                workspace_dependencies,
                &mut target_specs,
            );
            if target_specs.is_empty() {
                continue;
            }
            conditional.extend(
                target_specs
                    .into_iter()
                    .map(|dependency| ConditionalDependency {
                        target: target_name.clone(),
                        package: dependency.package,
                    }),
            );
        }
    }
    (specs, conditional)
}

fn collect_dependency_table(
    value: Option<&toml::Value>,
    workspace_dependencies: Option<&toml::Table>,
    specs: &mut Vec<DependencySpec>,
) {
    let Some(table) = value.and_then(toml::Value::as_table) else {
        return;
    };
    for (alias, value) in table {
        let details = value.as_table();
        let inherited_details = details
            .and_then(|details| details.get("workspace"))
            .and_then(toml::Value::as_bool)
            .filter(|workspace| *workspace)
            .and(workspace_dependencies)
            .and_then(|dependencies| dependencies.get(alias))
            .and_then(toml::Value::as_table);
        let package = details
            .and_then(|details| details.get("package"))
            .or_else(|| inherited_details.and_then(|details| details.get("package")))
            .and_then(toml::Value::as_str)
            .unwrap_or(alias)
            .to_string();
        let mut features = dependency_features(inherited_details);
        for feature in dependency_features(details) {
            if !features.contains(&feature) {
                features.push(feature);
            }
        }
        let optional = details
            .and_then(|details| details.get("optional"))
            .or_else(|| inherited_details.and_then(|details| details.get("optional")))
            .and_then(toml::Value::as_bool)
            .unwrap_or(false);
        specs.push(DependencySpec {
            alias: alias.clone(),
            package,
            features,
            optional,
        });
    }
}

fn dependency_features(details: Option<&toml::Table>) -> Vec<String> {
    details
        .and_then(|details| details.get("features"))
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str)
        .map(str::to_string)
        .collect()
}

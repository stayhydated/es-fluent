use super::common::{OutputFormat, WorkspaceArgs, WorkspaceCrates};
use crate::core::{CliError, CrateInfo};
use crate::source_inspector::{InspectionOutcome, SourceTarget};
use anstream::println;
use clap::Args;
use es_fluent_shared::fluent::FluentDomain;
use es_fluent_shared::namespace::ResolvedNamespace;
use es_fluent_shared::resource::{FallbackCatalog, ResourcePlan};
use es_fluent_toml::ResolvedI18nLayout;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Args)]
pub struct DoctorArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::default())]
    pub output: OutputFormat,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum DoctorStatus {
    Pass,
    Warning,
    Error,
}

impl DoctorStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warning => "WARN",
            Self::Error => "ERROR",
        }
    }
}

#[derive(Debug, Serialize)]
struct DoctorCheck {
    package: String,
    category: &'static str,
    status: DoctorStatus,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    help: Option<String>,
}

#[derive(Debug, Serialize)]
struct DoctorReport {
    crates_discovered: usize,
    crates_checked: usize,
    workspace_errors: Vec<String>,
    checks: Vec<DoctorCheck>,
    error_count: usize,
    warning_count: usize,
    healthy: bool,
}

impl DoctorReport {
    fn new(
        crates_discovered: usize,
        workspace_errors: Vec<String>,
        checks: Vec<DoctorCheck>,
    ) -> Self {
        let error_count = workspace_errors.len()
            + checks
                .iter()
                .filter(|check| matches!(check.status, DoctorStatus::Error))
                .count();
        let warning_count = checks
            .iter()
            .filter(|check| matches!(check.status, DoctorStatus::Warning))
            .count();
        let crates_checked = checks
            .iter()
            .map(|check| check.package.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len();

        Self {
            crates_discovered,
            crates_checked,
            workspace_errors,
            checks,
            error_count,
            warning_count,
            healthy: error_count == 0,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct DependencySpec {
    package: String,
    features: Vec<String>,
    optional: bool,
}

#[derive(Debug, Default)]
struct ManifestSummary {
    build_helper: bool,
    conditional_build_helpers: Vec<ConditionalDependency>,
    managers: Vec<DependencySpec>,
}

#[derive(Clone, Debug)]
struct ConditionalDependency {
    target: String,
    package: String,
}

pub fn run_doctor(args: DoctorArgs) -> Result<(), CliError> {
    let output = args.output;
    let workspace = match WorkspaceCrates::discover(args.workspace) {
        Ok(workspace) => workspace,
        Err(error) => {
            let report = DoctorReport::new(0, vec![error.to_string()], Vec::new());
            render_report(&report, output)?;
            return Err(CliError::Exit(1));
        },
    };

    let mut workspace_errors = workspace
        .empty_selection_message()
        .into_iter()
        .collect::<Vec<_>>();
    let checks = workspace
        .crates
        .iter()
        .flat_map(|krate| diagnose_crate(krate, &workspace.workspace_info.root_dir))
        .collect::<Vec<_>>();
    if workspace.crates.is_empty() && workspace_errors.is_empty() {
        workspace_errors.push("no configured crates were selected".to_string());
    }
    let report = DoctorReport::new(workspace.crates.len(), workspace_errors, checks);
    render_report(&report, output)?;

    if report.healthy {
        Ok(())
    } else {
        Err(CliError::Exit(1))
    }
}

fn diagnose_crate(krate: &CrateInfo, workspace_root: &Path) -> Vec<DoctorCheck> {
    let package = krate.name.to_string();
    let mut checks = Vec::new();
    let layout = match ResolvedI18nLayout::from_config_path(&krate.i18n_config_path) {
        Ok(layout) => {
            pass(
                &mut checks,
                &package,
                "configuration",
                format!(
                    "i18n.toml is valid (fallback `{}`, assets `{}`, missing-message policy `{}`)",
                    layout.fallback_language(),
                    relative_path(&layout.assets_dir, workspace_root),
                    layout.missing_message_policy()
                ),
            );
            layout
        },
        Err(error) => {
            fail(
                &mut checks,
                &package,
                "configuration",
                format!("failed to read i18n.toml: {error}"),
                "fix fallback_language, assets_dir, missing_message_policy, domains, namespaces, and feature values",
            );
            return checks;
        },
    };

    if let Some(library_target) = &krate.library_target_path {
        pass(
            &mut checks,
            &package,
            "library_target",
            format!(
                "Cargo library target is available at `{}` for derive inventory",
                relative_path(library_target, workspace_root)
            ),
        );
    } else {
        fail(
            &mut checks,
            &package,
            "library_target",
            "no Cargo library target is available for derive inventory",
            "add src/lib.rs or configure [lib] path in Cargo.toml",
        );
    }

    let fallback_dir = layout.output_dir.as_path();
    if fallback_dir.is_dir() {
        pass(
            &mut checks,
            &package,
            "fallback_locale",
            format!(
                "fallback locale directory exists at `{}`",
                relative_path(fallback_dir, workspace_root)
            ),
        );
    } else {
        fail(
            &mut checks,
            &package,
            "fallback_locale",
            format!(
                "fallback locale directory is missing at `{}`",
                relative_path(fallback_dir, workspace_root)
            ),
            format!("create {}", fallback_dir.display()),
        );
    }

    let manifest_path = krate.manifest_dir.join("Cargo.toml");
    let manifest = match manifest_summary(&manifest_path, workspace_root) {
        Ok(summary) => summary,
        Err(error) => {
            fail(
                &mut checks,
                &package,
                "manifest",
                error,
                "fix Cargo.toml before rerunning doctor",
            );
            ManifestSummary::default()
        },
    };

    if manifest.build_helper {
        pass(
            &mut checks,
            &package,
            "build_dependency",
            "es-fluent-build is declared under build-dependencies",
        );
    } else if !manifest.conditional_build_helpers.is_empty() {
        let targets = manifest
            .conditional_build_helpers
            .iter()
            .map(|dependency| format!("`{}`", dependency.target))
            .collect::<Vec<_>>()
            .join(", ");
        warn(
            &mut checks,
            &package,
            "build_dependency",
            format!(
                "es-fluent-build is declared only under target-specific build-dependencies whose active state could not be proven: {targets}"
            ),
            "declare es-fluent-build under [build-dependencies] or verify the active Cargo target and target condition",
        );
    } else {
        fail(
            &mut checks,
            &package,
            "build_dependency",
            "es-fluent-build is not declared under build-dependencies",
            "add es-fluent-build to [build-dependencies] in Cargo.toml",
        );
    }

    if let Some(build_target) = &krate.custom_build_target_path {
        let target_path = relative_path(build_target, workspace_root);
        match crate::source_inspector::inspect(
            build_target,
            &krate.manifest_dir,
            SourceTarget::Call("track_i18n_assets"),
        ) {
            InspectionOutcome::Found(evidence) => pass(
                &mut checks,
                &package,
                "build_script",
                format!(
                    "verified track_i18n_assets() call at `{}:{}` in Cargo custom-build target `{target_path}`",
                    relative_path(&evidence.path, workspace_root),
                    evidence.line
                ),
            ),
            InspectionOutcome::NotFound => fail(
                &mut checks,
                &package,
                "build_script",
                format!(
                    "no track_i18n_assets() call was found in Cargo custom-build target graph `{target_path}`"
                ),
                "call es_fluent_build::track_i18n_assets() from the selected custom-build target or a reachable local module",
            ),
            InspectionOutcome::Indeterminate(reason) => warn(
                &mut checks,
                &package,
                "build_script",
                format!(
                    "could not prove build integration for `{target_path}`: {}",
                    relative_message(&reason, workspace_root)
                ),
                "verify that the selected custom-build target calls es_fluent_build::track_i18n_assets()",
            ),
        }
    } else {
        fail(
            &mut checks,
            &package,
            "build_script",
            "Cargo metadata reports no custom-build target",
            "add a build script that calls es_fluent_build::track_i18n_assets() or enable the package build target",
        );
    }

    if manifest.managers.is_empty() {
        warn(
            &mut checks,
            &package,
            "manager",
            "no embedded, Dioxus, or Bevy manager dependency was found",
            "add a concrete manager or verify that this package intentionally uses a custom integration",
        );
    } else {
        let names = manifest
            .managers
            .iter()
            .map(|dependency| {
                if dependency.features.is_empty() {
                    dependency.package.clone()
                } else {
                    format!(
                        "{} [{}]",
                        dependency.package,
                        dependency.features.join(", ")
                    )
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        pass(
            &mut checks,
            &package,
            "manager",
            format!("manager dependency declared: {names}"),
        );

        for manager in &manifest.managers {
            if manager.package == "es-fluent-manager-dioxus"
                && !manager.optional
                && !manager
                    .features
                    .iter()
                    .any(|feature| matches!(feature.as_str(), "client" | "ssr"))
            {
                warn(
                    &mut checks,
                    &package,
                    "manager_features",
                    "Dioxus manager is active without a client or ssr runtime feature",
                    "enable client, ssr, or both on es-fluent-manager-dioxus",
                );
            }
        }
    }

    pass(
        &mut checks,
        &package,
        "missing_message_policy",
        format!(
            "package-local missing-message policy is `{}`",
            layout.missing_message_policy()
        ),
    );

    if let Some(library_target) = &krate.library_target_path {
        match crate::source_inspector::inspect(
            library_target,
            &krate.manifest_dir,
            SourceTarget::Macro("define_i18n_module"),
        ) {
            InspectionOutcome::Found(evidence) => pass(
                &mut checks,
                &package,
                "manager_registration",
                format!(
                    "verified define_i18n_module!() invocation at `{}:{}`",
                    relative_path(&evidence.path, workspace_root),
                    evidence.line
                ),
            ),
            InspectionOutcome::NotFound if !manifest.managers.is_empty() => fail(
                &mut checks,
                &package,
                "manager_registration",
                format!(
                    "no define_i18n_module!() invocation was found in Cargo library target graph `{}`",
                    relative_path(library_target, workspace_root)
                ),
                "register the package's locale assets from a library-reachable module",
            ),
            InspectionOutcome::NotFound => {},
            InspectionOutcome::Indeterminate(reason) if !manifest.managers.is_empty() => warn(
                &mut checks,
                &package,
                "manager_registration",
                format!(
                    "could not prove manager registration: {}",
                    relative_message(&reason, workspace_root)
                ),
                "verify that define_i18n_module!() is invoked from the selected library target graph",
            ),
            InspectionOutcome::Indeterminate(_) => {},
        }
    }

    match fallback_catalog_inputs(&layout, krate.name.as_str()) {
        Ok(0) => warn(
            &mut checks,
            &package,
            "catalog",
            "fallback catalog inputs contain no FTL resources",
            "run cargo es-fluent generate if the package declares localizable messages",
        ),
        Ok(resource_count) => pass(
            &mut checks,
            &package,
            "catalog",
            format!("fallback catalog inputs are ready across {resource_count} FTL resource(s)"),
        ),
        Err(error) => fail(
            &mut checks,
            &package,
            "catalog",
            error,
            "fix fallback FTL namespace paths, syntax, or duplicate message/term IDs, then rerun doctor",
        ),
    }

    checks
}

fn manifest_summary(path: &Path, workspace_root: &Path) -> Result<ManifestSummary, String> {
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
        build_helper: build_dependencies
            .iter()
            .any(|dependency| dependency.package == "es-fluent-build"),
        conditional_build_helpers: conditional_build_helpers
            .into_iter()
            .filter(|dependency| dependency.package == "es-fluent-build")
            .collect(),
        managers,
    })
}

#[cfg(test)]
fn dependency_specs(
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

fn fallback_catalog_inputs(layout: &ResolvedI18nLayout, package: &str) -> Result<usize, String> {
    let mut domains = vec![
        FluentDomain::try_new(package.to_string())
            .map_err(|error| format!("invalid package domain `{package}`: {error}"))?,
    ];
    domains.extend(layout.config.domains.iter().cloned());
    let mut catalog = FallbackCatalog::default();
    let mut resource_count = 0;

    for domain in domains {
        let paths = if assets_dir_is_manifest_root(layout) {
            // `sparse_from_assets` treats every directory as a locale. Root
            // assets intentionally share the crate root with ordinary Cargo
            // directories, so use the CLI's package-owned resource discovery
            // after the config layer has filtered locale candidates.
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

fn render_report(report: &DoctorReport, output: OutputFormat) -> Result<(), CliError> {
    if output.is_json() {
        return output.print_json(report);
    }

    println!("Fluent Setup Doctor");
    println!("Discovered {} crate(s)", report.crates_discovered);
    for error in &report.workspace_errors {
        println!("ERROR workspace: {error}");
    }
    let mut current_package = None;
    for check in &report.checks {
        if current_package != Some(check.package.as_str()) {
            println!();
            println!("{}", check.package);
            current_package = Some(check.package.as_str());
        }
        println!(
            "  {} {}: {}",
            check.status.label(),
            check.category,
            check.message
        );
        if let Some(help) = &check.help {
            println!("    help: {help}");
        }
    }
    println!();
    println!(
        "Summary: {} error(s), {} warning(s)",
        report.error_count, report.warning_count
    );
    Ok(())
}

fn pass(
    checks: &mut Vec<DoctorCheck>,
    package: &str,
    category: &'static str,
    message: impl Into<String>,
) {
    checks.push(DoctorCheck {
        package: package.to_string(),
        category,
        status: DoctorStatus::Pass,
        message: message.into(),
        help: None,
    });
}

fn warn(
    checks: &mut Vec<DoctorCheck>,
    package: &str,
    category: &'static str,
    message: impl Into<String>,
    help: impl Into<String>,
) {
    checks.push(DoctorCheck {
        package: package.to_string(),
        category,
        status: DoctorStatus::Warning,
        message: message.into(),
        help: Some(help.into()),
    });
}

fn fail(
    checks: &mut Vec<DoctorCheck>,
    package: &str,
    category: &'static str,
    message: impl Into<String>,
    help: impl Into<String>,
) {
    checks.push(DoctorCheck {
        package: package.to_string(),
        category,
        status: DoctorStatus::Error,
        message: message.into(),
        help: Some(help.into()),
    });
}

fn relative_path(path: &Path, root: &Path) -> String {
    crate::utils::paths::relative_slash_path(path, root)
}

fn relative_message(message: &str, root: &Path) -> String {
    crate::utils::paths::relative_slash_message(message, root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn dependency_specs_support_aliases_and_manager_features() {
        let manifest: toml::Value = toml::from_str(
            r#"
[dependencies]
i18n = { package = "es-fluent", version = "0.1" }
manager = { package = "es-fluent-manager-dioxus", version = "0.1", features = ["client"] }

[build-dependencies]
build-i18n = { package = "es-fluent-build", version = "0.1" }
"#,
        )
        .expect("manifest");
        let normal = dependency_specs(&manifest, "dependencies", None);
        let build = dependency_specs(&manifest, "build-dependencies", None);

        assert!(normal.iter().any(|dependency| {
            dependency.package == "es-fluent" && dependency.features.is_empty()
        }));
        assert!(normal.iter().any(|dependency| {
            dependency.package == "es-fluent-manager-dioxus"
                && dependency.features == ["client".to_string()]
        }));
        assert!(
            build
                .iter()
                .any(|dependency| dependency.package == "es-fluent-build")
        );
    }

    #[test]
    fn dependency_specs_merge_workspace_dependency_features() {
        let workspace: toml::Value = toml::from_str(
            r#"
[workspace.dependencies]
manager = { package = "es-fluent-manager-dioxus", version = "0.7", features = ["client"] }
"#,
        )
        .expect("workspace manifest");
        let package: toml::Value = toml::from_str(
            r#"
[dependencies]
manager = { workspace = true, features = ["ssr"] }
"#,
        )
        .expect("package manifest");
        let workspace_dependencies = workspace["workspace"]["dependencies"]
            .as_table()
            .expect("workspace dependencies");
        let dependencies = dependency_specs(&package, "dependencies", Some(workspace_dependencies));

        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies[0].package, "es-fluent-manager-dioxus");
        assert_eq!(dependencies[0].features, ["client", "ssr"]);
    }

    #[test]
    fn doctor_report_is_unhealthy_only_for_errors() {
        let warning = DoctorCheck {
            package: "app".to_string(),
            category: "manager",
            status: DoctorStatus::Warning,
            message: "custom manager".to_string(),
            help: None,
        };
        let report = DoctorReport::new(1, Vec::new(), vec![warning]);
        assert!(report.healthy);
        assert_eq!(report.warning_count, 1);

        let report = DoctorReport::new(0, vec!["missing config".to_string()], Vec::new());
        assert!(!report.healthy);
        assert_eq!(report.error_count, 1);
    }

    #[test]
    fn fallback_catalog_inputs_ignore_crate_root_project_directories() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("en")).expect("create locale");
        fs::create_dir_all(temp.path().join("src")).expect("create src");
        fs::create_dir_all(temp.path().join("target")).expect("create target");
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \".\"\n",
        )
        .expect("write config");
        fs::write(temp.path().join("en/test-app.ftl"), "hello = Hello\n")
            .expect("write fallback resource");
        let layout =
            ResolvedI18nLayout::from_config_path(temp.path().join("i18n.toml")).expect("layout");

        assert_eq!(
            fallback_catalog_inputs(&layout, "test-app").expect("catalog"),
            1
        );
    }

    #[test]
    fn fallback_catalog_inputs_recognize_normalized_crate_root_assets() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir(temp.path().join("locale")).expect("create normalized path component");
        fs::create_dir(temp.path().join("en")).expect("create locale");
        fs::create_dir(temp.path().join("src")).expect("create src");
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"locale/..\"\n",
        )
        .expect("write config");
        fs::write(temp.path().join("en/test-app.ftl"), "hello = Hello\n")
            .expect("write fallback resource");
        let layout =
            ResolvedI18nLayout::from_config_path(temp.path().join("i18n.toml")).expect("layout");

        assert_eq!(
            fallback_catalog_inputs(&layout, "test-app").expect("catalog"),
            1
        );
    }

    #[cfg(unix)]
    #[test]
    fn fallback_catalog_inputs_reject_symlinked_fallback_resource() {
        let temp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside tempdir");
        fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write config");
        let outside_resource = outside.path().join("test-app.ftl");
        fs::write(&outside_resource, "hello = Outside\n").expect("write outside resource");
        std::os::unix::fs::symlink(&outside_resource, temp.path().join("i18n/en/test-app.ftl"))
            .expect("create fallback resource symlink");
        let layout =
            ResolvedI18nLayout::from_config_path(temp.path().join("i18n.toml")).expect("layout");

        let error = fallback_catalog_inputs(&layout, "test-app")
            .expect_err("symlinked fallback resources should fail doctor validation");
        assert!(error.contains("catalog resource paths must not contain symlinks"));
    }

    #[test]
    fn fallback_catalog_inputs_reject_invalid_namespace_in_non_fallback_root_locale() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("en")).expect("create fallback locale");
        fs::create_dir_all(temp.path().join("fr/test-app")).expect("create namespace dir");
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \".\"\n",
        )
        .expect("write config");
        fs::write(temp.path().join("en/test-app.ftl"), "hello = Hello\n")
            .expect("write fallback resource");
        fs::write(
            temp.path().join("fr/test-app/ bad .ftl"),
            "hello = Bonjour\n",
        )
        .expect("write translated resource");
        let layout =
            ResolvedI18nLayout::from_config_path(temp.path().join("i18n.toml")).expect("layout");

        let error = fallback_catalog_inputs(&layout, "test-app")
            .expect_err("invalid namespace should fail doctor catalog validation");
        assert!(error.contains("discovered invalid namespace ' bad '"));
        assert!(error.contains("leading or trailing whitespace"));
    }
}

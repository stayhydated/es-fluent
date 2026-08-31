use crate::{
    core::CrateInfo,
    source_inspector::{InspectionOutcome, SourceTarget},
};
use es_fluent_toml::ResolvedI18nLayout;
use std::path::Path;

use super::{
    catalog::fallback_catalog_inputs,
    manifest::manifest_summary,
    model::{DoctorCheck, fail, pass, warn},
    render::{relative_message, relative_path},
};

pub(super) fn diagnose_crate(krate: &CrateInfo, workspace_root: &Path) -> Vec<DoctorCheck> {
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
            Default::default()
        },
    };

    if !manifest.build_helpers.is_empty() {
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
        let build_helper_roots = manifest
            .build_helpers
            .iter()
            .map(|dependency| dependency.alias.replace('-', "_"))
            .collect::<Vec<_>>();
        match crate::source_inspector::inspect(
            build_target,
            &krate.manifest_dir,
            SourceTarget::build_helper_call(&build_helper_roots),
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
        let manager_roots = manifest
            .managers
            .iter()
            .map(|dependency| dependency.alias.replace('-', "_"))
            .collect::<Vec<_>>();
        match crate::source_inspector::inspect(
            library_target,
            &krate.manifest_dir,
            SourceTarget::Macro("define_i18n_module", Some(&manager_roots)),
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

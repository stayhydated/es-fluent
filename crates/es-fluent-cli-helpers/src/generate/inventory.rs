use super::GeneratorError;
use es_fluent::registry::FtlTypeInfo;
use es_fluent_runner::PackageName;
use es_fluent_toml::ResolvedI18nLayout;
use std::path::Path;

pub(super) fn collect_type_infos(crate_name: &str) -> Vec<&'static FtlTypeInfo> {
    let crate_ident = PackageName::try_new(crate_name)
        .expect("crate names should be valid package names")
        .rust_module_prefix()
        .to_string();
    es_fluent::registry::get_all_ftl_type_infos()
        .filter(|info| {
            info.module_path() == crate_ident
                || info
                    .module_path()
                    .starts_with(&format!("{}::", crate_ident))
        })
        .collect()
}

pub(super) fn validate_namespaces(
    type_infos: &[&'static FtlTypeInfo],
    manifest_dir: &Path,
) -> Result<(), GeneratorError> {
    let layout = ResolvedI18nLayout::from_manifest_dir(manifest_dir).ok();
    let allowed = layout
        .as_ref()
        .and_then(ResolvedI18nLayout::allowed_namespaces);

    for info in type_infos {
        let Some(ns) = info
            .try_resolved_namespace(manifest_dir)
            .map_err(|details| GeneratorError::InvalidNamespacePath {
                namespace: info
                    .resolved_namespace(manifest_dir)
                    .unwrap_or_else(|| "<none>".to_string()),
                type_name: info.type_name().to_string(),
                details,
            })?
        else {
            continue;
        };

        if let Some(allowed_namespaces) = allowed
            && !allowed_namespaces
                .iter()
                .any(|allowed| allowed.as_str() == ns.as_str())
        {
            return Err(GeneratorError::InvalidNamespace {
                namespace: ns.to_string(),
                type_name: info.type_name().to_string(),
                allowed: allowed_namespaces
                    .iter()
                    .map(|namespace| namespace.as_str().to_string())
                    .collect(),
            });
        }
    }

    Ok(())
}

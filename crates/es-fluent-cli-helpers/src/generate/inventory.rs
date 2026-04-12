use super::GeneratorError;
use es_fluent::registry::FtlTypeInfo;
use es_fluent_toml::ResolvedI18nLayout;
use std::path::Path;

pub(super) fn collect_type_infos(crate_name: &str) -> Vec<&'static FtlTypeInfo> {
    let crate_ident = crate_name.replace('-', "_");
    es_fluent::registry::get_all_ftl_type_infos()
        .filter(|info| {
            info.module_path == crate_ident
                || info.module_path.starts_with(&format!("{}::", crate_ident))
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

    if let Some(allowed_namespaces) = allowed {
        for info in type_infos {
            if let Some(ns) = info.resolved_namespace(manifest_dir)
                && !allowed_namespaces.contains(&ns)
            {
                return Err(GeneratorError::InvalidNamespace {
                    namespace: ns,
                    type_name: info.type_name.to_string(),
                    allowed: allowed_namespaces.to_vec(),
                });
            }
        }
    }

    Ok(())
}

use es_fluent_derive_core::EsFluentResult;
use es_fluent_derive_core::registry::FtlTypeInfo;
use indexmap::IndexMap;
use std::fs;
use std::path::Path;

/// Cleans a Fluent translation file by removing unused orphan keys while preserving existing translations.
pub fn clean<P: AsRef<Path>, M: AsRef<Path>, I: AsRef<FtlTypeInfo>>(
    crate_name: &str,
    i18n_path: P,
    manifest_dir: M,
    items: &[I],
    dry_run: bool,
) -> EsFluentResult<bool> {
    let i18n_path = i18n_path.as_ref();
    let manifest_dir = manifest_dir.as_ref();
    let items_ref: Vec<&FtlTypeInfo> = items.iter().map(|i| i.as_ref()).collect();

    // Group items by namespace
    let mut namespaced: IndexMap<Option<String>, Vec<&FtlTypeInfo>> = IndexMap::new();
    for item in &items_ref {
        let namespace = item.resolved_namespace(manifest_dir);
        namespaced.entry(namespace).or_default().push(item);
    }

    let mut any_changed = false;

    for (namespace, ns_items) in namespaced {
        let (dir_path, file_path) = match namespace {
            Some(ns) => {
                let dir = i18n_path.join(crate_name);
                let file = dir.join(format!("{}.ftl", ns));
                (dir, file)
            },
            None => (
                i18n_path.to_path_buf(),
                i18n_path.join(format!("{}.ftl", crate_name)),
            ),
        };

        if !dry_run {
            fs::create_dir_all(&dir_path)?;
        }

        let existing_resource = crate::read_existing_resource(&file_path)?;
        let final_resource =
            crate::smart_merge(existing_resource, &ns_items, crate::MergeBehavior::Clean);

        // Use standard serialization to preserve order (no sorting for clean)
        if crate::write_updated_resource(&file_path, &final_resource, dry_run, |resource| {
            fluent_syntax::serializer::serialize(resource)
        })? {
            any_changed = true;
        }
    }

    Ok(any_changed)
}

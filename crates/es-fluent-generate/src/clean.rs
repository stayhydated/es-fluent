use crate::error::FluentGenerateError;
use es_fluent_derive_core::registry::FtlTypeInfo;
use std::fs;
use std::path::Path;

/// Cleans a Fluent translation file by removing unused orphan keys while preserving existing translations.
pub fn clean<P: AsRef<Path>, I: AsRef<FtlTypeInfo>>(
    crate_name: &str,
    i18n_path: P,
    items: &[I],
    dry_run: bool,
) -> Result<bool, FluentGenerateError> {
    let i18n_path = i18n_path.as_ref();

    if !dry_run {
        fs::create_dir_all(i18n_path)?;
    }

    let file_path = i18n_path.join(format!("{}.ftl", crate_name));

    let existing_resource = crate::read_existing_resource(&file_path)?;
    let items_ref: Vec<&FtlTypeInfo> = items.iter().map(|i| i.as_ref()).collect();
    let final_resource =
        crate::smart_merge(existing_resource, &items_ref, crate::MergeBehavior::Clean);

    // Use standard serialization to preserve order (no sorting for clean)
    crate::write_updated_resource(&file_path, &final_resource, dry_run, |resource| {
        fluent_syntax::serializer::serialize(resource)
    })
}

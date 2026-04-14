use es_fluent_shared::EsFluentResult;
use es_fluent_shared::registry::FtlTypeInfo;
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
    let mut any_changed = false;

    let operation = crate::pipeline::OutputOperation::Clean;
    for output in crate::pipeline::plan_outputs(crate_name, i18n_path, manifest_dir, items) {
        if crate::pipeline::apply_output_operation(output, &operation, dry_run)? {
            any_changed = true;
        }
    }

    Ok(any_changed)
}

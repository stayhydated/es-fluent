use es_fluent_shared::EsFluentResult;
use es_fluent_shared::registry::FtlTypeInfo;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

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
    let planned_outputs =
        crate::pipeline::plan_outputs(crate_name, i18n_path, manifest_dir, items)?;
    let main_file_path = i18n_path.join(format!("{}.ftl", crate_name));
    let has_main_output = planned_outputs
        .iter()
        .any(|output| output.file_path == main_file_path);
    let expected_namespace_files = planned_outputs
        .iter()
        .map(|output| output.file_path.clone())
        .filter(|path| path.starts_with(i18n_path.join(crate_name)))
        .collect::<HashSet<_>>();

    for output in planned_outputs {
        if crate::pipeline::apply_output_operation(output, &operation, dry_run)? {
            any_changed = true;
        }
    }
    if !has_main_output && remove_stale_main_file(&main_file_path, dry_run)? {
        any_changed = true;
    }
    if remove_stale_namespace_files(crate_name, i18n_path, &expected_namespace_files, dry_run)? {
        any_changed = true;
    }

    Ok(any_changed)
}

fn remove_stale_main_file(file_path: &Path, dry_run: bool) -> EsFluentResult<bool> {
    if !file_path.is_file() {
        return Ok(false);
    }

    if dry_run {
        let display_path = fs::canonicalize(file_path).unwrap_or_else(|_| file_path.to_path_buf());
        println!(
            "Would remove stale main FTL file: {}",
            display_path.display()
        );
        return Ok(true);
    }

    fs::remove_file(file_path)?;
    Ok(true)
}

fn remove_stale_namespace_files(
    crate_name: &str,
    i18n_path: &Path,
    expected_namespace_files: &HashSet<PathBuf>,
    dry_run: bool,
) -> EsFluentResult<bool> {
    let namespace_root = i18n_path.join(crate_name);
    if !namespace_root.is_dir() {
        return Ok(false);
    }

    let mut changed = false;
    let mut pending = vec![namespace_root.clone()];

    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                pending.push(path);
                continue;
            }

            if path.extension().and_then(|ext| ext.to_str()) != Some("ftl") {
                continue;
            }
            if expected_namespace_files.contains(&path) {
                continue;
            }

            if !dry_run {
                fs::remove_file(&path)?;
            }
            changed = true;
        }
    }

    if changed && !dry_run {
        remove_empty_namespace_dirs(&namespace_root)?;
    }

    Ok(changed)
}

fn remove_empty_namespace_dirs(root: &Path) -> EsFluentResult<()> {
    let mut dirs = vec![root.to_path_buf()];
    let mut all_dirs = Vec::new();

    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                dirs.push(entry.path());
            }
        }
        all_dirs.push(dir);
    }

    all_dirs.sort_by_key(|dir| std::cmp::Reverse(dir.components().count()));
    for dir in all_dirs {
        if fs::read_dir(&dir)?.next().is_none() {
            fs::remove_dir(&dir)?;
        }
    }

    Ok(())
}

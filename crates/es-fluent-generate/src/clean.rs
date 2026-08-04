use es_fluent_runner::FileTransaction;
use es_fluent_shared::EsFluentResult;
use es_fluent_shared::registry::FtlTypeInfo;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Makes selected package/domain resources match the supplied Rust inventory.
pub fn clean<P: AsRef<Path>, M: AsRef<Path>, I: AsRef<FtlTypeInfo>>(
    crate_name: &str,
    i18n_path: P,
    manifest_dir: M,
    items: &[I],
    dry_run: bool,
) -> EsFluentResult<bool> {
    let transaction = plan_clean(crate_name, i18n_path, manifest_dir, items)?;
    crate::io::apply_transaction(&transaction, dry_run)
}

/// Plans stale-entry and stale-file removal without writing to disk.
pub fn plan_clean<P: AsRef<Path>, M: AsRef<Path>, I: AsRef<FtlTypeInfo>>(
    crate_name: &str,
    i18n_path: P,
    manifest_dir: M,
    items: &[I],
) -> EsFluentResult<FileTransaction> {
    let i18n_path = i18n_path.as_ref();
    let manifest_dir = manifest_dir.as_ref();
    let mut transaction = FileTransaction::default();

    let operation = crate::pipeline::OutputOperation::Clean;
    let planned_outputs =
        crate::pipeline::plan_outputs(crate_name, i18n_path, manifest_dir, items)?;
    let mut owned_domains = vec![crate_name.to_string()];
    owned_domains.extend(items.iter().filter_map(|item| {
        item.as_ref()
            .domain()
            .map(|domain| domain.as_str().to_string())
    }));
    match es_fluent_toml::I18nConfig::from_manifest_dir(manifest_dir) {
        Ok(config) => owned_domains.extend(
            config
                .domains
                .iter()
                .map(|domain| domain.as_str().to_string()),
        ),
        Err(es_fluent_toml::I18nConfigError::NotFound) => {},
        Err(error) => return Err(std::io::Error::other(error).into()),
    }
    owned_domains.sort();
    owned_domains.dedup();
    let expected_main_files = planned_outputs
        .iter()
        .filter(|output| output.route.is_base())
        .map(|output| output.file_path.clone())
        .collect::<HashSet<_>>();
    let expected_namespace_files = planned_outputs
        .iter()
        .filter(|output| !output.route.is_base())
        .map(|output| output.file_path.clone())
        .collect::<HashSet<_>>();

    for output in planned_outputs {
        crate::pipeline::plan_output_operation(output, &operation, &mut transaction)?;
    }
    for domain in owned_domains {
        let main_file_path = i18n_path.join(format!("{domain}.ftl"));
        if !expected_main_files.contains(&main_file_path) {
            plan_stale_main_file(&mut transaction, &main_file_path)?;
        }
        plan_stale_namespace_files(
            &mut transaction,
            &domain,
            i18n_path,
            &expected_namespace_files,
        )?;
    }

    Ok(transaction)
}

fn plan_stale_main_file(
    transaction: &mut FileTransaction,
    file_path: &Path,
) -> EsFluentResult<bool> {
    if !file_path.is_file() {
        return Ok(false);
    }

    transaction
        .plan_remove(file_path)
        .map_err(|error| std::io::Error::other(error).into())
}

fn plan_stale_namespace_files(
    transaction: &mut FileTransaction,
    crate_name: &str,
    i18n_path: &Path,
    expected_namespace_files: &HashSet<PathBuf>,
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

            transaction
                .plan_remove(&path)
                .map_err(std::io::Error::other)?;
            changed = true;
        }
    }

    if changed {
        plan_empty_namespace_dirs(transaction, &namespace_root)?;
    }

    Ok(changed)
}

fn plan_empty_namespace_dirs(transaction: &mut FileTransaction, root: &Path) -> EsFluentResult<()> {
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

    for dir in all_dirs {
        transaction.plan_remove_empty_directory(dir);
    }

    Ok(())
}

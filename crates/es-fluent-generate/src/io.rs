use es_fluent_runner::FileTransaction;
use es_fluent_shared::EsFluentResult;
use fluent_syntax::ast;
use fs_err as fs;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

/// Print a colored line-by-line diff between old and new content.
pub(crate) fn print_diff(old: &str, new: &str) {
    use colored::Colorize as _;
    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(old, new);

    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            println!("{}", "  ...".dimmed());
        }
        for op in group {
            for change in diff.iter_changes(op) {
                let sign = match change.tag() {
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+",
                    ChangeTag::Equal => " ",
                };
                let line = format!("{} {}", sign, change);
                match change.tag() {
                    ChangeTag::Delete => print!("{}", line.red()),
                    ChangeTag::Insert => print!("{}", line.green()),
                    ChangeTag::Equal => print!("{}", line.dimmed()),
                }
            }
        }
    }
}

/// Read and parse an existing FTL resource file.
#[cfg(test)]
pub(crate) fn read_existing_resource(file_path: &Path) -> EsFluentResult<ast::Resource<String>> {
    crate::ftl::parse_ftl_file(file_path).map_err(Into::into)
}

pub(crate) fn read_existing_resource_and_content(
    file_path: &Path,
) -> EsFluentResult<(ast::Resource<String>, Option<Vec<u8>>)> {
    let content = match fs::read(file_path) {
        Ok(content) => Some(content),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let text = match &content {
        Some(content) => String::from_utf8(content.clone()).map_err(|error| {
            Error::new(
                ErrorKind::InvalidData,
                format!(
                    "FTL file is not valid UTF-8: {}: {error}",
                    file_path.display()
                ),
            )
        })?,
        None => String::new(),
    };
    let (resource, errors) = crate::ftl::parse_ftl_content(text);
    if !errors.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Refusing to use '{}' because it contains Fluent parse errors: {}",
                file_path.display(),
                crate::ftl::format_parse_errors(&errors)
            ),
        )
        .into());
    }

    Ok((resource, content))
}

pub(crate) fn plan_updated_resource(
    transaction: &mut FileTransaction,
    file_path: &Path,
    original: Option<Vec<u8>>,
    resource: &ast::Resource<String>,
    formatter: impl Fn(&ast::Resource<String>) -> String,
) -> EsFluentResult<bool> {
    let is_empty = resource.body.is_empty();
    let final_content = if is_empty {
        String::new()
    } else {
        formatter(resource)
    };
    let current_content = original
        .as_deref()
        .map(String::from_utf8_lossy)
        .unwrap_or_default();

    let has_changed = match is_empty {
        true => current_content.as_ref() != final_content && !current_content.trim().is_empty(),
        false => current_content.trim() != final_content.trim(),
    };
    if !has_changed {
        return Ok(false);
    }

    transaction
        .plan_write_from(file_path, original, final_content.into_bytes())
        .map_err(|error| Error::other(error).into())
}

/// Write an updated resource to disk, handling change detection and dry-run mode.
#[cfg(test)]
pub(crate) fn write_updated_resource(
    file_path: &Path,
    resource: &ast::Resource<String>,
    dry_run: bool,
    formatter: impl Fn(&ast::Resource<String>) -> String,
) -> EsFluentResult<bool> {
    let original = match fs::read(file_path) {
        Ok(content) => Some(content),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let mut transaction = FileTransaction::default();
    if !plan_updated_resource(&mut transaction, file_path, original, resource, formatter)? {
        log_unchanged(file_path, resource.body.is_empty(), dry_run);
        return Ok(false);
    }
    apply_transaction(&transaction, dry_run)
}

#[cfg(test)]
fn log_unchanged(file_path: &Path, is_empty: bool, dry_run: bool) {
    if dry_run {
        return;
    }
    let msg = match is_empty {
        true => format!(
            "FTL file unchanged (empty or no items): {}",
            file_path.display()
        ),
        false => format!("FTL file unchanged: {}", file_path.display()),
    };
    tracing::debug!("{}", msg);
}

/// Write changes to disk or preview them in dry-run mode.
#[cfg(test)]
pub(crate) fn write_or_preview(
    file_path: &Path,
    current_content: &str,
    final_content: &str,
    is_empty: bool,
    dry_run: bool,
) -> EsFluentResult<()> {
    let original = if file_path.exists() {
        Some(current_content.as_bytes().to_vec())
    } else {
        None
    };
    let mut transaction = FileTransaction::default();
    transaction
        .plan_write_from(file_path, original, final_content.as_bytes().to_vec())
        .map_err(Error::other)?;
    if dry_run {
        preview_write(file_path, current_content, final_content, is_empty);
    } else {
        transaction.commit().map_err(Error::other)?;
        log_committed_write(file_path, is_empty);
    }
    Ok(())
}

pub(crate) fn apply_transaction(
    transaction: &FileTransaction,
    dry_run: bool,
) -> EsFluentResult<bool> {
    if transaction.is_empty() {
        return Ok(false);
    }
    if dry_run {
        preview_transaction(transaction);
        return Ok(true);
    }

    transaction.commit().map_err(Error::other)?;
    for mutation in transaction.mutations() {
        match mutation.replacement() {
            Some(replacement) => log_committed_write(mutation.path(), replacement.is_empty()),
            None => tracing::info!("Removed FTL file: {}", mutation.path().display()),
        }
    }
    Ok(true)
}

pub(crate) fn preview_transaction(transaction: &FileTransaction) {
    for mutation in transaction.mutations() {
        let current = mutation
            .original()
            .map(String::from_utf8_lossy)
            .unwrap_or_default();
        match mutation.replacement() {
            Some(replacement) => {
                let final_content = String::from_utf8_lossy(replacement);
                preview_write(
                    mutation.path(),
                    &current,
                    &final_content,
                    replacement.is_empty(),
                );
            },
            None => {
                let display_path = canonical_or_owned(mutation.path());
                println!("Would remove FTL file: {}", display_path.display());
                print_diff(&current, "");
                println!();
            },
        }
    }
}

fn preview_write(file_path: &Path, current_content: &str, final_content: &str, is_empty: bool) {
    let display_path = canonical_or_owned(file_path);
    let msg = match (is_empty, !current_content.trim().is_empty()) {
        (true, true) => format!(
            "Would write empty FTL file (no items): {}",
            display_path.display()
        ),
        (true, false) => format!("Would write empty FTL file: {}", display_path.display()),
        (false, _) => format!("Would update FTL file: {}", display_path.display()),
    };
    println!("{}", msg);
    print_diff(current_content, final_content);
    println!();
}

fn log_committed_write(file_path: &Path, is_empty: bool) {
    let msg = match is_empty {
        true => format!("Wrote empty FTL file (no items): {}", file_path.display()),
        false => format!("Updated FTL file: {}", file_path.display()),
    };
    tracing::info!("{}", msg);
}

fn canonical_or_owned(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

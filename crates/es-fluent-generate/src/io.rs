use es_fluent_shared::EsFluentResult;
use fluent_syntax::ast;
use std::{fs, path::Path};

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
pub(crate) fn read_existing_resource(file_path: &Path) -> EsFluentResult<ast::Resource<String>> {
    crate::ftl::parse_ftl_file(file_path).map_err(Into::into)
}

/// Write an updated resource to disk, handling change detection and dry-run mode.
pub(crate) fn write_updated_resource(
    file_path: &Path,
    resource: &ast::Resource<String>,
    dry_run: bool,
    formatter: impl Fn(&ast::Resource<String>) -> String,
) -> EsFluentResult<bool> {
    let is_empty = resource.body.is_empty();
    let final_content = if is_empty {
        String::new()
    } else {
        formatter(resource)
    };

    let current_content = if file_path.exists() {
        fs::read_to_string(file_path)?
    } else {
        String::new()
    };

    let has_changed = match is_empty {
        true => current_content != final_content && !current_content.trim().is_empty(),
        false => current_content.trim() != final_content.trim(),
    };

    if !has_changed {
        log_unchanged(file_path, is_empty, dry_run);
        return Ok(false);
    }

    write_or_preview(
        file_path,
        &current_content,
        &final_content,
        is_empty,
        dry_run,
    )?;
    Ok(true)
}

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
pub(crate) fn write_or_preview(
    file_path: &Path,
    current_content: &str,
    final_content: &str,
    is_empty: bool,
    dry_run: bool,
) -> EsFluentResult<()> {
    if dry_run {
        let display_path = fs::canonicalize(file_path).unwrap_or_else(|_| file_path.to_path_buf());
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
        return Ok(());
    }

    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(file_path, final_content)?;
    let msg = match is_empty {
        true => format!("Wrote empty FTL file (no items): {}", file_path.display()),
        false => format!("Updated FTL file: {}", file_path.display()),
    };
    tracing::info!("{}", msg);
    Ok(())
}

use crate::error::FluentGenerateError;
use crate::smart_merge;
use es_fluent_core::registry::FtlTypeInfo;
use fluent_syntax::{ast, parser};
use std::fs;
use std::path::Path;

/// Cleans a Fluent translation file by removing unused orphan keys while preserving existing translations.
pub fn clean<P: AsRef<Path>>(
    crate_name: &str,
    i18n_path: P,
    items: Vec<FtlTypeInfo>,
    dry_run: bool,
) -> Result<(), FluentGenerateError> {
    let i18n_path = i18n_path.as_ref();

    if !dry_run {
        fs::create_dir_all(i18n_path)?;
    }

    let file_path = i18n_path.join(format!("{}.ftl", crate_name));

    let existing_resource = if file_path.exists() {
        let content = fs::read_to_string(&file_path)?;
        if content.trim().is_empty() {
            ast::Resource { body: Vec::new() }
        } else {
            match parser::parse(content) {
                Ok(res) => res,
                Err((res, errors)) => {
                    log::warn!(
                        "Warning: Encountered parsing errors in {}: {:?}",
                        file_path.display(),
                        errors
                    );
                    res
                }
            }
        }
    } else {
        ast::Resource { body: Vec::new() }
    };

    let final_resource = smart_merge(existing_resource, &items, crate::MergeBehavior::Clean);

    if !final_resource.body.is_empty() {
        // Use standard serialization to preserve order (no sorting)
        let final_content_to_write = fluent_syntax::serializer::serialize(&final_resource);

        let current_content = if file_path.exists() {
            fs::read_to_string(&file_path)?
        } else {
            String::new()
        };

        if current_content.trim() != final_content_to_write.trim() {
            if dry_run {
                println!("Would clean FTL file: {}", file_path.display());
            } else {
                fs::write(&file_path, &final_content_to_write)?;
                log::error!("Cleaned FTL file: {}", file_path.display());
            }
        } else {
             if dry_run {
                println!("FTL file would be unchanged: {}", file_path.display());
             } else {
                log::error!("FTL file unchanged (already clean): {}", file_path.display());
             }
        }
    } else {
        // If empty after clean, arguably we should delete it or leave it empty?
        // Following existing logic: write empty string if it wasn't empty
        let final_content_to_write = "".to_string();
        let current_content = if file_path.exists() {
            fs::read_to_string(&file_path)?
        } else {
            String::new()
        };

        if current_content != final_content_to_write && !current_content.trim().is_empty() {
            if dry_run {
                println!("Would clear FTL file (all items removed): {}", file_path.display());
            } else {
                fs::write(&file_path, &final_content_to_write)?;
                log::error!(
                    "Wrote empty FTL file (all items removed): {}",
                    file_path.display()
                );
            }
        } else {
            if current_content != final_content_to_write {
                if dry_run {
                    println!("Would write empty FTL file: {}", file_path.display());
                } else {
                    fs::write(&file_path, &final_content_to_write)?;
                }
            }
            if dry_run {
                println!("FTL file would be unchanged (empty): {}", file_path.display());
            } else {
                log::error!(
                    "FTL file unchanged (empty): {}",
                    file_path.display()
                );
            }
        }
    }

    Ok(())
}

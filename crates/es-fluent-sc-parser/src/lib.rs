use crate::visitor::FtlVisitor;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub mod error;
mod processor;
mod visitor;

use error::FluentScParserError;
use es_fluent_core::registry::FtlTypeInfo;

pub fn parse_directory(dir_path: &Path) -> Result<Vec<FtlTypeInfo>, FluentScParserError> {
    log::info!(
        "Starting FTL type info parsing in directory: {}",
        dir_path.display()
    );

    let rust_files: Vec<PathBuf> = WalkDir::new(dir_path)
        .into_iter()
        .filter_map(|entry_result| match entry_result {
            Ok(entry) => {
                let path = entry.path();
                if path.is_file()
                    && let Some(ext) = path.extension()
                    && ext == "rs"
                {
                    Some(Ok(path.to_path_buf()))
                } else {
                    None
                }
            },
            Err(e) => Some(Err(FluentScParserError::WalkDir(dir_path.to_path_buf(), e))),
        })
        .collect::<Result<Vec<_>, _>>()?;

    log::debug!("Found {} Rust files to parse.", rust_files.len());

    let file_results: Result<Vec<Vec<FtlTypeInfo>>, FluentScParserError> = rust_files
        .iter()
        .map(|file_path| {
            log::trace!("Parsing file: {}", file_path.display());
            let content = fs::read_to_string(file_path)
                .map_err(|e| FluentScParserError::Io(file_path.clone(), e))?;
            let syntax_tree = syn::parse_file(&content)
                .map_err(|e| FluentScParserError::Syn(file_path.clone(), e))?;

            let mut visitor = FtlVisitor::new(file_path);
            syn::visit::visit_file(&mut visitor, &syntax_tree);
            Ok(visitor.type_infos().to_owned())
        })
        .collect();

    let results: Vec<FtlTypeInfo> = file_results?
        .into_iter()
        .filter(|type_infos| !type_infos.is_empty())
        .flatten()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    log::info!(
        "Finished parsing. Found {} FTL type info entries.",
        results.len()
    );
    Ok(results)
}

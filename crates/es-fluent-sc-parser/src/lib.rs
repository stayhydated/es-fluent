#![doc = include_str!("../README.md")]

use crate::visitor::FtlVisitor;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub mod error;
mod processor;
mod visitor;

use error::FluentScParserError;
use es_fluent_core::registry::FtlTypeInfo;

/// Parses a directory of Rust source code and returns a list of `FtlTypeInfo`
/// objects.
///
/// # Arguments
///
/// * `dir_path` - The path to the directory to parse.
///
/// # Errors
///
/// This function will return an error if the directory cannot be read, or if
/// any of the files in the directory cannot be parsed.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_directory_empty() {
        let temp_dir = TempDir::new().unwrap();
        let result = parse_directory(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_parse_directory_with_nonexistent_path() {
        let non_existent_path = Path::new("/non/existent/path");
        let result = parse_directory(non_existent_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_directory_with_rust_file() {
        let temp_dir = TempDir::new().unwrap();
        let rust_file_path = temp_dir.path().join("test.rs");

        let rust_content = r#"
use es_fluent_core::EsFluent;

#[derive(EsFluent)]
pub enum TestEnum {
    Variant1,
    Variant2,
}
"#;

        fs::write(&rust_file_path, rust_content).unwrap();

        let result = parse_directory(temp_dir.path());
        assert!(result.is_ok());

        let type_infos = result.unwrap();
        assert!(!type_infos.is_empty());
    }

    #[test]
    fn test_parse_directory_with_multiple_rust_files() {
        let temp_dir = TempDir::new().unwrap();

        let rust_file1_path = temp_dir.path().join("test1.rs");
        let rust_content1 = r#"
use es_fluent_core::EsFluent;

#[derive(EsFluent)]
pub enum TestEnum1 {
    VariantA,
}
"#;

        fs::write(&rust_file1_path, rust_content1).unwrap();

        let rust_file2_path = temp_dir.path().join("test2.rs");
        let rust_content2 = r#"
use es_fluent_core::EsFluent;

#[derive(EsFluent)]
pub enum TestEnum2 {
    VariantB,
}
"#;
        fs::write(&rust_file2_path, rust_content2).unwrap();

        let result = parse_directory(temp_dir.path());
        assert!(result.is_ok());

        let type_infos = result.unwrap();
        assert!(type_infos.len() >= 2);
    }

    #[test]
    fn test_parse_directory_with_non_rust_file() {
        let temp_dir = TempDir::new().unwrap();
        let non_rust_file_path = temp_dir.path().join("test.txt");
        fs::write(&non_rust_file_path, "not a rust file").unwrap();

        let result = parse_directory(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_parse_enum_with_both_es_fluent_and_es_fluent_kv() {
        let temp_dir = TempDir::new().unwrap();
        let rust_file_path = temp_dir.path().join("test.rs");

        let rust_content = r#"
use es_fluent::{EsFluent, EsFluentKv};

#[derive(EsFluent, EsFluentKv)]
#[fluent_kv(keys = ["description", "label"])]
pub enum Country {
    USA(USAState),
    Canada(CanadaProvince),
}

pub struct USAState;
pub struct CanadaProvince;
"#;

        fs::write(&rust_file_path, rust_content).unwrap();

        let result = parse_directory(temp_dir.path());
        assert!(result.is_ok());

        let type_infos = result.unwrap();

        // Should have entries for:
        // 1. Country (from EsFluent)
        // 2. CountryDescriptionKvFtl (from EsFluentKv)
        // 3. CountryLabelKvFtl (from EsFluentKv)
        let type_names: Vec<_> = type_infos.iter().map(|t| t.type_name.clone()).collect();
        println!("Found type_infos: {:?}", type_names);

        assert!(
            type_names.contains(&"Country".to_string()),
            "Should have Country from EsFluent"
        );
        assert!(
            type_names.contains(&"CountryDescriptionKvFtl".to_string()),
            "Should have CountryDescriptionKvFtl from EsFluentKv"
        );
        assert!(
            type_names.contains(&"CountryLabelKvFtl".to_string()),
            "Should have CountryLabelKvFtl from EsFluentKv"
        );
    }
}

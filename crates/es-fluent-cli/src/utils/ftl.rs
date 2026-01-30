//! Shared FTL file operations for CLI commands.

use crate::ftl::{extract_message_keys, parse_ftl_file};
use anyhow::Result;
use fluent_syntax::ast;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Build the path to the main FTL file for a crate in a locale.
pub fn main_ftl_path(assets_dir: &Path, locale: &str, crate_name: &str) -> PathBuf {
    assets_dir.join(locale).join(format!("{}.ftl", crate_name))
}

/// Build a path to the FTL output directory for a locale.
pub fn locale_output_dir(assets_dir: &Path, locale: &str) -> PathBuf {
    assets_dir.join(locale)
}

/// Discover all FTL files for a given locale and crate, including main and namespaced files.
pub fn discover_ftl_files(
    assets_dir: &Path,
    locale: &str,
    crate_name: &str,
) -> Result<Vec<FtlFileInfo>> {
    let mut files = Vec::new();
    let locale_dir = assets_dir.join(locale);

    // Check main FTL file
    let main_file = locale_dir.join(format!("{}.ftl", crate_name));
    if main_file.exists() {
        files.push(FtlFileInfo::new(
            main_file.clone(),
            PathBuf::from(format!("{}.ftl", crate_name)),
        ));
    }

    // Discover namespaced FTL files in subdirectories
    let crate_subdir = locale_dir.join(crate_name);
    if crate_subdir.exists() && crate_subdir.is_dir() {
        discover_ftl_files_recursive(&crate_subdir, &locale_dir, &mut files)?;
    }

    Ok(files)
}

/// Recursively discover FTL files in subdirectories.
fn discover_ftl_files_recursive(
    dir: &Path,
    base_dir: &Path,
    files: &mut Vec<FtlFileInfo>,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Recurse into subdirectories
            discover_ftl_files_recursive(&path, base_dir, files)?;
        } else if path.extension().is_some_and(|e| e == "ftl") {
            // Calculate relative path from base_dir
            let relative_path = path.strip_prefix(base_dir).map_err(|_| {
                anyhow::anyhow!("Failed to calculate relative path for {}", path.display())
            })?;

            files.push(FtlFileInfo::new(path.clone(), relative_path.to_path_buf()));
        }
    }

    Ok(())
}

/// Information about a discovered FTL file.
#[derive(Clone, Debug)]
pub struct FtlFileInfo {
    /// Absolute path to FTL file
    pub abs_path: PathBuf,
    /// Relative path from locale directory (e.g., "crate_name.ftl" or "crate_name/ui.ftl")
    pub relative_path: PathBuf,
}

impl FtlFileInfo {
    /// Create a new FtlFileInfo with absolute and relative paths
    pub fn new(abs_path: PathBuf, relative_path: PathBuf) -> Self {
        Self {
            abs_path,
            relative_path,
        }
    }
}

/// A fully loaded FTL file with content and metadata.
#[derive(Clone, Debug)]
pub struct LoadedFtlFile {
    /// Absolute path to FTL file
    pub abs_path: PathBuf,
    /// Relative path from locale directory
    pub relative_path: PathBuf,
    /// The parsed resource
    pub resource: ast::Resource<String>,
    /// Extracted message keys
    pub keys: HashSet<String>,
}

/// Load and parse FTL files, returning a list of loaded file info.
pub fn load_ftl_files(files: Vec<FtlFileInfo>) -> Result<Vec<LoadedFtlFile>> {
    let mut loaded_files = Vec::new();

    for file_info in files {
        if file_info.abs_path.exists() {
            let resource = parse_ftl_file(&file_info.abs_path)?;
            let keys = extract_message_keys(&resource);

            loaded_files.push(LoadedFtlFile {
                abs_path: file_info.abs_path.clone(),
                relative_path: file_info.relative_path.clone(),
                resource,
                keys,
            });
        }
    }

    Ok(loaded_files)
}

/// Discover and load all FTL files for a locale and crate.
pub fn discover_and_load_ftl_files(
    assets_dir: &Path,
    locale: &str,
    crate_name: &str,
) -> Result<Vec<LoadedFtlFile>> {
    let files = discover_ftl_files(assets_dir, locale, crate_name)?;
    load_ftl_files(files)
}

/// Parse an FTL file and return both the resource and any parse errors.
pub fn parse_ftl_file_with_errors(
    ftl_path: &Path,
) -> Result<(
    fluent_syntax::ast::Resource<String>,
    Vec<fluent_syntax::parser::ParserError>,
)> {
    if !ftl_path.exists() {
        return Ok((
            fluent_syntax::ast::Resource { body: Vec::new() },
            Vec::new(),
        ));
    }

    let content = fs::read_to_string(ftl_path)?;

    if content.trim().is_empty() {
        return Ok((
            fluent_syntax::ast::Resource { body: Vec::new() },
            Vec::new(),
        ));
    }

    match fluent_syntax::parser::parse(content) {
        Ok(res) => Ok((res, Vec::new())),
        Err((res, errors)) => Ok((res, errors)),
    }
}

#[cfg(test)]
mod tests {
    use assert_fs::TempDir;

    use super::*;
    use std::fs;

    #[test]
    fn test_ftl_file_info_new() {
        let abs_path = PathBuf::from("/test/example.ftl");
        let relative_path = PathBuf::from("example.ftl");
        let info = FtlFileInfo::new(abs_path.clone(), relative_path.clone());

        assert_eq!(info.abs_path, abs_path);
        assert_eq!(info.relative_path, relative_path);
    }

    #[test]
    fn test_discover_ftl_files_main_only() {
        let temp_dir = TempDir::new().unwrap();
        let locale_dir = temp_dir.path().join("en");
        fs::create_dir_all(&locale_dir).unwrap();

        // Create main FTL file
        let main_ftl = locale_dir.join("test-crate.ftl");
        fs::write(&main_ftl, "hello = Hello\nworld = World").unwrap();

        let files = discover_ftl_files(temp_dir.path(), "en", "test-crate").unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, PathBuf::from("test-crate.ftl"));
        assert_eq!(files[0].abs_path, main_ftl);
    }

    #[test]
    fn test_discover_ftl_files_with_namespace() {
        let temp_dir = TempDir::new().unwrap();
        let locale_dir = temp_dir.path().join("en");
        let crate_dir = locale_dir.join("test-crate");
        fs::create_dir_all(&crate_dir).unwrap();

        // Create main FTL file
        let main_ftl = locale_dir.join("test-crate.ftl");
        fs::write(&main_ftl, "hello = Hello").unwrap();

        // Create namespaced FTL file
        let namespace_ftl = crate_dir.join("ui.ftl");
        fs::write(&namespace_ftl, "button = Click").unwrap();

        let files = discover_ftl_files(temp_dir.path(), "en", "test-crate").unwrap();

        assert_eq!(files.len(), 2);

        // Check main file
        let main_file = files
            .iter()
            .find(|f| f.relative_path == PathBuf::from("test-crate.ftl"))
            .unwrap();
        assert_eq!(main_file.abs_path, main_ftl);

        // Check namespace file
        let ns_file = files
            .iter()
            .find(|f| f.relative_path == PathBuf::from("test-crate/ui.ftl"))
            .unwrap();
        assert_eq!(ns_file.abs_path, namespace_ftl);
    }

    #[test]
    fn test_discover_and_load_ftl_files() {
        let temp_dir = TempDir::new().unwrap();
        let locale_dir = temp_dir.path().join("en");
        fs::create_dir_all(&locale_dir).unwrap();

        // Create FTL file with content
        let ftl_path = locale_dir.join("test-crate.ftl");
        fs::write(&ftl_path, "hello = Hello { $name }").unwrap();

        let loaded_files =
            discover_and_load_ftl_files(temp_dir.path(), "en", "test-crate").unwrap();

        assert_eq!(loaded_files.len(), 1);
        assert_eq!(
            loaded_files[0].relative_path,
            PathBuf::from("test-crate.ftl")
        );
        assert_eq!(loaded_files[0].abs_path, ftl_path);
        assert!(loaded_files[0].resource.body.len() > 0);
        assert!(loaded_files[0].keys.contains("hello"));
    }
}

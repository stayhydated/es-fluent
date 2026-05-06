//! FTL file layout and discovery utilities.
use anyhow::{Result, anyhow};
use fluent_syntax::ast;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Build the path to the main FTL file for a crate in a locale.
pub fn main_ftl_path(assets_dir: &Path, locale: &str, crate_name: &str) -> PathBuf {
    assets_dir.join(locale).join(format!("{}.ftl", crate_name))
}

/// Build a path to the locale output directory.
pub fn locale_output_dir(assets_dir: &Path, locale: &str) -> PathBuf {
    assets_dir.join(locale)
}

/// Shared file layout for a crate within a locale directory.
#[derive(Clone, Debug)]
pub struct CrateFtlLayout {
    locale_dir: PathBuf,
    crate_name: String,
}

impl CrateFtlLayout {
    /// Create a layout from an assets directory and locale name.
    pub fn from_assets_dir(assets_dir: &Path, locale: &str, crate_name: &str) -> Self {
        Self::new(locale_output_dir(assets_dir, locale), crate_name)
    }

    /// Create a layout directly from a locale directory.
    pub fn new(locale_dir: PathBuf, crate_name: &str) -> Self {
        Self {
            locale_dir,
            crate_name: crate_name.to_string(),
        }
    }

    /// Returns the main file for this crate in the locale.
    pub fn main_file(&self) -> PathBuf {
        self.locale_dir.join(format!("{}.ftl", self.crate_name))
    }

    /// Returns the namespaced crate directory for this locale.
    pub fn crate_dir(&self) -> PathBuf {
        self.locale_dir.join(&self.crate_name)
    }

    /// Discover all FTL files for this crate in the locale.
    pub fn discover_files(&self) -> Result<Vec<FtlFileInfo>> {
        discover_crate_ftl_files_in_locale_dir(&self.locale_dir, &self.crate_name)
    }

    /// Discover and load all FTL files for this crate in the locale.
    pub fn discover_and_load_files(&self) -> Result<Vec<LoadedFtlFile>> {
        load_ftl_files(self.discover_files()?)
    }

    /// Mirror the fallback crate file set into this locale.
    pub fn expected_files_from_fallback(
        &self,
        fallback: &CrateFtlLayout,
    ) -> Result<HashSet<PathBuf>> {
        let mut expected = HashSet::new();

        if fallback.main_file().exists() {
            expected.insert(self.main_file());
        }

        let fallback_crate_dir = fallback.crate_dir();
        if fallback_crate_dir.exists() {
            for fallback_file in
                discover_nested_ftl_files(&fallback_crate_dir, &fallback.locale_dir)?
            {
                expected.insert(self.locale_dir.join(fallback_file.relative_path));
            }
        }

        Ok(expected)
    }
}

/// Information about a discovered FTL file.
#[derive(Clone, Debug)]
pub struct FtlFileInfo {
    /// Absolute path to FTL file.
    pub abs_path: PathBuf,
    /// Relative path from locale directory.
    pub relative_path: PathBuf,
}

impl FtlFileInfo {
    /// Create a new `FtlFileInfo`.
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
    /// Absolute path to FTL file.
    pub abs_path: PathBuf,
    /// Relative path from locale directory.
    pub relative_path: PathBuf,
    /// Parsed resource.
    pub resource: ast::Resource<String>,
    /// Extracted message keys.
    pub keys: HashSet<String>,
}

/// Discover every FTL file under a locale directory.
pub fn discover_locale_ftl_files(locale_dir: &Path) -> Result<Vec<FtlFileInfo>> {
    discover_nested_ftl_files(locale_dir, locale_dir)
}

/// Discover all FTL files for a given locale and crate, including main and namespaced files.
#[cfg(test)]
pub fn discover_ftl_files(
    assets_dir: &Path,
    locale: &str,
    crate_name: &str,
) -> Result<Vec<FtlFileInfo>> {
    CrateFtlLayout::from_assets_dir(assets_dir, locale, crate_name).discover_files()
}

/// Discover all FTL files for a crate within a concrete locale directory.
pub fn discover_crate_ftl_files_in_locale_dir(
    locale_dir: &Path,
    crate_name: &str,
) -> Result<Vec<FtlFileInfo>> {
    let layout = CrateFtlLayout::new(locale_dir.to_path_buf(), crate_name);
    let mut files = Vec::new();

    let main_file = layout.main_file();
    if main_file.exists() {
        files.push(FtlFileInfo::new(
            main_file,
            PathBuf::from(format!("{}.ftl", crate_name)),
        ));
    }

    let crate_subdir = layout.crate_dir();
    if crate_subdir.exists() {
        if !crate_subdir.is_dir() {
            return Err(anyhow!(
                "Expected crate namespace path to be a directory: {}",
                crate_subdir.display()
            ));
        }

        files.extend(discover_nested_ftl_files(&crate_subdir, locale_dir)?);
    }

    Ok(files)
}

/// Recursively discover FTL files under `dir`, returning paths relative to `base_dir`.
pub fn discover_nested_ftl_files(dir: &Path, base_dir: &Path) -> Result<Vec<FtlFileInfo>> {
    let mut files = Vec::new();

    if !dir.exists() {
        return Ok(files);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            files.extend(discover_nested_ftl_files(&path, base_dir)?);
        } else if path.extension().is_some_and(|ext| ext == "ftl") {
            let relative_path = path.strip_prefix(base_dir).map_err(|_| {
                anyhow::anyhow!("Failed to calculate relative path for {}", path.display())
            })?;
            files.push(FtlFileInfo::new(path.clone(), relative_path.to_path_buf()));
        }
    }

    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(files)
}

/// Load and parse FTL files, returning a list of loaded file info.
pub fn load_ftl_files(files: Vec<FtlFileInfo>) -> Result<Vec<LoadedFtlFile>> {
    let mut loaded_files = Vec::new();

    for file_info in files {
        if file_info.abs_path.exists() {
            let resource = crate::ftl::parse_ftl_file(&file_info.abs_path)?;
            let keys = crate::ftl::extract_message_keys(&resource);

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
    CrateFtlLayout::from_assets_dir(assets_dir, locale, crate_name).discover_and_load_files()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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

        let main_ftl = locale_dir.join("test-crate.ftl");
        fs::write(&main_ftl, "hello = Hello").unwrap();

        let namespace_ftl = crate_dir.join("ui.ftl");
        fs::write(&namespace_ftl, "button = Click").unwrap();

        let files = discover_ftl_files(temp_dir.path(), "en", "test-crate").unwrap();

        assert_eq!(files.len(), 2);
        assert!(
            files
                .iter()
                .any(|info| info.relative_path == std::path::Path::new("test-crate.ftl"))
        );
        assert!(
            files
                .iter()
                .any(|info| info.relative_path == std::path::Path::new("test-crate/ui.ftl"))
        );
    }

    #[test]
    fn test_discover_ftl_files_errors_when_namespace_path_is_file() {
        let temp_dir = TempDir::new().unwrap();
        let locale_dir = temp_dir.path().join("en");
        fs::create_dir_all(&locale_dir).unwrap();
        fs::write(locale_dir.join("test-crate"), "not a directory").unwrap();

        let err = discover_ftl_files(temp_dir.path(), "en", "test-crate")
            .expect_err("namespace path file should fail");
        assert!(
            err.to_string()
                .contains("Expected crate namespace path to be a directory"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_discover_locale_ftl_files_recurses() {
        let temp_dir = TempDir::new().unwrap();
        let locale_dir = temp_dir.path().join("en");
        fs::create_dir_all(locale_dir.join("app/forms")).unwrap();
        fs::write(locale_dir.join("app.ftl"), "hello = Hello").unwrap();
        fs::write(locale_dir.join("app/forms/input.ftl"), "input = Input").unwrap();

        let files = discover_locale_ftl_files(&locale_dir).unwrap();
        assert_eq!(files.len(), 2);
        assert!(
            files
                .iter()
                .any(|info| info.relative_path == std::path::Path::new("app.ftl"))
        );
        assert!(
            files
                .iter()
                .any(|info| info.relative_path == std::path::Path::new("app/forms/input.ftl"))
        );
    }

    #[test]
    fn test_discover_and_load_ftl_files() {
        let temp_dir = TempDir::new().unwrap();
        let locale_dir = temp_dir.path().join("en");
        fs::create_dir_all(&locale_dir).unwrap();
        fs::write(
            locale_dir.join("test-crate.ftl"),
            "hello = Hello\nworld = World",
        )
        .unwrap();

        let files = discover_and_load_ftl_files(temp_dir.path(), "en", "test-crate").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].keys.len(), 2);
        assert!(files[0].keys.contains("hello"));
        assert!(files[0].keys.contains("world"));
    }

    #[test]
    fn crate_layout_mirrors_fallback_structure() {
        let temp_dir = TempDir::new().unwrap();
        let fallback = temp_dir.path().join("en");
        let target = temp_dir.path().join("es");
        fs::create_dir_all(fallback.join("test-crate/forms")).unwrap();
        fs::write(fallback.join("test-crate.ftl"), "hello = Hello").unwrap();
        fs::write(fallback.join("test-crate/forms/input.ftl"), "input = Input").unwrap();

        let fallback_layout = CrateFtlLayout::new(fallback, "test-crate");
        let target_layout = CrateFtlLayout::new(target.clone(), "test-crate");
        let expected = target_layout
            .expected_files_from_fallback(&fallback_layout)
            .unwrap();

        assert!(expected.contains(&target.join("test-crate.ftl")));
        assert!(expected.contains(&target.join("test-crate/forms/input.ftl")));
    }
}

use super::inventory::KeyInfo;
use crate::core::{
    CrateInfo, FtlSyntaxError, MissingKeyError, MissingVariableWarning, ValidationIssue,
};
use crate::ftl::LocaleContext;
use crate::ftl::extract_variables_from_message;
use crate::utils::{LoadedFtlFile, discover_and_load_ftl_files, ftl::main_ftl_path};
use anyhow::Result;
use fluent_syntax::ast;
use indexmap::IndexMap;
use miette::{NamedSource, SourceSpan};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use terminal_link::Link;

/// Context for FTL validation to reduce argument count.
struct ValidationContext<'a> {
    expected_keys: &'a IndexMap<String, KeyInfo>,
    workspace_root: &'a Path,
    manifest_dir: &'a Path,
}

impl ValidationContext<'_> {
    fn format_terminal_link(&self, label: &str, url: &str) -> String {
        if crate::utils::ui::terminal_links_enabled() {
            Link::new(label, url).to_string()
        } else {
            label.to_string()
        }
    }

    /// Helper to make a path relative to workspace root.
    fn to_relative_path(&self, path: &Path) -> String {
        // Try to canonicalize both for accurate diffing
        let path_canon = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let base_canon = fs::canonicalize(self.workspace_root)
            .unwrap_or_else(|_| self.workspace_root.to_path_buf());

        // Try to strip prefix
        if let Ok(rel) = path_canon.strip_prefix(&base_canon) {
            return rel.display().to_string();
        }

        // If straightforward strip failed, we can return the path as is or try simple path strip
        // (sometimes canonicalize fails or resolves symlinks unpredictably)
        if let Ok(rel) = path.strip_prefix(self.workspace_root) {
            return rel.display().to_string();
        }

        // Fallback: return absolute path or best effort
        path.display().to_string()
    }

    /// Generate missing key issues when an FTL file doesn't exist.
    fn missing_file_issues(&self, locale: &str, ftl_path: &str) -> Vec<ValidationIssue> {
        self.expected_keys
            .keys()
            .map(|key| {
                ValidationIssue::MissingKey(MissingKeyError {
                    src: NamedSource::new(ftl_path, String::new()),
                    key: key.clone(),
                    locale: locale.to_string(),
                    help: format!("Add translation for '{}' in {}", key, ftl_path),
                })
            })
            .collect()
    }

    /// Validate multiple loaded FTL files against expected keys.
    fn validate_loaded_ftl_files(
        &self,
        loaded_files: Vec<LoadedFtlFile>,
        locale: &str,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let mut all_actual_keys: IndexMap<String, (HashSet<String>, String, String)> =
            IndexMap::new(); // key -> (vars, file_path, header_link)

        // Process all files and collect keys
        for file in loaded_files {
            let _content = fs::read_to_string(&file.abs_path).unwrap_or_default();
            let ftl_relative_path = self.to_relative_path(&file.abs_path);
            let ftl_header_link = self.format_terminal_link(
                &ftl_relative_path,
                &format!("file://{}", file.abs_path.display()),
            );

            // Collect actual keys from this file
            for entry in &file.resource.body {
                if let ast::Entry::Message(msg) = entry {
                    let key = msg.id.name.clone();
                    let vars = extract_variables_from_message(msg);

                    // Store the key with its file info
                    all_actual_keys.insert(
                        key.clone(),
                        (vars, ftl_relative_path.clone(), ftl_header_link.clone()),
                    );
                }
            }
        }

        // Check for missing keys and variables
        for (key, key_info) in self.expected_keys {
            let Some((actual_vars, _file_path, header_link)) = all_actual_keys.get(key) else {
                // Key is missing from all files - report it in the first file as a reasonable default
                let default_file_path =
                    if let Some((_, path, link)) = all_actual_keys.values().next() {
                        (path.clone(), link.clone())
                    } else {
                        // No files at all, this case should be handled earlier but let's provide a fallback
                        (format!("{}.ftl", "unknown"), format!("{}.ftl", "unknown"))
                    };

                issues.push(ValidationIssue::MissingKey(MissingKeyError {
                    src: NamedSource::new(default_file_path.1, String::new()),
                    key: key.clone(),
                    locale: locale.to_string(),
                    help: format!("Add translation for '{}' in {}", key, default_file_path.0),
                }));
                continue;
            };

            // Check for missing variables
            for var in &key_info.variables {
                if actual_vars.contains(var) {
                    continue;
                }

                // Find the span in the actual file (this is approximate)
                let span = SourceSpan::new(0_usize.into(), 1_usize);

                // Build help message with source location if available
                let help = match (&key_info.source_file, key_info.source_line) {
                    (Some(file), Some(line)) => {
                        let file_path = Path::new(file);
                        let abs_file = if file_path.is_absolute() {
                            file_path.to_path_buf()
                        } else {
                            self.manifest_dir.join(file_path)
                        };

                        let rel_file = self.to_relative_path(&abs_file);
                        let file_label = format!("{rel_file}:{line}");
                        let file_url = format!("file://{}", abs_file.display());
                        let file_link = self.format_terminal_link(&file_label, &file_url);

                        format!("Variable '${var}' is declared at {file_link}")
                    },
                    (Some(file), None) => {
                        let file_path = Path::new(file);
                        let abs_file = if file_path.is_absolute() {
                            file_path.to_path_buf()
                        } else {
                            self.manifest_dir.join(file_path)
                        };
                        let rel_file = self.to_relative_path(&abs_file);

                        let file_url = format!("file://{}", abs_file.display());
                        let file_link = self.format_terminal_link(&rel_file, &file_url);

                        format!("Variable '${var}' is declared in {file_link}")
                    },
                    _ => format!("Variable '${var}' is declared in Rust code"),
                };

                issues.push(ValidationIssue::MissingVariable(MissingVariableWarning {
                    src: NamedSource::new(header_link.clone(), String::new()),
                    span,
                    variable: var.clone(),
                    key: key.clone(),
                    locale: locale.to_string(),
                    help,
                }));
            }
        }

        issues
    }
}

/// Validate a single crate's FTL files using already-collected inventory data.
pub(crate) fn validate_crate(
    krate: &CrateInfo,
    workspace_root: &Path,
    temp_dir: &Path,
    check_all: bool,
) -> Result<Vec<ValidationIssue>> {
    // Read the inventory that was already collected in the first pass
    let expected_keys = super::inventory::read_inventory_file(temp_dir, &krate.name)?;

    // Validate FTL files against expected keys
    validate_ftl_files(krate, workspace_root, &expected_keys, check_all)
}

/// Validate FTL files against expected keys using shared discovery logic.
fn validate_ftl_files(
    krate: &CrateInfo,
    workspace_root: &Path,
    expected_keys: &IndexMap<String, KeyInfo>,
    check_all: bool,
) -> Result<Vec<ValidationIssue>> {
    let locale_ctx = LocaleContext::from_crate(krate, check_all)?;
    let assets_dir = &locale_ctx.assets_dir;
    let ctx = ValidationContext {
        expected_keys,
        workspace_root,
        manifest_dir: &krate.manifest_dir,
    };

    let mut issues = Vec::new();

    for locale in &locale_ctx.locales {
        // Use shared discovery and loading logic
        match discover_and_load_ftl_files(assets_dir, locale, &locale_ctx.crate_name) {
            Ok(loaded_files) => {
                if loaded_files.is_empty() {
                    // No FTL files found at all - treat as missing main file
                    let ftl_abs_path = main_ftl_path(assets_dir, locale, &locale_ctx.crate_name);
                    let ftl_relative_path = ctx.to_relative_path(&ftl_abs_path);
                    let ftl_header_link = ctx.format_terminal_link(
                        &ftl_relative_path,
                        &format!("file://{}", ftl_abs_path.display()),
                    );

                    issues.extend(ctx.missing_file_issues(locale, &ftl_header_link));
                    continue;
                }

                issues.extend(ctx.validate_loaded_ftl_files(loaded_files, locale));
            },
            Err(e) => {
                // Handle discovery/loading errors
                let ftl_abs_path = main_ftl_path(assets_dir, locale, &locale_ctx.crate_name);
                let ftl_relative_path = ctx.to_relative_path(&ftl_abs_path);
                let ftl_header_link = ctx.format_terminal_link(
                    &ftl_relative_path,
                    &format!("file://{}", ftl_abs_path.display()),
                );

                issues.push(ValidationIssue::SyntaxError(FtlSyntaxError {
                    src: NamedSource::new(ftl_header_link, String::new()),
                    span: SourceSpan::new(0_usize.into(), 1_usize),
                    locale: locale.clone(),
                    help: format!("Failed to discover FTL files: {}", e),
                }));
            },
        }
    }

    Ok(issues)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::LoadedFtlFile;
    use indexmap::IndexMap;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use tempfile::tempdir;

    fn key_info(vars: &[&str], source_file: Option<&str>, source_line: Option<u32>) -> KeyInfo {
        KeyInfo {
            variables: vars.iter().map(|v| v.to_string()).collect(),
            source_file: source_file.map(ToString::to_string),
            source_line,
        }
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn missing_file_issues_returns_issue_for_each_expected_key() {
        let mut expected_keys = IndexMap::new();
        expected_keys.insert("first".to_string(), key_info(&[], None, None));
        expected_keys.insert("second".to_string(), key_info(&[], None, None));

        let temp = tempdir().unwrap();
        let ctx = ValidationContext {
            expected_keys: &expected_keys,
            workspace_root: temp.path(),
            manifest_dir: temp.path(),
        };

        let issues = ctx.missing_file_issues("en", "i18n/en/test-app.ftl");
        assert_eq!(issues.len(), 2);
        assert!(
            issues.iter().any(
                |issue| matches!(issue, ValidationIssue::MissingKey(err) if err.key == "first")
            )
        );
        assert!(
            issues.iter().any(
                |issue| matches!(issue, ValidationIssue::MissingKey(err) if err.key == "second")
            )
        );
    }

    #[test]
    fn validate_loaded_ftl_files_reports_missing_key_and_variable() {
        let temp = tempdir().unwrap();
        let ftl_path = temp.path().join("i18n/en/test-app.ftl");
        fs::create_dir_all(ftl_path.parent().unwrap()).unwrap();
        fs::write(&ftl_path, "hello = Hello\n").unwrap();

        let resource = fluent_syntax::parser::parse("hello = Hello\n".to_string()).unwrap();
        let loaded_files = vec![LoadedFtlFile {
            abs_path: ftl_path.clone(),
            relative_path: PathBuf::from("test-app.ftl"),
            resource,
            keys: ["hello".to_string()].into_iter().collect(),
        }];

        let mut expected_keys = IndexMap::new();
        expected_keys.insert(
            "hello".to_string(),
            key_info(&["name"], Some("src/lib.rs"), Some(7)),
        );
        expected_keys.insert("goodbye".to_string(), key_info(&[], None, None));

        let ctx = ValidationContext {
            expected_keys: &expected_keys,
            workspace_root: temp.path(),
            manifest_dir: temp.path(),
        };

        let issues = ctx.validate_loaded_ftl_files(loaded_files, "en");
        assert!(issues.iter().any(|issue| {
            matches!(
                issue,
                ValidationIssue::MissingVariable(warning)
                    if warning.key == "hello" && warning.variable == "name"
            )
        }));
        assert!(issues.iter().any(
            |issue| matches!(issue, ValidationIssue::MissingKey(err) if err.key == "goodbye")
        ));
    }

    #[test]
    fn validate_crate_reports_missing_main_file_as_missing_key() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .unwrap();

        let inventory_path =
            es_fluent_derive_core::get_metadata_inventory_path(temp.path(), "test-crate");
        fs::create_dir_all(inventory_path.parent().unwrap()).unwrap();
        fs::write(
            &inventory_path,
            r#"{
  "expected_keys": [
    {
      "key": "hello",
      "variables": [],
      "source_file": null,
      "source_line": null
    }
  ]
}"#,
        )
        .unwrap();

        let krate = CrateInfo {
            name: "test-crate".to_string(),
            manifest_dir: temp.path().to_path_buf(),
            src_dir: temp.path().join("src"),
            i18n_config_path: temp.path().join("i18n.toml"),
            ftl_output_dir: temp.path().join("i18n/en"),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        };

        let issues = validate_crate(&krate, temp.path(), temp.path(), false).unwrap();
        assert_eq!(issues.len(), 1);
        assert!(
            issues.iter().any(
                |issue| matches!(issue, ValidationIssue::MissingKey(err) if err.key == "hello")
            )
        );
    }

    #[test]
    fn validate_loaded_ftl_files_handles_source_file_variants_and_terminal_links() {
        let _env_guard = env_lock().lock().unwrap();
        // SAFETY: Serialized via env_lock mutex in this test module.
        unsafe {
            std::env::set_var("FORCE_HYPERLINK", "1");
        }

        let temp = tempdir().unwrap();
        let ftl_path = temp.path().join("i18n/en/test-app.ftl");
        fs::create_dir_all(ftl_path.parent().unwrap()).unwrap();
        fs::write(&ftl_path, "hello = Hello\nbye = Bye\nraw = Raw\n").unwrap();

        let resource =
            fluent_syntax::parser::parse("hello = Hello\nbye = Bye\nraw = Raw\n".to_string())
                .unwrap();
        let loaded_files = vec![LoadedFtlFile {
            abs_path: ftl_path,
            relative_path: PathBuf::from("test-app.ftl"),
            resource,
            keys: ["hello".to_string(), "bye".to_string(), "raw".to_string()]
                .into_iter()
                .collect(),
        }];

        let mut expected_keys = IndexMap::new();
        expected_keys.insert(
            "hello".to_string(),
            key_info(
                &["name"],
                Some(temp.path().join("src/lib.rs").to_string_lossy().as_ref()),
                Some(7),
            ),
        );
        expected_keys.insert(
            "bye".to_string(),
            key_info(&["who"], Some("src/lib.rs"), None),
        );
        expected_keys.insert("raw".to_string(), key_info(&["value"], None, None));

        let ctx = ValidationContext {
            expected_keys: &expected_keys,
            workspace_root: temp.path(),
            manifest_dir: temp.path(),
        };

        let issues = ctx.validate_loaded_ftl_files(loaded_files, "en");

        assert!(issues.iter().any(|issue| {
            matches!(
                issue,
                ValidationIssue::MissingVariable(warning)
                    if warning.key == "hello" && warning.help.contains("src/lib.rs:7")
            )
        }));
        assert!(issues.iter().any(|issue| {
            matches!(
                issue,
                ValidationIssue::MissingVariable(warning)
                    if warning.key == "bye" && warning.help.contains("declared in")
            )
        }));
        assert!(issues.iter().any(|issue| {
            matches!(
                issue,
                ValidationIssue::MissingVariable(warning)
                    if warning.key == "raw" && warning.help.contains("Rust code")
            )
        }));
    }

    #[test]
    fn validate_loaded_ftl_files_falls_back_to_unknown_when_no_actual_files() {
        let temp = tempdir().unwrap();
        let mut expected_keys = IndexMap::new();
        expected_keys.insert("missing".to_string(), key_info(&[], None, None));

        let ctx = ValidationContext {
            expected_keys: &expected_keys,
            workspace_root: temp.path(),
            manifest_dir: temp.path(),
        };

        let issues = ctx.validate_loaded_ftl_files(Vec::new(), "en");
        assert_eq!(issues.len(), 1);
        assert!(matches!(
            &issues[0],
            ValidationIssue::MissingKey(err) if err.key == "missing"
        ));
    }

    #[cfg(unix)]
    #[test]
    fn validate_ftl_files_reports_syntax_issue_when_discovery_errors() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempdir().unwrap();
        let src_dir = temp.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").unwrap();
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .unwrap();

        let broken_dir = temp.path().join("i18n/en/test-crate");
        fs::create_dir_all(&broken_dir).unwrap();
        let mut perms = fs::metadata(&broken_dir).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&broken_dir, perms).unwrap();

        let krate = CrateInfo {
            name: "test-crate".to_string(),
            manifest_dir: temp.path().to_path_buf(),
            src_dir,
            i18n_config_path: temp.path().join("i18n.toml"),
            ftl_output_dir: temp.path().join("i18n/en"),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        };

        let issues = validate_ftl_files(&krate, temp.path(), &IndexMap::new(), false).unwrap();

        let mut restore = fs::metadata(&broken_dir).unwrap().permissions();
        restore.set_mode(0o755);
        fs::set_permissions(&broken_dir, restore).unwrap();

        assert!(issues.iter().any(|issue| {
            matches!(issue, ValidationIssue::SyntaxError(err) if err.help.contains("Failed to discover FTL files"))
        }));
    }

    #[cfg(unix)]
    #[test]
    fn to_relative_path_uses_non_canonical_strip_fallback() {
        use std::os::unix::fs::symlink;

        let temp = tempdir().unwrap();
        let real_root = temp.path().join("workspace-real");
        fs::create_dir_all(&real_root).unwrap();
        let symlink_root = temp.path().join("workspace-link");
        symlink(&real_root, &symlink_root).unwrap();

        let expected_keys = IndexMap::new();
        let ctx = ValidationContext {
            expected_keys: &expected_keys,
            workspace_root: &symlink_root,
            manifest_dir: &symlink_root,
        };

        let virtual_path = symlink_root.join("i18n/en/missing.ftl");
        let rel = ctx.to_relative_path(&virtual_path);
        assert_eq!(rel, "i18n/en/missing.ftl");

        let outside = temp.path().join("outside.ftl");
        let outside_rel = ctx.to_relative_path(&outside);
        assert_eq!(outside_rel, outside.display().to_string());
    }
}

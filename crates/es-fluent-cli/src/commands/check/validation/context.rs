use super::super::inventory::KeyInfo;
use crate::core::{
    DuplicateKeyError, FtlSyntaxError, MissingKeyError, MissingVariableWarning,
    UnexpectedVariableError, ValidationIssue,
};
use indexmap::IndexMap;
use miette::{NamedSource, SourceSpan};
use std::fs;
use std::path::Path;
use terminal_link::Link;

pub(super) struct ValidationContext<'a> {
    pub(super) expected_keys: &'a IndexMap<String, KeyInfo>,
    pub(super) workspace_root: &'a Path,
    pub(super) manifest_dir: &'a Path,
}

impl ValidationContext<'_> {
    pub(super) fn format_terminal_link(&self, label: &str, url: &str) -> String {
        if crate::utils::ui::Ui::terminal_links_enabled() {
            Link::new(label, url).to_string()
        } else {
            label.to_string()
        }
    }

    pub(super) fn to_relative_path(&self, path: &Path) -> String {
        let path_canon = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let base_canon = fs::canonicalize(self.workspace_root)
            .unwrap_or_else(|_| self.workspace_root.to_path_buf());

        if let Ok(rel) = path_canon.strip_prefix(&base_canon) {
            return rel.display().to_string();
        }

        if let Ok(rel) = path.strip_prefix(self.workspace_root) {
            return rel.display().to_string();
        }

        path.display().to_string()
    }

    pub(super) fn missing_file_issues(&self, locale: &str, ftl_path: &str) -> Vec<ValidationIssue> {
        self.expected_keys
            .keys()
            .cloned()
            .map(|key| {
                let help = format!("Add translation for '{}' in {}", key, ftl_path);
                ValidationIssue::MissingKey(MissingKeyError {
                    src: NamedSource::new(ftl_path, String::new()),
                    key,
                    locale: locale.to_string(),
                    help,
                })
            })
            .collect()
    }

    pub(super) fn missing_key_issue(
        &self,
        key: &str,
        locale: &str,
        file_path: &str,
        header_link: &str,
    ) -> ValidationIssue {
        ValidationIssue::MissingKey(MissingKeyError {
            src: NamedSource::new(header_link, String::new()),
            key: key.to_string(),
            locale: locale.to_string(),
            help: format!("Add translation for '{}' in {}", key, file_path),
        })
    }

    pub(super) fn missing_variable_issue(
        &self,
        key: &str,
        variable: &str,
        locale: &str,
        header_link: &str,
        source_file: Option<&str>,
        source_line: Option<u32>,
    ) -> ValidationIssue {
        ValidationIssue::MissingVariable(MissingVariableWarning {
            src: NamedSource::new(header_link, String::new()),
            span: SourceSpan::new(0_usize.into(), 1_usize),
            variable: variable.to_string(),
            key: key.to_string(),
            locale: locale.to_string(),
            help: self.missing_variable_help(variable, source_file, source_line),
        })
    }

    pub(super) fn unexpected_variable_issue(
        &self,
        key: &str,
        variable: &str,
        locale: &str,
        header_link: &str,
    ) -> ValidationIssue {
        ValidationIssue::UnexpectedVariable(UnexpectedVariableError {
            src: NamedSource::new(header_link, String::new()),
            span: SourceSpan::new(0_usize.into(), 1_usize),
            variable: variable.to_string(),
            key: key.to_string(),
            locale: locale.to_string(),
            help: format!("Remove variable '${variable}' from '{key}' or declare it in Rust code"),
        })
    }

    pub(super) fn duplicate_key_issue(
        &self,
        key: &str,
        locale: &str,
        first_file: &str,
        duplicate_file: &str,
        duplicate_header_link: &str,
    ) -> ValidationIssue {
        ValidationIssue::DuplicateKey(DuplicateKeyError {
            src: NamedSource::new(duplicate_header_link, String::new()),
            span: SourceSpan::new(0_usize.into(), 1_usize),
            key: key.to_string(),
            locale: locale.to_string(),
            first_file: first_file.to_string(),
            duplicate_file: duplicate_file.to_string(),
            help: format!(
                "Remove one definition of '{}' from either {} or {}",
                key, first_file, duplicate_file
            ),
        })
    }

    pub(super) fn syntax_error_issue(
        &self,
        locale: &str,
        file_path: &Path,
        help: String,
    ) -> ValidationIssue {
        let relative_path = self.to_relative_path(file_path);
        let header_link =
            self.format_terminal_link(&relative_path, &format!("file://{}", file_path.display()));

        ValidationIssue::SyntaxError(FtlSyntaxError {
            src: NamedSource::new(header_link, String::new()),
            span: SourceSpan::new(0_usize.into(), 1_usize),
            locale: locale.to_string(),
            help,
        })
    }

    fn missing_variable_help(
        &self,
        variable: &str,
        source_file: Option<&str>,
        source_line: Option<u32>,
    ) -> String {
        match (source_file, source_line) {
            (Some(file), Some(line)) => {
                let abs_file = self.absolute_source_path(file);
                let rel_file = self.to_relative_path(&abs_file);
                let file_label = format!("{rel_file}:{line}");
                let file_url = format!("file://{}", abs_file.display());
                let file_link = self.format_terminal_link(&file_label, &file_url);
                format!("Variable '${variable}' is declared at {file_link}")
            },
            (Some(file), None) => {
                let abs_file = self.absolute_source_path(file);
                let rel_file = self.to_relative_path(&abs_file);
                let file_url = format!("file://{}", abs_file.display());
                let file_link = self.format_terminal_link(&rel_file, &file_url);
                format!("Variable '${variable}' is declared in {file_link}")
            },
            _ => format!("Variable '${variable}' is declared in Rust code"),
        }
    }

    fn absolute_source_path(&self, file: &str) -> std::path::PathBuf {
        let file_path = Path::new(file);
        if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            self.manifest_dir.join(file_path)
        }
    }
}

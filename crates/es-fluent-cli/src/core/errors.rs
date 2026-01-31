//! CLI error types using miette for beautiful Rust-style diagnostics.
//!
//! These error types provide clippy-style error messages with source snippets,
//! labels, and helpful suggestions.

// Fields in these structs are read by miette's Diagnostic derive macro
#![allow(unused)]

use miette::{Diagnostic, NamedSource, SourceSpan};
use std::path::PathBuf;
use thiserror::Error;

/// Error when the i18n.toml configuration file is not found.
#[derive(Debug, Diagnostic, Error)]
#[error("i18n.toml configuration file not found")]
#[diagnostic(
    code(es_fluent::config::not_found),
    help(
        "Create an i18n.toml file in your crate root with the following content:\n\n  \
          fallback_language = \"en\"\n  \
          assets_dir = \"i18n\"\n"
    )
)]
pub struct ConfigNotFoundError {
    /// The path where the config was expected.
    pub expected_path: PathBuf,
}

/// Error when parsing the i18n.toml configuration file.
#[derive(Debug, Diagnostic, Error)]
#[error("failed to parse i18n.toml configuration")]
#[diagnostic(code(es_fluent::config::parse_error))]
pub struct ConfigParseError {
    /// The source content of the config file.
    #[source_code]
    pub src: NamedSource<String>,

    /// The span where the error occurred.
    #[label("error occurred here")]
    pub span: Option<SourceSpan>,

    /// The underlying parse error message.
    #[help]
    pub help: String,
}

/// Error when the assets directory doesn't exist.
#[derive(Debug, Diagnostic, Error)]
#[error("assets directory not found: {path}")]
#[diagnostic(
    code(es_fluent::config::assets_not_found),
    help("Create the assets directory or update assets_dir in i18n.toml")
)]
pub struct AssetsNotFoundError {
    /// The path that was expected.
    pub path: PathBuf,
}

/// Error when the fallback language directory doesn't exist.
#[derive(Debug, Diagnostic, Error)]
#[error("fallback language directory not found: {language}")]
#[diagnostic(
    code(es_fluent::config::fallback_not_found),
    help("Create a directory named '{language}' in your assets folder")
)]
pub struct FallbackLanguageNotFoundError {
    /// The fallback language.
    pub language: String,
}

/// Error when a language identifier is invalid.
#[derive(Debug, Diagnostic, Error)]
#[error("invalid language identifier: {identifier}")]
#[diagnostic(
    code(es_fluent::config::invalid_language),
    help("Use a valid BCP 47 language tag (e.g., 'en', 'en-US', 'zh-Hans')")
)]
pub struct InvalidLanguageError {
    /// The invalid language identifier.
    pub identifier: String,
}

/// Error when a specified locale doesn't exist.
#[derive(Debug, Diagnostic, Error)]
#[error("locale '{locale}' not found")]
#[diagnostic(
    code(es_fluent::config::locale_not_found),
    help("Available locales: {available}")
)]
pub struct LocaleNotFoundError {
    /// The locale that was specified but not found.
    pub locale: String,
    /// Comma-separated list of available locales.
    pub available: String,
}

/// A single missing key diagnostic.
#[derive(Debug, Diagnostic, Error)]
#[error("missing translation key")]
#[diagnostic(code(es_fluent::validate::missing_key), severity(Error))]
pub struct MissingKeyError {
    /// The source content of the FTL file.
    #[source_code]
    pub src: NamedSource<String>,

    /// The key that is missing.
    pub key: String,

    /// The locale where the key is missing.
    pub locale: String,

    /// Help text.
    #[help]
    pub help: String,
}

/// A single missing variable diagnostic (warning).
#[derive(Debug, Diagnostic, Error)]
#[error("translation omits variable")]
#[diagnostic(code(es_fluent::validate::missing_variable), severity(Warning))]
pub struct MissingVariableWarning {
    /// The source content of the FTL file.
    #[source_code]
    pub src: NamedSource<String>,

    /// The span where the message is defined.
    #[label("this message omits variable '${variable}'")]
    pub span: SourceSpan,

    /// The variable that is missing.
    pub variable: String,

    /// The key containing the issue.
    pub key: String,

    /// The locale where the issue exists.
    pub locale: String,

    /// Help text.
    #[help]
    pub help: String,
}

/// Error when an FTL file has syntax errors.
#[derive(Debug, Diagnostic, Error)]
#[error("FTL syntax error")]
#[diagnostic(code(es_fluent::validate::syntax_error))]
pub struct FtlSyntaxError {
    /// The source content of the FTL file.
    #[source_code]
    pub src: NamedSource<String>,

    /// The span where the error occurred.
    #[label("syntax error here")]
    pub span: SourceSpan,

    /// The locale.
    pub locale: String,

    /// Help text.
    #[help]
    pub help: String,
}

/// Aggregated validation report containing multiple issues.
#[derive(Debug, Diagnostic, Error)]
#[error("validation found {error_count} error(s) and {warning_count} warning(s)")]
#[diagnostic(code(es_fluent::validate::report))]
pub struct ValidationReport {
    /// Number of errors found.
    pub error_count: usize,

    /// Number of warnings found.
    pub warning_count: usize,

    /// Related diagnostics (missing keys, missing variables, etc.).
    #[related]
    pub issues: Vec<ValidationIssue>,
}

/// A validation issue (either error or warning).
#[derive(Debug, Diagnostic, Error)]
pub enum ValidationIssue {
    #[error(transparent)]
    #[diagnostic(transparent)]
    MissingKey(#[from] MissingKeyError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    MissingVariable(#[from] MissingVariableWarning),

    #[error(transparent)]
    #[diagnostic(transparent)]
    SyntaxError(#[from] FtlSyntaxError),
}

impl ValidationIssue {
    /// Get a sort key for deterministic ordering of issues.
    ///
    /// The key includes:
    /// 1. File path (from source name)
    /// 2. Issue type priority (SyntaxError > MissingKey > MissingVariable)
    /// 3. Key/Variable name
    pub fn sort_key(&self) -> String {
        match self {
            ValidationIssue::SyntaxError(e) => {
                format!("1:{:?}", e.src.name())
            },
            ValidationIssue::MissingKey(e) => {
                format!("2:{:?}:{}", e.src.name(), e.key)
            },
            ValidationIssue::MissingVariable(e) => {
                format!("3:{:?}:{}:{}", e.src.name(), e.key, e.variable)
            },
        }
    }
}

/// Error when formatting fails for an FTL file.
#[derive(Debug, Diagnostic, Error)]
#[error("failed to format {path}")]
#[diagnostic(code(es_fluent::format::failed))]
pub struct FormatError {
    /// The path to the file.
    pub path: PathBuf,

    /// The underlying error.
    #[help]
    pub help: String,
}

/// Report for format command results.
#[derive(Debug, Diagnostic, Error)]
#[error("formatted {formatted_count} file(s), {error_count} error(s)")]
#[diagnostic(code(es_fluent::format::report))]
pub struct FormatReport {
    /// Number of files formatted.
    pub formatted_count: usize,

    /// Number of errors.
    pub error_count: usize,

    /// Related format errors.
    #[related]
    pub errors: Vec<FormatError>,
}

/// Warning when a key needs to be synced to another locale.
#[derive(Debug, Diagnostic, Error)]
#[error("missing translation for key '{key}' in locale '{target_locale}'")]
#[diagnostic(code(es_fluent::sync::missing), severity(Warning))]
pub struct SyncMissingKey {
    /// The key that is missing.
    pub key: String,

    /// The target locale where the key is missing.
    pub target_locale: String,

    /// The source locale (fallback).
    pub source_locale: String,
}

/// Report for sync command results.
#[derive(Debug, Diagnostic, Error)]
#[error("sync: added {added_count} key(s) to {locale_count} locale(s)")]
#[diagnostic(code(es_fluent::sync::report))]
pub struct SyncReport {
    /// Number of keys added.
    pub added_count: usize,

    /// Number of locales affected.
    pub locale_count: usize,

    /// Keys that were synced.
    #[related]
    pub synced_keys: Vec<SyncMissingKey>,
}

#[derive(Debug, Diagnostic, Error)]
pub enum CliError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    ConfigNotFound(#[from] ConfigNotFoundError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    ConfigParse(#[from] ConfigParseError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    AssetsNotFound(#[from] AssetsNotFoundError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    FallbackNotFound(#[from] FallbackLanguageNotFoundError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    InvalidLanguage(#[from] InvalidLanguageError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    LocaleNotFound(#[from] LocaleNotFoundError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Validation(#[from] ValidationReport),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Format(#[from] FormatReport),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Sync(#[from] SyncReport),

    #[error("IO error: {0}")]
    #[diagnostic(code(es_fluent::io))]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    #[diagnostic(code(es_fluent::other))]
    Other(String),
}

impl From<anyhow::Error> for CliError {
    fn from(err: anyhow::Error) -> Self {
        CliError::Other(err.to_string())
    }
}

/// Calculate line and column from byte offset in source text.
pub fn line_col_from_offset(source: &str, offset: usize) -> (usize, usize) {
    let mut current_offset = 0;
    for (i, line) in source.lines().enumerate() {
        let line_len = line.len() + 1; // +1 for newline
        if current_offset + line_len > offset {
            let col = offset - current_offset + 1;
            return (i + 1, col);
        }
        current_offset += line_len;
    }
    (source.lines().count().max(1), 1)
}

/// Calculate SourceSpan from line and column in source text.
#[allow(dead_code)]
pub fn span_from_line_col(source: &str, line: usize, col: usize, len: usize) -> SourceSpan {
    let mut offset = 0;
    for (i, line_content) in source.lines().enumerate() {
        if i + 1 == line {
            offset += col.saturating_sub(1);
            break;
        }
        offset += line_content.len() + 1; // +1 for newline
    }
    SourceSpan::new(offset.into(), len)
}

/// Find the byte offset and length of a key in the FTL source.
#[allow(dead_code)]
pub fn find_key_span(source: &str, key: &str) -> Option<SourceSpan> {
    // Look for the key at the start of a line (message definition)
    for (line_idx, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(key)
            && (rest.starts_with(" =") || rest.starts_with('='))
        {
            // Found the key
            let line_start: usize = source.lines().take(line_idx).map(|l| l.len() + 1).sum();
            let key_start = line_start + (line.len() - trimmed.len());
            return Some(SourceSpan::new(key_start.into(), key.len()));
        }
    }
    None
}

/// Find all lines containing a message definition for a key.
#[allow(dead_code)]
pub fn find_message_span(source: &str, key: &str) -> Option<SourceSpan> {
    let mut in_message = false;
    let mut start_offset = 0;
    let mut current_offset = 0;

    for line in source.lines() {
        let trimmed = line.trim_start();

        if let Some(rest) = trimmed.strip_prefix(key) {
            if rest.starts_with(" =") || rest.starts_with('=') {
                in_message = true;
                start_offset = current_offset + (line.len() - trimmed.len());
            }
        } else if in_message {
            // Check if this is a continuation line (starts with whitespace) or a new entry
            if !line.starts_with(' ') && !line.starts_with('\t') && !trimmed.is_empty() {
                // End of message
                let end_offset = current_offset;
                return Some(SourceSpan::new(
                    start_offset.into(),
                    end_offset - start_offset,
                ));
            }
        }

        current_offset += line.len() + 1; // +1 for newline
    }

    // If we're still in a message at EOF
    if in_message {
        return Some(SourceSpan::new(
            start_offset.into(),
            current_offset.saturating_sub(1) - start_offset,
        ));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_key_span() {
        let source = r#"## Comment
hello = Hello
world = World"#;
        let span = find_key_span(source, "hello").unwrap();
        assert_eq!(span.offset(), 11);
        assert_eq!(span.len(), 5);
    }

    #[test]
    fn test_find_key_span_with_spaces() {
        let source = r#"hello =Hello
world = World"#;
        let span = find_key_span(source, "hello").unwrap();
        assert_eq!(span.offset(), 0);
        assert_eq!(span.len(), 5);
    }

    #[test]
    fn test_find_message_span_multiline() {
        let source = r#"greeting = Hello
    World
next = Next"#;
        let span = find_message_span(source, "greeting").unwrap();
        assert_eq!(span.offset(), 0);
        // Should include the full multiline message
    }

    #[test]
    fn test_span_from_line_col() {
        let source = r#"line1
line2
line3"#;
        let span = span_from_line_col(source, 2, 1, 5);
        assert_eq!(span.offset(), 6); // "line1\n" = 6 chars
        assert_eq!(span.len(), 5);
    }
}

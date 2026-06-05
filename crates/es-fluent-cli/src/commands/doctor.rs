//! Doctor command implementation.

use super::common::{OutputFormat, WorkspaceArgs, WorkspaceCrates};
use crate::core::{CliError, CrateInfo};
use crate::ftl::LocaleContext;
use clap::Parser;
use fs_err as fs;
use serde::Serialize;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use toml_edit::{DocumentMut, Item};

/// Arguments for the doctor command.
#[derive(Debug, Parser)]
pub struct DoctorArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::default())]
    pub output: OutputFormat,
}

#[derive(Serialize)]
struct DoctorReport {
    crates_discovered: usize,
    error_count: usize,
    warning_count: usize,
    issues: Vec<DoctorIssue>,
}

#[derive(Debug, Serialize)]
struct DoctorIssue {
    severity: &'static str,
    crate_name: Option<String>,
    message: String,
    help: String,
}

/// Run the doctor command.
pub fn run_doctor(args: DoctorArgs) -> Result<(), CliError> {
    let output = args.output;
    let workspace = match WorkspaceCrates::discover(args.workspace) {
        Ok(workspace) => workspace,
        Err(error) => {
            if output.is_json() {
                output.print_json(&DoctorReport {
                    crates_discovered: 0,
                    error_count: 1,
                    warning_count: 0,
                    issues: vec![DoctorIssue {
                        severity: "error",
                        crate_name: None,
                        message: "workspace could not be inspected".to_string(),
                        help: error.to_string(),
                    }],
                })?;
                return Err(CliError::Exit(1));
            }
            return Err(error);
        },
    };
    let mut issues = Vec::new();

    if workspace.crates.is_empty() {
        if let Some(package) = workspace.package_not_found() {
            issues.push(DoctorIssue {
                severity: "warning",
                crate_name: None,
                message: format!("no configured crate found matching package filter '{package}'"),
                help: "Check the package name or omit --package to inspect every configured crate."
                    .to_string(),
            });
        } else {
            issues.push(DoctorIssue {
                severity: "warning",
                crate_name: None,
                message: "no crates with i18n.toml were found".to_string(),
                help: "Run `cargo es-fluent init` in a crate that should use es-fluent."
                    .to_string(),
            });
        }
    }

    for krate in &workspace.skipped {
        issues.push(DoctorIssue {
            severity: "error",
            crate_name: Some(krate.name.to_string()),
            message: "crate has i18n.toml but no library target".to_string(),
            help: "Add a Cargo library target, such as src/lib.rs or a [lib] path in Cargo.toml."
                .to_string(),
        });
    }

    for krate in &workspace.crates {
        inspect_crate(krate, &mut issues);
    }

    if output.is_json() {
        normalize_doctor_issue_help(&mut issues, &workspace.workspace_info.root_dir);
    }

    let error_count = issues
        .iter()
        .filter(|issue| issue.severity == "error")
        .count();
    let warning_count = issues
        .iter()
        .filter(|issue| issue.severity == "warning")
        .count();
    let report = DoctorReport {
        crates_discovered: workspace.crates.len(),
        error_count,
        warning_count,
        issues,
    };

    if output.is_json() {
        output.print_json(&report)?;
    } else {
        print_doctor_report(&report);
    }

    if report.error_count > 0 {
        Err(CliError::Exit(1))
    } else {
        Ok(())
    }
}

fn inspect_crate(krate: &CrateInfo, issues: &mut Vec<DoctorIssue>) {
    match LocaleContext::from_crate(krate, true) {
        Ok(ctx) => {
            let fallback_dir = ctx.locale_dir(&ctx.fallback);
            let fallback_path_invalid = !crate::ftl::is_real_locale_directory(&fallback_dir);
            if fallback_path_invalid {
                issues.push(DoctorIssue {
                    severity: "error",
                    crate_name: Some(krate.name.to_string()),
                    message: format!(
                        "fallback locale directory '{}' is missing or not a directory",
                        ctx.fallback
                    ),
                    help: format!("Create {}", fallback_dir.display()),
                });
            }
            inspect_locale_path_entries(krate, &ctx, issues, fallback_path_invalid);
        },
        Err(error) => {
            let help = error.to_string();
            issues.push(DoctorIssue {
                severity: "error",
                crate_name: Some(krate.name.to_string()),
                message: locale_context_error_message(&help).to_string(),
                help,
            });
        },
    }

    if !krate.has_lib_rs {
        return;
    }

    let manifest_path = krate.manifest_dir.join("Cargo.toml");
    let i18n_module_path = krate.src_dir.join("i18n.rs");
    let manifest = fs::read_to_string(&manifest_path).unwrap_or_default();
    let Some(library_target_path) = inspect_library_target_path(krate, &manifest, issues) else {
        return;
    };
    let library_target_source = fs::read_to_string(&library_target_path).unwrap_or_default();
    let active_library_target_source = active_rust_source_text(&library_target_source);
    let library_target_defines_i18n_module =
        contains_macro_invocation(&active_library_target_source, "define_i18n_module");
    let i18n_module_is_real_file = inspect_i18n_module_path(
        krate,
        &i18n_module_path,
        !library_target_defines_i18n_module,
        issues,
    );
    inspect_library_i18n_module_declaration(
        krate,
        &library_target_path,
        i18n_module_is_real_file,
        library_target_defines_i18n_module,
        issues,
    );
    let (active_i18n_module, active_i18n_module_path) = if i18n_module_is_real_file {
        (
            active_rust_source_text(&fs::read_to_string(&i18n_module_path).unwrap_or_default()),
            i18n_module_path.as_path(),
        )
    } else if library_target_defines_i18n_module {
        (active_library_target_source, library_target_path.as_path())
    } else {
        (String::new(), i18n_module_path.as_path())
    };

    if let Some(manager_dependency) = manager_dependency_from_module(&active_i18n_module)
        && !manifest_has_dependency(&manifest, "dependencies", manager_dependency)
    {
        let module_path = relative_to_crate(krate, active_i18n_module_path);
        issues.push(DoctorIssue {
            severity: "warning",
            crate_name: Some(krate.name.to_string()),
            message: format!(
                "{module_path} references {manager_dependency}, but Cargo.toml does not"
            ),
            help: format!("Add `{manager_dependency}` under [dependencies]."),
        });
    }

    if i18n_module_is_real_file
        && !contains_macro_invocation(&active_i18n_module, "define_i18n_module")
    {
        let module_path = relative_to_crate(krate, &i18n_module_path);
        issues.push(DoctorIssue {
            severity: "warning",
            crate_name: Some(krate.name.to_string()),
            message: format!("{module_path} does not invoke `define_i18n_module!()`"),
            help: format!(
                "Add the manager macro call to {module_path}, such as `es_fluent_manager_embedded::define_i18n_module!();`."
            ),
        });
    }

    inspect_build_script(krate, &manifest, &active_i18n_module, issues);
}

fn locale_context_error_message(help: &str) -> &'static str {
    if help.contains("Locale directory") && help.contains("must use canonical BCP-47 form") {
        "locale assets contain a non-canonical locale directory name"
    } else if help.contains("Invalid language identifier")
        || help.contains("could not be parsed as an ICU locale")
    {
        "locale assets contain an invalid locale directory name"
    } else {
        "i18n.toml or locale assets could not be read"
    }
}

fn inspect_library_target_path(
    krate: &CrateInfo,
    manifest: &str,
    issues: &mut Vec<DoctorIssue>,
) -> Option<PathBuf> {
    let raw_path = manifest_library_path_value(manifest);
    let raw_path = raw_path.as_deref().unwrap_or("src/lib.rs");
    let normalized = match normalize_library_target_path(raw_path) {
        Ok(path) => path,
        Err(reason) => {
            issues.push(DoctorIssue {
                severity: "error",
                crate_name: Some(krate.name.to_string()),
                message: "library target path is invalid".to_string(),
                help: format!("Cargo.toml [lib].path {reason}: {raw_path}"),
            });
            return None;
        },
    };
    let abs_path = krate.manifest_dir.join(&normalized);

    if let Some(reason) =
        library_target_existing_path_violation(krate.manifest_dir.as_path(), &normalized)
    {
        issues.push(DoctorIssue {
            severity: "error",
            crate_name: Some(krate.name.to_string()),
            message: "library target path is not a real in-crate source path".to_string(),
            help: format!(
                "{} uses {reason}; replace it with real in-crate directories and a real Rust source file.",
                relative_to_crate(krate, &abs_path)
            ),
        });
        return None;
    }

    Some(abs_path)
}

fn inspect_library_i18n_module_declaration(
    krate: &CrateInfo,
    library_target_path: &Path,
    module_file_exists: bool,
    library_target_defines_i18n_module: bool,
    issues: &mut Vec<DoctorIssue>,
) {
    let library_path = relative_to_crate(krate, library_target_path);
    let module_path = relative_to_crate(krate, &krate.src_dir.join("i18n.rs"));
    let source = match fs::read_to_string(library_target_path) {
        Ok(source) => source,
        Err(error) => {
            issues.push(DoctorIssue {
                severity: "error",
                crate_name: Some(krate.name.to_string()),
                message: format!("{library_path} could not be read"),
                help: error.to_string(),
            });
            return;
        },
    };

    match super::init::i18n_module_declaration_kind_in_source(&source) {
        Some(super::init::I18nModuleDeclaration::External(
            super::init::I18nModuleVisibility::Public,
        )) => {},
        Some(super::init::I18nModuleDeclaration::External(
            super::init::I18nModuleVisibility::Restricted,
        )) => issues.push(DoctorIssue {
            severity: "error",
            crate_name: Some(krate.name.to_string()),
            message: format!("{library_path} declares module `i18n` without public visibility"),
            help: format!("Change the declaration in {library_path} to `pub mod i18n;`."),
        }),
        Some(super::init::I18nModuleDeclaration::Inline(_)) => issues.push(DoctorIssue {
            severity: "error",
            crate_name: Some(krate.name.to_string()),
            message: format!("{library_path} defines inline module `i18n`"),
            help: format!(
                "Move the inline module to i18n.rs or remove it, then declare `pub mod i18n;` from {library_path}."
            ),
        }),
        None if module_file_exists => issues.push(DoctorIssue {
            severity: "error",
            crate_name: Some(krate.name.to_string()),
            message: if library_target_defines_i18n_module {
                format!(
                    "{module_path} exists but {library_path} directly calls `define_i18n_module!()`"
                )
            } else {
                format!("{library_path} does not declare module `i18n`")
            },
            help: if library_target_defines_i18n_module {
                format!(
                    "Remove {module_path}, or move the macro there and declare it from {library_path} with `pub mod i18n;`."
                )
            } else {
                format!(
                    "Add `pub mod i18n;` to {library_path} so the generated i18n module is compiled."
                )
            },
        }),
        None => {},
    }
}

fn manifest_library_path_value(manifest: &str) -> Option<String> {
    manifest.parse::<DocumentMut>().ok().and_then(|manifest| {
        manifest
            .as_table()
            .get("lib")
            .and_then(Item::as_table_like)
            .and_then(|lib| lib.get("path"))
            .and_then(Item::as_value)
            .and_then(|value| value.as_str().map(str::to_string))
    })
}

fn normalize_library_target_path(raw_path: &str) -> Result<PathBuf, &'static str> {
    let path = Path::new(raw_path);
    if path.is_absolute() {
        return Err("must be relative to the crate root");
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {},
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err("must stay inside the crate root");
                }
            },
            Component::Prefix(_) | Component::RootDir => {
                return Err("must be relative to the crate root");
            },
        }
    }

    if normalized.as_os_str().is_empty() {
        return Err("must point to a library source file");
    }

    Ok(normalized)
}

fn library_target_existing_path_violation(root: &Path, lib_path: &Path) -> Option<&'static str> {
    let mut current = root.to_path_buf();
    for component in lib_path.components() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Some("a symlinked library target path component");
            },
            Ok(_) => {},
            Err(error)
                if matches!(error.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) =>
            {
                return None;
            },
            Err(_) => return None,
        }
    }

    if existing_components_escape_root(root, &root.join(lib_path)) {
        return Some("existing path components outside the crate root");
    }

    None
}

fn existing_components_escape_root(root: &Path, path: &Path) -> bool {
    let Ok(root) = root.canonicalize() else {
        return false;
    };
    let Some(existing_ancestor) = path.ancestors().find(|ancestor| ancestor.exists()) else {
        return false;
    };
    let Ok(existing_ancestor) = existing_ancestor.canonicalize() else {
        return false;
    };

    !existing_ancestor.starts_with(root)
}

fn normalize_doctor_issue_help(issues: &mut [DoctorIssue], workspace_root: &Path) {
    for issue in issues {
        issue.help = relative_doctor_message(&issue.help, workspace_root);
    }
}

fn relative_doctor_message(message: &str, base: &Path) -> String {
    let base_canon = std::fs::canonicalize(base).unwrap_or_else(|_| base.to_path_buf());
    let base_canon = base_canon.display().to_string();
    let base = base.display().to_string();
    let mut normalized = replace_doctor_path_prefix(message, &base_canon);
    if base != base_canon {
        normalized = replace_doctor_path_prefix(&normalized, &base);
    }
    normalized
}

fn replace_doctor_path_prefix(message: &str, base: &str) -> String {
    if base.is_empty() {
        return message.to_string();
    }

    let slash_prefix = format!("{base}/");
    let separator_prefix = format!("{base}{}", std::path::MAIN_SEPARATOR);
    message
        .replace(&slash_prefix, "")
        .replace(&separator_prefix, "")
}

fn inspect_i18n_module_path(
    krate: &CrateInfo,
    i18n_module_path: &std::path::Path,
    warn_when_missing: bool,
    issues: &mut Vec<DoctorIssue>,
) -> bool {
    let module_path = relative_to_crate(krate, i18n_module_path);

    match fs::symlink_metadata(i18n_module_path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            issues.push(DoctorIssue {
                severity: "error",
                crate_name: Some(krate.name.to_string()),
                message: format!("{module_path} is a symlink"),
                help: format!("Replace it with a real Rust module file at {module_path}."),
            });
            return false;
        },
        Ok(metadata) if metadata.is_file() => return true,
        Ok(_) => {
            issues.push(DoctorIssue {
                severity: "error",
                crate_name: Some(krate.name.to_string()),
                message: format!("{module_path} exists but is not a file"),
                help: format!("Replace it with a Rust module file at {module_path}."),
            });
            return false;
        },
        Err(error) if error.kind() == ErrorKind::NotFound => {},
        Err(error) => {
            issues.push(DoctorIssue {
                severity: "error",
                crate_name: Some(krate.name.to_string()),
                message: format!("{module_path} could not be inspected"),
                help: error.to_string(),
            });
            return false;
        },
    }

    if warn_when_missing {
        issues.push(DoctorIssue {
            severity: "warning",
            crate_name: Some(krate.name.to_string()),
            message: format!("{module_path} is missing"),
            help: format!(
                "Run `cargo es-fluent init` or create {module_path} with `define_i18n_module!()`."
            ),
        });
    }
    false
}

fn relative_to_crate(krate: &CrateInfo, path: &std::path::Path) -> String {
    path.strip_prefix(krate.manifest_dir.as_path())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn inspect_locale_path_entries(
    krate: &CrateInfo,
    ctx: &LocaleContext,
    issues: &mut Vec<DoctorIssue>,
    fallback_path_invalid: bool,
) {
    let locale_path_issues = match crate::ftl::locale_named_non_directory_paths(&ctx.assets_dir) {
        Ok(issues) => issues,
        Err(error) => {
            issues.push(DoctorIssue {
                severity: "error",
                crate_name: Some(krate.name.to_string()),
                message: "locale assets could not be read".to_string(),
                help: error.to_string(),
            });
            return;
        },
    };

    for issue in locale_path_issues {
        if fallback_path_invalid && issue.locale == ctx.fallback {
            continue;
        }

        issues.push(DoctorIssue {
            severity: "error",
            crate_name: Some(krate.name.to_string()),
            message: format!("locale path '{}' is not a directory", issue.locale),
            help: format!(
                "Remove the file or replace it with a directory: {}",
                issue.path.display()
            ),
        });
    }

    for locale in &ctx.locales {
        let locale_dir = ctx.locale_dir(locale);
        if !crate::ftl::is_real_locale_directory(&locale_dir) {
            continue;
        }

        if let Err(error) =
            crate::ftl::CrateFtlLayout::from_assets_dir(&ctx.assets_dir, locale, &ctx.crate_name)
                .discover_files()
        {
            issues.push(DoctorIssue {
                severity: "error",
                crate_name: Some(krate.name.to_string()),
                message: "FTL file layout could not be read".to_string(),
                help: error.to_string(),
            });
        }
    }
}

fn manager_dependency_from_module(module: &str) -> Option<&'static str> {
    if contains_manager_crate_reference(module, "es_fluent_manager_embedded") {
        Some("es-fluent-manager-embedded")
    } else if contains_manager_crate_reference(module, "es_fluent_manager_dioxus") {
        Some("es-fluent-manager-dioxus")
    } else if contains_manager_crate_reference(module, "es_fluent_manager_bevy") {
        Some("es-fluent-manager-bevy")
    } else {
        None
    }
}

fn contains_manager_crate_reference(source: &str, segment: &str) -> bool {
    contains_rust_path_segment(source, segment)
        || contains_use_root_import(source, segment)
        || contains_extern_crate_import(source, segment)
}

fn contains_rust_path_segment(source: &str, segment: &str) -> bool {
    let mut search_start = 0;
    while let Some(offset) = source[search_start..].find(segment) {
        let start = search_start + offset;
        let end = start + segment.len();
        if rust_path_segment_boundary_before(source, start)
            && rust_path_segment_boundary_after(source, end)
        {
            return true;
        }
        search_start = end;
    }

    false
}

fn contains_use_root_import(source: &str, segment: &str) -> bool {
    let bytes = source.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if !starts_rust_identifier(bytes, index, b"use") {
            index += 1;
            continue;
        }

        let mut next = skip_ascii_whitespace(bytes, index + b"use".len());
        if bytes.get(next..next + 2) == Some(b"::") {
            next += 2;
            next = skip_ascii_whitespace(bytes, next);
        }

        if starts_rust_identifier(bytes, next, segment.as_bytes()) {
            let after_segment = skip_ascii_whitespace(bytes, next + segment.len());
            if bytes.get(after_segment) == Some(&b';')
                || bytes.get(after_segment..after_segment + 2) == Some(b"::")
                || starts_rust_identifier(bytes, after_segment, b"as")
            {
                return true;
            }
        }

        index += b"use".len();
    }

    false
}

fn contains_extern_crate_import(source: &str, segment: &str) -> bool {
    let bytes = source.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if !starts_rust_identifier(bytes, index, b"extern") {
            index += 1;
            continue;
        }

        let mut next = skip_ascii_whitespace(bytes, index + b"extern".len());
        if !starts_rust_identifier(bytes, next, b"crate") {
            index += b"extern".len();
            continue;
        }

        next = skip_ascii_whitespace(bytes, next + b"crate".len());
        if starts_rust_identifier(bytes, next, segment.as_bytes()) {
            let after_segment = skip_ascii_whitespace(bytes, next + segment.len());
            if bytes.get(after_segment) == Some(&b';')
                || starts_rust_identifier(bytes, after_segment, b"as")
            {
                return true;
            }
        }

        index += b"extern".len();
    }

    false
}

fn starts_rust_identifier(bytes: &[u8], index: usize, ident: &[u8]) -> bool {
    bytes.get(index..index + ident.len()) == Some(ident)
        && !bytes
            .get(index.wrapping_sub(1))
            .is_some_and(|byte| is_rust_identifier_continue(*byte))
        && !bytes
            .get(index + ident.len())
            .is_some_and(|byte| is_rust_identifier_continue(*byte))
}

fn skip_ascii_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while bytes
        .get(index)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        index += 1;
    }
    index
}

fn rust_path_segment_boundary_before(source: &str, start: usize) -> bool {
    source[..start]
        .chars()
        .next_back()
        .is_none_or(|ch| !is_rust_identifier_continue_char(ch))
}

fn rust_path_segment_boundary_after(source: &str, end: usize) -> bool {
    let rest = &source[end..];
    rest.trim_start().starts_with("::")
}

fn manifest_has_dependency(manifest: &str, section: &str, dependency: &str) -> bool {
    manifest
        .parse::<DocumentMut>()
        .ok()
        .and_then(|manifest| {
            manifest
                .as_table()
                .get(section)
                .and_then(Item::as_table_like)
                .map(|dependencies| dependencies.contains_key(dependency))
        })
        .unwrap_or(false)
}

fn inspect_build_script(
    krate: &CrateInfo,
    manifest: &str,
    i18n_module: &str,
    issues: &mut Vec<DoctorIssue>,
) {
    if !contains_macro_invocation(i18n_module, "define_i18n_module") {
        return;
    }

    let build_rs = krate.manifest_dir.join("build.rs");
    match fs::symlink_metadata(&build_rs) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            issues.push(DoctorIssue {
                severity: "error",
                crate_name: Some(krate.name.to_string()),
                message: "build.rs is a symlink".to_string(),
                help: "Replace build.rs with a real Rust build script file that calls `es_fluent_build::track_i18n_assets();`, and add `es-fluent-build` under [build-dependencies].".to_string(),
            });
            return;
        },
        Ok(metadata) if metadata.is_file() => {},
        Ok(_) => {
            issues.push(DoctorIssue {
                severity: "error",
                crate_name: Some(krate.name.to_string()),
                message: "build.rs exists but is not a file".to_string(),
                help: "Replace build.rs with a Rust build script file that calls `es_fluent_build::track_i18n_assets();`, and add `es-fluent-build` under [build-dependencies].".to_string(),
            });
            return;
        },
        Err(error) if error.kind() == ErrorKind::NotFound => {
            issues.push(DoctorIssue {
                severity: "warning",
                crate_name: Some(krate.name.to_string()),
                message: "build.rs does not track locale asset changes".to_string(),
                help: "Create build.rs with `fn main() { es_fluent_build::track_i18n_assets(); }`, and add `es-fluent-build` under [build-dependencies].".to_string(),
            });
            return;
        },
        Err(error) => {
            issues.push(DoctorIssue {
                severity: "error",
                crate_name: Some(krate.name.to_string()),
                message: "build.rs could not be inspected".to_string(),
                help: error.to_string(),
            });
            return;
        },
    }

    let build_rs_contents = fs::read_to_string(&build_rs).unwrap_or_default();
    if !super::init::build_rs_contains_active_tracking_call(&build_rs_contents) {
        issues.push(DoctorIssue {
            severity: "warning",
            crate_name: Some(krate.name.to_string()),
            message: "build.rs exists but does not call track_i18n_assets".to_string(),
            help: "Call `es_fluent_build::track_i18n_assets();` from build.rs, and add `es-fluent-build` under [build-dependencies].".to_string(),
        });
    } else if !manifest_has_dependency(manifest, "build-dependencies", "es-fluent-build") {
        issues.push(DoctorIssue {
            severity: "warning",
            crate_name: Some(krate.name.to_string()),
            message: "build.rs calls track_i18n_assets but Cargo.toml lacks es-fluent-build"
                .to_string(),
            help: "Add `es-fluent-build` under [build-dependencies].".to_string(),
        });
    }
}

pub(crate) fn active_rust_source_text(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut output = String::with_capacity(source.len());
    let mut index = 0;

    while index < bytes.len() {
        if let Some(next) = skip_rust_comment_or_literal(bytes, index) {
            output.extend(std::iter::repeat_n(' ', next.saturating_sub(index)));
            index = next;
            continue;
        }

        output.push(bytes[index] as char);
        index += 1;
    }

    output
}

pub(crate) fn contains_macro_invocation(source: &str, macro_name: &str) -> bool {
    let bytes = source.as_bytes();
    let macro_bytes = macro_name.as_bytes();
    let mut index = 0;

    while index + macro_bytes.len() <= bytes.len() {
        if bytes[index..].starts_with(macro_bytes)
            && !bytes
                .get(index.wrapping_sub(1))
                .is_some_and(|byte| is_rust_identifier_continue(*byte))
        {
            let after_name = index + macro_bytes.len();
            if !bytes
                .get(after_name)
                .is_some_and(|byte| is_rust_identifier_continue(*byte))
                && next_non_whitespace_byte(bytes, after_name) == Some(b'!')
            {
                return true;
            }
        }

        index += 1;
    }

    false
}

fn next_non_whitespace_byte(bytes: &[u8], mut index: usize) -> Option<u8> {
    while index < bytes.len() {
        if !bytes[index].is_ascii_whitespace() {
            return Some(bytes[index]);
        }
        index += 1;
    }

    None
}

fn is_rust_identifier_continue(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

fn is_rust_identifier_continue_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn skip_rust_comment_or_literal(bytes: &[u8], index: usize) -> Option<usize> {
    skip_rust_comment(bytes, index)
        .or_else(|| skip_rust_raw_string(bytes, index))
        .or_else(|| skip_rust_quoted_literal(bytes, index))
}

fn skip_rust_comment(bytes: &[u8], index: usize) -> Option<usize> {
    if bytes.get(index..index + 2) == Some(b"//") {
        return Some(
            bytes[index..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(bytes.len(), |offset| index + offset + 1),
        );
    }

    if bytes.get(index..index + 2) != Some(b"/*") {
        return None;
    }

    let mut depth = 1usize;
    let mut next = index + 2;
    while next + 1 < bytes.len() {
        match &bytes[next..next + 2] {
            b"/*" => {
                depth += 1;
                next += 2;
            },
            b"*/" => {
                depth -= 1;
                next += 2;
                if depth == 0 {
                    return Some(next);
                }
            },
            _ => next += 1,
        }
    }

    Some(bytes.len())
}

fn skip_rust_raw_string(bytes: &[u8], index: usize) -> Option<usize> {
    let mut next = index;
    if bytes.get(next) == Some(&b'b') {
        next += 1;
    }
    if bytes.get(next) != Some(&b'r') {
        return None;
    }
    next += 1;

    let hash_start = next;
    while bytes.get(next) == Some(&b'#') {
        next += 1;
    }
    if bytes.get(next) != Some(&b'"') {
        return None;
    }
    next += 1;

    let hash_count = next - hash_start - 1;
    while next < bytes.len() {
        if bytes[next] == b'"'
            && next + 1 + hash_count <= bytes.len()
            && bytes[next + 1..next + 1 + hash_count]
                .iter()
                .all(|byte| *byte == b'#')
        {
            return Some(next + 1 + hash_count);
        }
        next += 1;
    }

    Some(bytes.len())
}

fn skip_rust_quoted_literal(bytes: &[u8], index: usize) -> Option<usize> {
    let quote_index = match bytes.get(index..index + 2) {
        Some(b"b\"") => index + 1,
        _ if bytes.get(index) == Some(&b'"') || bytes.get(index) == Some(&b'\'') => index,
        _ => return None,
    };
    let quote = bytes[quote_index];
    let mut next = quote_index + 1;
    let mut escaped = false;

    while next < bytes.len() {
        let byte = bytes[next];
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == quote {
            return Some(next + 1);
        } else if quote == b'\'' && byte == b'\n' {
            return None;
        }
        next += 1;
    }

    Some(bytes.len())
}

fn print_doctor_report(report: &DoctorReport) {
    println!("Fluent FTL Doctor");
    if report.issues.is_empty() {
        println!("No setup issues found.");
        return;
    }

    for issue in &report.issues {
        let crate_label = issue
            .crate_name
            .as_deref()
            .map(|name| format!("{name}: "))
            .unwrap_or_default();
        println!(
            "{}: {}{}",
            issue.severity.to_uppercase(),
            crate_label,
            issue.message
        );
        println!("  help: {}", issue.help);
    }

    println!(
        "{} error(s), {} warning(s)",
        report.error_count, report.warning_count
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::common::WorkspaceArgs;
    use fs_err as fs;

    #[test]
    fn run_doctor_succeeds_for_basic_workspace() {
        let temp = crate::test_fixtures::create_test_crate_workspace();

        let result = run_doctor(DoctorArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            output: OutputFormat::Text,
        });

        assert!(result.is_ok());
    }

    #[test]
    fn run_doctor_fails_when_fallback_locale_directory_is_missing() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");

        let result = run_doctor(DoctorArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            output: OutputFormat::Text,
        });

        assert!(matches!(result, Err(CliError::Exit(1))));
    }

    #[test]
    fn run_doctor_fails_when_fallback_locale_path_is_file() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");
        fs::write(temp.path().join("i18n/en"), "not a directory\n").expect("write fallback file");

        let result = run_doctor(DoctorArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            output: OutputFormat::Text,
        });

        assert!(matches!(result, Err(CliError::Exit(1))));

        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("discover workspace");
        let mut issues = Vec::new();
        inspect_crate(&workspace.crates[0], &mut issues);

        assert_eq!(
            issues
                .iter()
                .filter(|issue| issue
                    .message
                    .contains("fallback locale directory 'en' is missing or not a directory"))
                .count(),
            1
        );
        assert!(
            !issues
                .iter()
                .any(|issue| issue.message == "locale path 'en' is not a directory")
        );
    }

    #[cfg(unix)]
    #[test]
    fn run_doctor_fails_when_fallback_locale_path_is_symlink() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        let outside = tempfile::tempdir().expect("outside tempdir");
        fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");
        fs::create_dir_all(outside.path().join("en")).expect("create outside locale");
        std::os::unix::fs::symlink(outside.path().join("en"), temp.path().join("i18n/en"))
            .expect("create fallback locale symlink");

        let result = run_doctor(DoctorArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            output: OutputFormat::Text,
        });

        assert!(matches!(result, Err(CliError::Exit(1))));

        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("discover workspace");
        let mut issues = Vec::new();
        inspect_crate(&workspace.crates[0], &mut issues);

        assert_eq!(
            issues
                .iter()
                .filter(|issue| issue
                    .message
                    .contains("fallback locale directory 'en' is missing or not a directory"))
                .count(),
            1
        );
        assert!(
            !issues
                .iter()
                .any(|issue| issue.message == "locale path 'en' is not a directory")
        );
    }

    #[test]
    fn run_doctor_reports_noncanonical_locale_directory_in_message() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::create_dir_all(temp.path().join("i18n/en-us")).expect("create noncanonical locale");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("discover workspace");
        let mut issues = Vec::new();

        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(issues.iter().any(|issue| {
            issue.severity == "error"
                && issue
                    .message
                    .contains("non-canonical locale directory name")
                && issue.help.contains("en-US")
        }));
        assert!(!issues.iter().any(|issue| {
            issue.message == "i18n.toml or locale assets could not be read"
                && issue.help.contains("en-us")
        }));
    }

    #[test]
    fn run_doctor_reports_invalid_locale_directory_in_message() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::create_dir_all(temp.path().join("i18n/not_a_locale")).expect("create invalid locale");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("discover workspace");
        let mut issues = Vec::new();

        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(issues.iter().any(|issue| {
            issue.severity == "error"
                && issue.message.contains("invalid locale directory name")
                && issue.help.contains("not_a_locale")
        }));
        assert!(!issues.iter().any(|issue| {
            issue.message == "i18n.toml or locale assets could not be read"
                && issue.help.contains("not_a_locale")
        }));
    }

    #[cfg(unix)]
    #[test]
    fn run_doctor_reports_symlinked_i18n_module_path() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        let outside = tempfile::tempdir().expect("outside tempdir");
        fs::write(
            outside.path().join("i18n.rs"),
            "es_fluent_manager_embedded::define_i18n_module!();\n",
        )
        .expect("write outside i18n module");
        std::os::unix::fs::symlink(
            outside.path().join("i18n.rs"),
            temp.path().join("src/i18n.rs"),
        )
        .expect("create i18n module symlink");

        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("discover workspace");
        let mut issues = Vec::new();
        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(issues.iter().any(|issue| {
            issue.severity == "error"
                && issue.message.contains("src/i18n.rs is a symlink")
                && issue.help.contains("real Rust module file")
        }));
    }

    #[cfg(unix)]
    #[test]
    fn run_doctor_reports_symlinked_build_rs_path() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        let outside = tempfile::tempdir().expect("outside tempdir");
        fs::write(
            temp.path().join("src/i18n.rs"),
            "es_fluent_manager_embedded::define_i18n_module!();\n",
        )
        .expect("write i18n module");
        fs::write(
            outside.path().join("build.rs"),
            "fn main() { es_fluent_build::track_i18n_assets(); }\n",
        )
        .expect("write outside build script");
        std::os::unix::fs::symlink(
            outside.path().join("build.rs"),
            temp.path().join("build.rs"),
        )
        .expect("create build.rs symlink");

        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("discover workspace");
        let mut issues = Vec::new();
        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(issues.iter().any(|issue| {
            issue.severity == "error"
                && issue.message == "build.rs is a symlink"
                && issue.help.contains("real Rust build script file")
        }));
    }

    #[cfg(unix)]
    #[test]
    fn run_doctor_reports_symlinked_default_library_target_path() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        let outside = tempfile::tempdir().expect("outside tempdir");
        fs::remove_file(temp.path().join("src/lib.rs")).expect("remove real lib");
        fs::write(outside.path().join("lib.rs"), "pub mod i18n;\n").expect("write outside lib");
        std::os::unix::fs::symlink(
            outside.path().join("lib.rs"),
            temp.path().join("src/lib.rs"),
        )
        .expect("create lib.rs symlink");

        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("discover workspace");
        let mut issues = Vec::new();
        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(issues.iter().any(|issue| {
            issue.severity == "error"
                && issue.message == "library target path is not a real in-crate source path"
                && issue.help.contains("src/lib.rs")
                && issue.help.contains("symlinked library target")
        }));
    }

    #[cfg(unix)]
    #[test]
    fn run_doctor_reports_symlinked_custom_library_parent_path() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        let outside = tempfile::tempdir().expect("outside tempdir");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"linked/lib.rs\"\n",
        )
        .expect("write custom manifest");
        fs::create_dir_all(outside.path().join("libsrc")).expect("create outside lib src");
        fs::write(outside.path().join("libsrc/lib.rs"), "pub mod i18n;\n")
            .expect("write outside lib");
        std::os::unix::fs::symlink(outside.path().join("libsrc"), temp.path().join("linked"))
            .expect("create custom lib parent symlink");

        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("discover workspace");
        let mut issues = Vec::new();
        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(issues.iter().any(|issue| {
            issue.severity == "error"
                && issue.message == "library target path is not a real in-crate source path"
                && issue.help.contains("linked/lib.rs")
                && issue.help.contains("symlinked library target")
        }));
    }

    #[test]
    fn run_doctor_fails_when_locale_named_asset_path_is_file() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");

        let result = run_doctor(DoctorArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            output: OutputFormat::Text,
        });

        assert!(matches!(result, Err(CliError::Exit(1))));
    }

    #[test]
    fn run_doctor_fails_when_ftl_path_is_directory() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::remove_file(temp.path().join("i18n/en/test-app.ftl")).expect("remove fallback ftl");
        fs::create_dir_all(temp.path().join("i18n/en/test-app.ftl")).expect("create ftl directory");

        let result = run_doctor(DoctorArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            output: OutputFormat::Text,
        });

        assert!(matches!(result, Err(CliError::Exit(1))));
    }

    #[test]
    fn run_doctor_ignores_non_locale_files_in_assets_dir() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(temp.path().join("i18n/README.md"), "notes\n").expect("write notes file");

        let result = run_doctor(DoctorArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            output: OutputFormat::Text,
        });

        assert!(result.is_ok());
    }

    #[test]
    fn normalize_doctor_issue_help_strips_workspace_paths_for_json() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut issues = vec![DoctorIssue {
            severity: "error",
            crate_name: Some("test-app".to_string()),
            message: "locale path is invalid".to_string(),
            help: format!(
                "Remove the file or replace it with a directory: {}",
                temp.path().join("i18n/fr").display()
            ),
        }];

        normalize_doctor_issue_help(&mut issues, temp.path());

        assert_eq!(
            issues[0].help,
            "Remove the file or replace it with a directory: i18n/fr"
        );
    }

    #[test]
    fn run_doctor_warns_for_manager_dependency_mismatch_without_failing() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(temp.path().join("src/lib.rs"), "pub mod i18n;\n").expect("declare i18n module");
        fs::write(
            temp.path().join("src/i18n.rs"),
            "es_fluent_manager_embedded::define_i18n_module!();\n",
        )
        .expect("write i18n module without matching manifest dependency");

        let result = run_doctor(DoctorArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            output: OutputFormat::Text,
        });

        assert!(result.is_ok(), "warnings should not fail doctor");
    }

    #[test]
    fn run_doctor_warns_for_aliased_manager_dependency_mismatch() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(temp.path().join("src/lib.rs"), "pub mod i18n;\n").expect("declare i18n module");
        fs::write(
            temp.path().join("src/i18n.rs"),
            "use es_fluent_manager_embedded as manager;\nmanager::define_i18n_module!();\n",
        )
        .expect("write aliased i18n module without matching manifest dependency");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("workspace should discover crate");
        let mut issues = Vec::new();

        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(
            issues.iter().any(|issue| {
                issue
                    .message
                    .contains("references es-fluent-manager-embedded")
            }),
            "aliased manager imports should still be checked against Cargo.toml"
        );
    }

    #[test]
    fn run_doctor_warns_when_i18n_module_is_missing() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("workspace should discover crate");
        let mut issues = Vec::new();

        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(issues.iter().any(|issue| {
            issue.severity == "warning"
                && issue.message.contains("src/i18n.rs is missing")
                && issue.help.contains("cargo es-fluent init")
        }));
    }

    #[test]
    fn run_doctor_accepts_direct_library_i18n_module_macro() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nes-fluent-manager-embedded = \"0.16\"\n\n[build-dependencies]\nes-fluent-build = \"0.16\"\n",
        )
        .expect("write manifest");
        fs::write(
            temp.path().join("src/lib.rs"),
            "es_fluent_manager_embedded::define_i18n_module!();\npub struct Demo;\n",
        )
        .expect("write direct macro lib");
        fs::write(
            temp.path().join("build.rs"),
            "fn main() { es_fluent_build::track_i18n_assets(); }\n",
        )
        .expect("write tracked build script");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("workspace should discover crate");
        let mut issues = Vec::new();

        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(
            issues.iter().all(|issue| {
                !issue.message.contains("src/i18n.rs is missing")
                    && !issue.message.contains("build.rs")
            }),
            "direct library macro with build tracking should be doctor-clean: {issues:?}"
        );
    }

    #[test]
    fn run_doctor_checks_build_tracking_for_direct_library_i18n_module_macro() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nes-fluent-manager-embedded = \"0.16\"\n",
        )
        .expect("write manifest");
        fs::write(
            temp.path().join("src/lib.rs"),
            "es_fluent_manager_embedded::define_i18n_module!();\npub struct Demo;\n",
        )
        .expect("write direct macro lib");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("workspace should discover crate");
        let mut issues = Vec::new();

        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(issues.iter().any(|issue| {
            issue.severity == "warning"
                && issue
                    .message
                    .contains("build.rs does not track locale asset changes")
        }));
        assert!(
            issues
                .iter()
                .all(|issue| !issue.message.contains("src/i18n.rs is missing")),
            "direct library macro should not also warn about missing src/i18n.rs: {issues:?}"
        );
    }

    #[test]
    fn run_doctor_reports_stale_i18n_module_file_for_direct_library_macro() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(
            temp.path().join("src/lib.rs"),
            "es_fluent_manager_embedded::define_i18n_module!();\npub struct Demo;\n",
        )
        .expect("write direct macro lib");
        fs::write(
            temp.path().join("src/i18n.rs"),
            "es_fluent_manager_embedded::define_i18n_module!();\n",
        )
        .expect("write stale i18n module");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("workspace should discover crate");
        let mut issues = Vec::new();

        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(issues.iter().any(|issue| {
            issue.severity == "error"
                && issue.message.contains("src/i18n.rs exists")
                && issue
                    .message
                    .contains("src/lib.rs directly calls `define_i18n_module!()`")
                && issue.help.contains("Remove src/i18n.rs")
        }));
    }

    #[test]
    fn run_doctor_errors_when_i18n_module_file_is_not_declared_from_library() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(
            temp.path().join("src/i18n.rs"),
            "es_fluent_manager_embedded::define_i18n_module!();\n",
        )
        .expect("write i18n module");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("workspace should discover crate");
        let mut issues = Vec::new();

        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(issues.iter().any(|issue| {
            issue.severity == "error"
                && issue
                    .message
                    .contains("src/lib.rs does not declare module `i18n`")
                && issue.help.contains("pub mod i18n;")
        }));
    }

    #[test]
    fn run_doctor_warns_when_declared_i18n_module_lacks_manager_macro() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(temp.path().join("src/lib.rs"), "pub mod i18n;\n").expect("declare i18n module");
        fs::write(temp.path().join("src/i18n.rs"), "pub fn placeholder() {}\n")
            .expect("write i18n module without manager macro");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("workspace should discover crate");
        let mut issues = Vec::new();

        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(issues.iter().any(|issue| {
            issue.severity == "warning"
                && issue
                    .message
                    .contains("src/i18n.rs does not invoke `define_i18n_module!()`")
                && issue.help.contains("es_fluent_manager_embedded")
        }));
    }

    #[test]
    fn run_doctor_errors_when_i18n_module_declaration_is_not_public() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(temp.path().join("src/lib.rs"), "mod i18n;\n").expect("write private module");
        fs::write(
            temp.path().join("src/i18n.rs"),
            "es_fluent_manager_embedded::define_i18n_module!();\n",
        )
        .expect("write i18n module");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("workspace should discover crate");
        let mut issues = Vec::new();

        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(issues.iter().any(|issue| {
            issue.severity == "error"
                && issue
                    .message
                    .contains("src/lib.rs declares module `i18n` without public visibility")
                && issue.help.contains("pub mod i18n;")
        }));
    }

    #[test]
    fn run_doctor_errors_when_library_defines_inline_i18n_module() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(temp.path().join("src/lib.rs"), "pub mod i18n {}\n")
            .expect("write inline module");
        fs::write(
            temp.path().join("src/i18n.rs"),
            "es_fluent_manager_embedded::define_i18n_module!();\n",
        )
        .expect("write i18n module");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("workspace should discover crate");
        let mut issues = Vec::new();

        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(issues.iter().any(|issue| {
            issue.severity == "error"
                && issue
                    .message
                    .contains("src/lib.rs defines inline module `i18n`")
                && issue.help.contains("pub mod i18n;")
        }));
    }

    #[test]
    fn run_doctor_errors_when_inline_i18n_module_blocks_generated_module() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(temp.path().join("src/lib.rs"), "pub mod i18n {}\n")
            .expect("write inline module");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("workspace should discover crate");
        let mut issues = Vec::new();

        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(issues.iter().any(|issue| {
            issue.severity == "error"
                && issue
                    .message
                    .contains("src/lib.rs defines inline module `i18n`")
                && issue.help.contains("pub mod i18n;")
        }));
        assert!(issues.iter().any(|issue| {
            issue.severity == "warning" && issue.message.contains("src/i18n.rs is missing")
        }));
    }

    #[test]
    fn run_doctor_skips_i18n_module_guess_without_library_target() {
        let temp = crate::test_fixtures::create_binary_only_i18n_workspace();
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("workspace should discover binary-only crate");
        let mut issues = Vec::new();

        inspect_crate(&workspace.skipped[0], &mut issues);

        assert!(
            !issues
                .iter()
                .any(|issue| issue.message.contains("src/i18n.rs is missing")),
            "doctor should not guess an i18n module path before a library target exists: {issues:?}"
        );
    }

    #[test]
    fn run_doctor_errors_when_i18n_module_path_is_not_a_file() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::create_dir_all(temp.path().join("src/i18n.rs")).expect("create i18n module dir");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("workspace should discover crate");
        let mut issues = Vec::new();

        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(issues.iter().any(|issue| {
            issue.severity == "error"
                && issue
                    .message
                    .contains("src/i18n.rs exists but is not a file")
        }));
    }

    #[test]
    fn run_doctor_ignores_commented_manager_macro_references_but_warns_for_missing_macro() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(temp.path().join("src/lib.rs"), "pub mod i18n;\n").expect("declare i18n module");
        fs::write(
            temp.path().join("src/i18n.rs"),
            "// es_fluent_manager_embedded::define_i18n_module!();\nconst NOTE: &str = \"es_fluent_manager_dioxus::define_i18n_module!();\";\npub fn marker() {}\n",
        )
        .expect("write comment-only i18n module");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("workspace should discover crate");
        let mut issues = Vec::new();

        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(
            issues.iter().all(|issue| {
                !issue.message.contains("references es-fluent-manager")
                    && !issue.message.contains("build.rs")
            }),
            "commented/string manager macro references should not trigger doctor warnings"
        );
        assert!(
            issues.iter().any(|issue| {
                issue
                    .message
                    .contains("does not invoke `define_i18n_module!()`")
            }),
            "commented/string manager macro references should still leave a missing macro warning"
        );
    }

    #[test]
    fn run_doctor_ignores_commented_or_dev_manager_dependencies() {
        let cases = [
            (
                "commented",
                "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nes-fluent = \"0.16\"\nunic-langid = \"0.9\"\n# es-fluent-manager-embedded = \"0.16\"\n",
            ),
            (
                "dev",
                "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nes-fluent = \"0.16\"\nunic-langid = \"0.9\"\n\n[dev-dependencies]\nes-fluent-manager-embedded = \"0.16\"\n",
            ),
        ];

        for (name, manifest) in cases {
            let temp = crate::test_fixtures::create_test_crate_workspace();
            fs::write(temp.path().join("Cargo.toml"), manifest).expect("write manifest");
            fs::write(temp.path().join("src/lib.rs"), "pub mod i18n;\n")
                .expect("declare i18n module");
            fs::write(
                temp.path().join("src/i18n.rs"),
                "es_fluent_manager_embedded::define_i18n_module!();\n",
            )
            .expect("write i18n module");
            fs::write(
                temp.path().join("build.rs"),
                "fn main() { es_fluent_build::track_i18n_assets(); }\n",
            )
            .expect("write tracked build script");
            let workspace = WorkspaceCrates::discover(WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            })
            .expect("workspace should discover crate");
            let mut issues = Vec::new();

            inspect_crate(&workspace.crates[0], &mut issues);

            assert!(
                issues.iter().any(|issue| {
                    issue
                        .message
                        .contains("references es-fluent-manager-embedded")
                }),
                "{name} manager dependency should not satisfy runtime dependency check"
            );
        }
    }

    #[test]
    fn run_doctor_warns_when_build_tracking_dependency_is_missing() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nes-fluent = \"0.16\"\nes-fluent-manager-embedded = \"0.16\"\nunic-langid = \"0.9\"\n",
        )
        .expect("write manifest without build dependency");
        fs::write(temp.path().join("src/lib.rs"), "pub mod i18n;\n").expect("declare i18n module");
        fs::write(
            temp.path().join("src/i18n.rs"),
            "es_fluent_manager_embedded::define_i18n_module!();\n",
        )
        .expect("write i18n module");
        fs::write(
            temp.path().join("build.rs"),
            "fn main() { es_fluent_build::track_i18n_assets(); }\n",
        )
        .expect("write tracked build script");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("workspace should discover crate");
        let mut issues = Vec::new();

        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(issues.iter().any(|issue| {
            issue.message.contains("lacks es-fluent-build")
                && issue.help.contains("[build-dependencies]")
        }));
    }

    #[test]
    fn run_doctor_errors_when_build_rs_path_is_not_a_file() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(temp.path().join("src/lib.rs"), "pub mod i18n;\n").expect("declare i18n module");
        fs::write(
            temp.path().join("src/i18n.rs"),
            "es_fluent_manager_embedded::define_i18n_module!();\n",
        )
        .expect("write i18n module");
        fs::create_dir_all(temp.path().join("build.rs")).expect("create build.rs dir");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("workspace should discover crate");
        let mut issues = Vec::new();

        inspect_crate(&workspace.crates[0], &mut issues);

        assert!(issues.iter().any(|issue| {
            issue.severity == "error"
                && issue.message.contains("build.rs exists but is not a file")
                && issue.help.contains("track_i18n_assets")
        }));
    }

    #[test]
    fn run_doctor_reports_custom_i18n_module_path_in_manager_warning() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"custom-lib\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"lib.rs\"\n\n[dependencies]\nes-fluent = \"0.16\"\nunic-langid = \"0.9\"\n",
        )
        .expect("write manifest");
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n config");
        fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
        fs::write(temp.path().join("lib.rs"), "pub mod i18n;\n").expect("write custom lib");
        fs::write(
            temp.path().join("i18n.rs"),
            "es_fluent_manager_embedded::define_i18n_module!();\n",
        )
        .expect("write root i18n module");

        let result = run_doctor(DoctorArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            output: OutputFormat::Json,
        });

        assert!(result.is_ok(), "warnings should not fail doctor");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("workspace should discover custom lib");
        let mut issues = Vec::new();
        inspect_crate(&workspace.crates[0], &mut issues);
        assert!(issues.iter().any(|issue| {
            issue
                .message
                .contains("i18n.rs references es-fluent-manager-embedded")
                && !issue.message.contains("src/i18n.rs")
        }));
    }

    #[test]
    fn doctor_helpers_cover_dependency_detection_and_build_script_warnings() {
        assert_eq!(
            manager_dependency_from_module("es_fluent_manager_embedded::define_i18n_module!();"),
            Some("es-fluent-manager-embedded")
        );
        assert_eq!(
            manager_dependency_from_module("es_fluent_manager_dioxus::define_i18n_module!();"),
            Some("es-fluent-manager-dioxus")
        );
        assert_eq!(
            manager_dependency_from_module("es_fluent_manager_bevy::define_i18n_module!();"),
            Some("es-fluent-manager-bevy")
        );
        assert_eq!(
            manager_dependency_from_module("use es_fluent_manager_embedded :: define_i18n_module;"),
            Some("es-fluent-manager-embedded")
        );
        assert_eq!(
            manager_dependency_from_module(
                "use es_fluent_manager_embedded as manager;\nmanager::define_i18n_module!();",
            ),
            Some("es-fluent-manager-embedded")
        );
        assert_eq!(
            manager_dependency_from_module(
                "extern crate es_fluent_manager_embedded as manager;\nmanager::define_i18n_module!();",
            ),
            Some("es-fluent-manager-embedded")
        );
        assert_eq!(
            manager_dependency_from_module("use ::es_fluent_manager_embedded as manager;"),
            Some("es-fluent-manager-embedded")
        );
        assert_eq!(
            manager_dependency_from_module("fn es_fluent_manager_embedded_marker() {}"),
            None
        );
        assert_eq!(
            manager_dependency_from_module("let es_fluent_manager_embedded = ();"),
            None
        );
        assert_eq!(
            manager_dependency_from_module("use es_fluent_manager_embedded_marker as manager;"),
            None
        );
        assert_eq!(
            manager_dependency_from_module("extern crate es_fluent_manager_embedded_marker;"),
            None
        );
        assert_eq!(manager_dependency_from_module("no manager"), None);
        assert!(contains_macro_invocation(
            "define_i18n_module !();",
            "define_i18n_module"
        ));
        assert!(!contains_macro_invocation(
            "not_define_i18n_module!();",
            "define_i18n_module"
        ));
        assert!(!contains_macro_invocation(
            "define_i18n_module_name!();",
            "define_i18n_module"
        ));
        let inactive_macro = active_rust_source_text(
            "// define_i18n_module !();\nconst NOTE: &str = \"define_i18n_module!();\";\n",
        );
        assert!(!contains_macro_invocation(
            &inactive_macro,
            "define_i18n_module"
        ));
        assert!(manifest_has_dependency(
            "[dependencies]\nes-fluent-manager-embedded = { version = \"0.16\" }\n",
            "dependencies",
            "es-fluent-manager-embedded"
        ));
        assert!(!manifest_has_dependency(
            "[dev-dependencies]\nes-fluent-manager-embedded = \"0.16\"\n",
            "dependencies",
            "es-fluent-manager-embedded"
        ));
        assert!(manifest_has_dependency(
            "[build-dependencies]\nes-fluent-build = { version = \"0.16\" }\n",
            "build-dependencies",
            "es-fluent-build"
        ));

        let temp = crate::test_fixtures::create_test_crate_workspace();
        let manifest_without_build_dep =
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";
        let manifest_with_build_dep = "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[build-dependencies]\nes-fluent-build = \"0.16\"\n";
        let krate = CrateInfo {
            name: es_fluent_runner::PackageName::try_new("test-app").expect("valid package name"),
            manifest_dir: crate::core::ManifestDir::from_discovered(temp.path().to_path_buf()),
            src_dir: crate::core::SourceDir::from_discovered(temp.path().join("src")),
            i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(
                temp.path().join("i18n.toml"),
            ),
            ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
                temp.path().join("i18n/en"),
            ),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        };
        let mut issues = Vec::new();

        inspect_build_script(
            &krate,
            manifest_without_build_dep,
            "define_i18n_module !();",
            &mut issues,
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("build.rs does not track"))
        );
        assert!(
            issues.iter().any(|issue| {
                issue.help.contains("es-fluent-build")
                    && issue.help.contains("[build-dependencies]")
                    && !issue.help.contains("--force")
            }),
            "missing-build.rs help should not recommend init --force"
        );

        fs::write(temp.path().join("build.rs"), "fn main() {}\n").expect("write build.rs");
        issues.clear();
        inspect_build_script(
            &krate,
            manifest_without_build_dep,
            "define_i18n_module!();",
            &mut issues,
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("does not call"))
        );

        fs::write(
            temp.path().join("build.rs"),
            "fn main() {\n    // es_fluent_build::track_i18n_assets();\n}\n",
        )
        .expect("write commented tracking build.rs");
        issues.clear();
        inspect_build_script(
            &krate,
            manifest_without_build_dep,
            "define_i18n_module!();",
            &mut issues,
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("does not call")),
            "commented-out tracking calls should still warn"
        );

        fs::write(
            temp.path().join("build.rs"),
            "fn helper() { es_fluent_build::track_i18n_assets(); }\nfn main() {}\n",
        )
        .expect("write helper-only tracking build.rs");
        issues.clear();
        inspect_build_script(
            &krate,
            manifest_without_build_dep,
            "define_i18n_module!();",
            &mut issues,
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("does not call")),
            "helper-only tracking calls should still warn"
        );

        fs::write(
            temp.path().join("build.rs"),
            "fn main() {\n    fn helper() { es_fluent_build::track_i18n_assets(); }\n}\n",
        )
        .expect("write nested-helper tracking build.rs");
        issues.clear();
        inspect_build_script(
            &krate,
            manifest_without_build_dep,
            "define_i18n_module!();",
            &mut issues,
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("does not call")),
            "nested helper tracking calls should still warn"
        );

        fs::write(
            temp.path().join("build.rs"),
            "use es_fluent_build::track_i18n_assets;\nfn main() { track_i18n_assets(); }\n",
        )
        .expect("write imported tracking build.rs");
        issues.clear();
        inspect_build_script(
            &krate,
            manifest_without_build_dep,
            "define_i18n_module!();",
            &mut issues,
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("lacks es-fluent-build")),
            "active tracking without build dependency should warn"
        );
        issues.clear();
        inspect_build_script(
            &krate,
            manifest_with_build_dep,
            "define_i18n_module!();",
            &mut issues,
        );
        assert!(issues.is_empty());

        fs::write(
            temp.path().join("build.rs"),
            "fn main() {\n    use es_fluent_build::track_i18n_assets;\n    track_i18n_assets();\n}\n",
        )
        .expect("write main-local imported tracking build.rs");
        issues.clear();
        inspect_build_script(
            &krate,
            manifest_with_build_dep,
            "define_i18n_module!();",
            &mut issues,
        );
        assert!(
            issues.is_empty(),
            "main-local imported tracking calls should satisfy doctor tracking check"
        );

        fs::write(
            temp.path().join("build.rs"),
            "use es_fluent_build::{other_helper, track_i18n_assets};\nfn main() { track_i18n_assets(); }\n",
        )
        .expect("write grouped imported tracking build.rs");
        issues.clear();
        inspect_build_script(
            &krate,
            manifest_with_build_dep,
            "define_i18n_module!();",
            &mut issues,
        );
        assert!(
            issues.is_empty(),
            "grouped imported tracking calls should satisfy doctor tracking check"
        );

        for (name, build_rs_contents) in [
            (
                "direct alias",
                "use es_fluent_build::track_i18n_assets as track_assets;\nfn main() { track_assets(); }\n",
            ),
            (
                "grouped alias",
                "use es_fluent_build::{other_helper, track_i18n_assets as track_assets};\nfn main() { track_assets(); }\n",
            ),
        ] {
            fs::write(temp.path().join("build.rs"), build_rs_contents)
                .expect("write aliased tracking build.rs");
            issues.clear();
            inspect_build_script(
                &krate,
                manifest_with_build_dep,
                "define_i18n_module!();",
                &mut issues,
            );
            assert!(
                issues.is_empty(),
                "{name} tracking calls should satisfy doctor tracking check"
            );
        }

        for (name, build_rs_contents) in [
            (
                "direct module alias",
                "use es_fluent_build as efb;\nfn main() { efb::track_i18n_assets(); }\n",
            ),
            (
                "grouped module alias",
                "use es_fluent_build::{self as efb, other_helper};\nfn main() { efb::track_i18n_assets(); }\n",
            ),
        ] {
            fs::write(temp.path().join("build.rs"), build_rs_contents)
                .expect("write aliased module tracking build.rs");
            issues.clear();
            inspect_build_script(
                &krate,
                manifest_with_build_dep,
                "define_i18n_module!();",
                &mut issues,
            );
            assert!(
                issues.is_empty(),
                "{name} tracking calls should satisfy doctor tracking check"
            );
        }

        for (name, build_rs_contents) in [
            (
                "extern crate",
                "extern crate es_fluent_build;\nfn main() { es_fluent_build::track_i18n_assets(); }\n",
            ),
            (
                "extern crate alias",
                "extern crate es_fluent_build as efb;\nfn main() { efb::track_i18n_assets(); }\n",
            ),
        ] {
            fs::write(temp.path().join("build.rs"), build_rs_contents)
                .expect("write extern crate tracking build.rs");
            issues.clear();
            inspect_build_script(
                &krate,
                manifest_with_build_dep,
                "define_i18n_module!();",
                &mut issues,
            );
            assert!(
                issues.is_empty(),
                "{name} tracking calls should satisfy doctor tracking check"
            );
        }

        fs::write(
            temp.path().join("build.rs"),
            "fn track_i18n_assets() {}\nfn main() { track_i18n_assets(); }\n",
        )
        .expect("write local helper tracking build.rs");
        issues.clear();
        inspect_build_script(
            &krate,
            manifest_without_build_dep,
            "define_i18n_module!();",
            &mut issues,
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("does not call")),
            "local helper calls should not satisfy doctor tracking check"
        );

        fs::write(
            temp.path().join("build.rs"),
            "fn helper() {\n    use es_fluent_build::track_i18n_assets;\n}\nfn main() { track_i18n_assets(); }\n",
        )
        .expect("write helper-scoped imported tracking build.rs");
        issues.clear();
        inspect_build_script(
            &krate,
            manifest_with_build_dep,
            "define_i18n_module!();",
            &mut issues,
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("does not call")),
            "helper-scoped imports should not satisfy doctor tracking check"
        );

        fs::write(
            temp.path().join("build.rs"),
            "fn main() { es_fluent_build::track_i18n_assets(); }\n",
        )
        .expect("write tracked build.rs");
        issues.clear();
        inspect_build_script(
            &krate,
            manifest_with_build_dep,
            "define_i18n_module!();",
            &mut issues,
        );
        assert!(issues.is_empty());
    }

    #[test]
    fn run_doctor_json_reports_empty_workspace_warning() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"empty\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .expect("write manifest");
        fs::create_dir_all(temp.path().join("src")).expect("create src");
        fs::write(temp.path().join("src/lib.rs"), "pub struct Empty;\n").expect("write lib");

        let result = run_doctor(DoctorArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            output: OutputFormat::Json,
        });

        assert!(result.is_ok());
    }
}

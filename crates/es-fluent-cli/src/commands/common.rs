use crate::core::{CliError, CrateInfo, GenerateResult, GenerationAction, WorkspaceInfo};
use crate::generation::MonolithicExecutor;
use crate::utils::ui;
use anyhow::Context as _;
use clap::{Args, ValueEnum};
use colored::Colorize as _;
use fs_err as fs;
use serde::Serialize;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use toml_edit::{DocumentMut, Item};

#[derive(Args, Clone, Debug)]
pub struct WorkspaceArgs {
    /// Existing path to a crate/workspace root, its Cargo.toml, or a path inside a crate (defaults to current directory).
    #[arg(short, long)]
    pub path: Option<PathBuf>,
    /// Workspace package name to process, even when --path points inside a different member.
    #[arg(short = 'P', long)]
    pub package: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

impl OutputFormat {
    pub fn is_json(self) -> bool {
        matches!(self, Self::Json)
    }

    pub fn print_json<T: Serialize>(self, value: &T) -> Result<(), CliError> {
        if self.is_json() {
            println!(
                "{}",
                serde_json::to_string_pretty(value)
                    .map_err(|error| CliError::Other(error.to_string()))?
            );
        }

        Ok(())
    }
}

/// Represents a resolved set of crates for a command to operate on.
#[derive(Clone, Debug)]
pub struct WorkspaceCrates {
    /// Workspace information (root dir, target dir, all crates).
    pub workspace_info: WorkspaceInfo,
    /// All crates discovered (after optional package filtering).
    pub crates: Vec<CrateInfo>,
    /// Crates that are eligible for operations (have a Cargo library target).
    pub valid: Vec<CrateInfo>,
    /// Crates that were skipped (missing a Cargo library target).
    pub skipped: Vec<CrateInfo>,
    /// Package filter that matched no crates, if one was supplied.
    pub(crate) package_not_found: Option<String>,
    /// All workspace packages that have an i18n.toml, without parsing each config.
    pub(crate) all_i18n_package_names: Vec<String>,
}

impl WorkspaceCrates {
    /// Discover crates for a command, applying the common filtering and partitioning logic.
    pub fn discover(args: WorkspaceArgs) -> Result<Self, CliError> {
        let WorkspaceArgs { path, package } = normalize_workspace_args(args)?;
        let path = path.unwrap_or_else(|| PathBuf::from("."));
        let package_filter = package.clone();
        let requested_path = path
            .canonicalize()
            .with_context(|| format!("Failed to canonicalize root directory {}", path.display()))
            .map_err(CliError::from)?;
        let lexical_requested_path = lexical_absolute_path(&path).map_err(CliError::from)?;
        let metadata_dir =
            workspace_metadata_dir(&lexical_requested_path, requested_path.as_path());
        let discovery_scope = if let Some(package) = package.as_deref() {
            crate::utils::DiscoveryScope::Package(package)
        } else {
            crate::utils::DiscoveryScope::RequestedPaths {
                lexical: lexical_requested_path.as_path(),
                canonical: requested_path.as_path(),
            }
        };
        let all_i18n_package_names = crate::utils::discover_i18n_package_names(&metadata_dir)?;
        let workspace_info =
            crate::utils::discover_workspace_scoped(&metadata_dir, discovery_scope)?;
        let crates = if package.is_some() {
            crate::utils::filter_crates_by_package(workspace_info.crates.clone(), package.as_ref())
        } else {
            crates_for_requested_path(
                workspace_info.crates.clone(),
                &workspace_info,
                &[lexical_requested_path.as_path(), requested_path.as_path()],
            )
        };
        let package_not_found = package_filter.filter(|_| crates.is_empty());
        let (valid_refs, skipped_refs) = crate::utils::partition_by_lib_rs(&crates);
        let valid = valid_refs.into_iter().cloned().collect();
        let skipped = skipped_refs.into_iter().cloned().collect();

        Ok(Self {
            workspace_info,
            crates,
            valid,
            skipped,
            package_not_found,
            all_i18n_package_names,
        })
    }

    /// Print a standardized discovery summary, including skipped crates.
    ///
    /// Returns `false` when no crates were discovered to allow early-exit flows.
    pub fn print_discovery(&self, header: impl Fn()) -> bool {
        header();

        if self.crates.is_empty() {
            self.print_no_crates_found();
            return false;
        }

        ui::Ui::print_discovered(&self.crates);

        for krate in &self.skipped {
            ui::Ui::print_missing_lib_rs(krate.name.as_str());
        }

        true
    }

    /// Print the appropriate empty-selection message.
    pub fn print_no_crates_found(&self) {
        if let Some(package) = &self.package_not_found {
            ui::Ui::print_package_not_found(package);
        } else {
            ui::Ui::print_no_crates_found();
        }
    }

    /// Return the package filter that matched no crates, if any.
    pub fn package_not_found(&self) -> Option<&str> {
        self.package_not_found.as_deref()
    }

    /// Return an actionable message for an empty command selection.
    pub fn empty_selection_message(&self) -> Option<String> {
        if !self.crates.is_empty() {
            return None;
        }

        Some(if let Some(package) = &self.package_not_found {
            format!("no configured crate found matching package filter '{package}'")
        } else {
            "no crates with i18n.toml were found".to_string()
        })
    }

    /// Require a command to have at least one selected crate.
    pub fn require_non_empty_selection(&self) -> Result<(), CliError> {
        if let Some(message) = self.empty_selection_message() {
            return Err(CliError::Other(message));
        }

        Ok(())
    }

    /// Require every selected crate to have a Cargo library target.
    pub fn require_all_crates_valid(&self) -> Result<(), CliError> {
        if !self.skipped.is_empty() {
            let crate_names = self
                .skipped
                .iter()
                .map(|krate| format!("'{}'", krate.name))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(CliError::Other(format!(
                "configured crate(s) missing a Cargo library target: {crate_names}"
            )));
        }

        if self.valid.is_empty() {
            return Err(CliError::Other(
                "no discovered crates have a Cargo library target".to_string(),
            ));
        }

        Ok(())
    }
}

fn normalize_workspace_args(args: WorkspaceArgs) -> Result<WorkspaceArgs, CliError> {
    if let Some(path) = args.path.as_ref()
        && path.as_os_str().to_string_lossy().trim().is_empty()
    {
        return Err(CliError::Other(
            "workspace path must not be empty; pass a path or omit --path".to_string(),
        ));
    }

    let package = match args.package {
        Some(package) => {
            let package = package.trim();
            if package.is_empty() {
                return Err(CliError::Other(
                    "package filter must not be empty; pass a Cargo package name or omit --package"
                        .to_string(),
                ));
            }
            Some(package.to_string())
        },
        None => None,
    };

    Ok(WorkspaceArgs {
        path: args.path,
        package,
    })
}

fn lexical_absolute_path(path: &Path) -> anyhow::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .context("Failed to read current directory")?
            .join(path)
    };
    Ok(normalize_lexical_path(&absolute))
}

fn normalize_lexical_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {},
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            },
            Component::Normal(part) => normalized.push(part),
        }
    }

    if normalized.as_os_str().is_empty() {
        normalized.push(".");
    }
    normalized
}

fn workspace_metadata_dir(
    lexical_requested_path: &Path,
    canonical_requested_path: &Path,
) -> PathBuf {
    let lexical_start = if canonical_requested_path.is_file() {
        lexical_requested_path
            .parent()
            .unwrap_or(lexical_requested_path)
    } else {
        lexical_requested_path
    };

    if let Some(manifest_ancestor) = lexical_start
        .ancestors()
        .find(|ancestor| ancestor.join("Cargo.toml").is_file())
    {
        return manifest_ancestor.to_path_buf();
    }

    if canonical_requested_path.is_file() {
        canonical_requested_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| canonical_requested_path.to_path_buf())
    } else {
        canonical_requested_path.to_path_buf()
    }
}

#[cfg(test)]
mod workspace_arg_tests {
    use super::*;

    #[test]
    fn normalize_workspace_args_trims_package_filter() {
        let args = normalize_workspace_args(WorkspaceArgs {
            path: None,
            package: Some(" test-app ".to_string()),
        })
        .expect("package filter should normalize");

        assert_eq!(args.package.as_deref(), Some("test-app"));
    }

    #[test]
    fn normalize_workspace_args_rejects_empty_package_filter() {
        let err = normalize_workspace_args(WorkspaceArgs {
            path: None,
            package: Some(" ".to_string()),
        })
        .expect_err("empty package filter should fail");

        assert!(err.to_string().contains("package filter must not be empty"));
    }

    #[test]
    fn normalize_workspace_args_rejects_blank_path() {
        let err = normalize_workspace_args(WorkspaceArgs {
            path: Some(PathBuf::from("   ")),
            package: None,
        })
        .expect_err("blank path should fail");

        assert!(err.to_string().contains("workspace path must not be empty"));
    }
}

fn crates_for_requested_path(
    crates: Vec<CrateInfo>,
    workspace_info: &WorkspaceInfo,
    requested_paths: &[&std::path::Path],
) -> Vec<CrateInfo> {
    if requested_paths.iter().any(|requested_path| {
        let is_workspace_manifest = requested_path
            .file_name()
            .is_some_and(|name| name == "Cargo.toml")
            && requested_path.parent() == Some(workspace_info.root_dir.as_path());
        *requested_path == workspace_info.root_dir.as_path() || is_workspace_manifest
    }) {
        return crates;
    }

    if let Some(manifest_dir) = crates
        .iter()
        .filter(|krate| {
            requested_paths
                .iter()
                .any(|requested_path| requested_path.starts_with(krate.manifest_dir.as_path()))
        })
        .map(|krate| krate.manifest_dir.as_path().to_path_buf())
        .max_by_key(|path| path.components().count())
    {
        return crates
            .into_iter()
            .filter(|krate| krate.manifest_dir.as_path() == manifest_dir)
            .collect();
    }

    if requested_paths
        .iter()
        .any(|requested_path| requested_path.starts_with(&workspace_info.root_dir))
    {
        return Vec::new();
    }

    crates
}

/// Run generation-like work using the monolithic temp crate approach.
///
/// This prepares a single temp crate at workspace root that links the requested
/// crates, then runs the binary sequentially for each crate. Much faster on
/// subsequent runs.
///
/// If `force_run` is true, the staleness check is skipped and the runner is always rebuilt.
pub fn run_generation_for_crates(
    workspace: &WorkspaceInfo,
    crates: &[CrateInfo],
    action: &GenerationAction,
    force_run: bool,
    show_progress: bool,
) -> Vec<GenerateResult> {
    let runner_workspace = WorkspaceInfo {
        root_dir: workspace.root_dir.clone(),
        target_dir: workspace.target_dir.clone(),
        crates: crates.to_vec(),
    };

    let _runner_lock =
        match crate::generation::acquire_monolithic_runner_lock(&runner_workspace.root_dir) {
            Ok(lock) => lock,
            Err(e) => {
                return crates
                    .iter()
                    .map(|k| {
                        GenerateResult::failure(
                            k.name.clone(),
                            std::time::Duration::ZERO,
                            e.to_string(),
                        )
                    })
                    .collect();
            },
        };

    // Prepare the monolithic temp crate once upfront
    if let Err(e) = crate::generation::prepare_monolithic_runner_crate(&runner_workspace) {
        // If preparation fails, return error results for all crates
        return crates
            .iter()
            .map(|k| {
                GenerateResult::failure(k.name.clone(), std::time::Duration::ZERO, e.to_string())
            })
            .collect();
    }

    let executor = MonolithicExecutor::new(&runner_workspace);
    let pb = if show_progress {
        ui::Ui::create_progress_bar(crates.len() as u64, "Processing crates...")
    } else {
        indicatif::ProgressBar::hidden()
    };

    // Process sequentially since they share the same binary
    // (parallel could cause contention on first build)
    crates
        .iter()
        .map(|krate| {
            let result = executor.execute_generation_action(krate, action, force_run);
            pb.inc(1);
            result
        })
        .collect()
}

pub(crate) fn validate_generation_paths(
    crates: &[CrateInfo],
    validate_fallback_locale: bool,
) -> Result<(), CliError> {
    let mut invalid_paths = Vec::new();

    for krate in crates {
        if let Some(error) = library_target_path_setup_error(krate) {
            invalid_paths.push(error);
        }
        if let Some(error) = library_i18n_module_declaration_setup_error(krate) {
            invalid_paths.push(error);
        }

        let ctx = crate::ftl::LocaleContext::from_crate(krate, false)
            .map_err(|error| CliError::Other(error.to_string()))?;

        if ctx.assets_dir.exists() && !ctx.assets_dir.is_dir() {
            invalid_paths.push(format!(
                "assets_dir for {} is not a directory: {}",
                krate.name,
                ctx.assets_dir.display()
            ));
        }

        if let Some(blocked_path) = non_directory_ancestor(&ctx.assets_dir) {
            invalid_paths.push(format!(
                "assets_dir for {} cannot be created because a path component is not a directory: {}",
                krate.name,
                blocked_path.display()
            ));
        }

        let fallback_dir = ctx.locale_dir(&ctx.fallback);
        if validate_fallback_locale && fallback_dir.exists() && !fallback_dir.is_dir() {
            invalid_paths.push(format!(
                "fallback locale path '{}' for {} is not a directory: {}",
                ctx.fallback,
                krate.name,
                fallback_dir.display()
            ));
        }

        if validate_fallback_locale
            && let Some(blocked_path) = non_directory_ancestor(&fallback_dir)
        {
            invalid_paths.push(format!(
                "fallback locale path '{}' for {} cannot be created because a path component is not a directory: {}",
                ctx.fallback,
                krate.name,
                blocked_path.display()
            ));
        }

        if validate_fallback_locale && fallback_dir.is_dir() {
            let layout = crate::ftl::CrateFtlLayout::from_assets_dir(
                &ctx.assets_dir,
                &ctx.fallback,
                krate.name.as_str(),
            );
            if let Err(error) = layout.discover_files() {
                invalid_paths.push(format!(
                    "fallback locale FTL layout for {} could not be read: {}",
                    krate.name, error
                ));
            }
        }
    }

    if !invalid_paths.is_empty() {
        invalid_paths.sort();
        return Err(CliError::Other(format!(
            "generation path setup error(s): {}",
            invalid_paths.join(", ")
        )));
    }

    Ok(())
}

fn non_directory_ancestor(path: &Path) -> Option<PathBuf> {
    path.parent()?
        .ancestors()
        .find(|ancestor| ancestor.exists() && !ancestor.is_dir())
        .map(Path::to_path_buf)
}

pub(crate) fn library_target_path_setup_error(krate: &CrateInfo) -> Option<String> {
    if !krate.has_lib_rs {
        return None;
    }

    let (normalized, abs_path) = match normalized_library_target_path(krate) {
        Ok(path) => path,
        Err(error) => return Some(error),
    };

    library_target_existing_path_violation(krate.manifest_dir.as_path(), &normalized).map(
        |reason| {
            format!(
                "library target path for {} is not a real in-crate source path: {} uses {reason}",
                krate.name,
                relative_to_crate(krate, &abs_path)
            )
        },
    )
}

pub(crate) fn library_i18n_module_declaration_setup_error(krate: &CrateInfo) -> Option<String> {
    if !krate.has_lib_rs {
        return None;
    }

    let (_, library_target_path) = match normalized_library_target_path(krate) {
        Ok(path) => path,
        Err(_) => return None,
    };
    let library_path = relative_to_crate(krate, &library_target_path);
    let source = match fs::read_to_string(&library_target_path) {
        Ok(source) => source,
        Err(error) => {
            return Some(format!(
                "{}: {library_path} could not be read: {error}",
                krate.name
            ));
        },
    };
    let library_target_defines_i18n_module = super::doctor::contains_macro_invocation(
        &super::doctor::active_rust_source_text(&source),
        "define_i18n_module",
    );
    let i18n_module_path = krate.src_dir.join("i18n.rs");
    let module_path = relative_to_crate(krate, &i18n_module_path);
    let module_file_exists = match fs::symlink_metadata(&i18n_module_path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Some(format!(
                "{}: {module_path} is a symlink; replace it with a real Rust module file",
                krate.name
            ));
        },
        Ok(metadata) if metadata.is_file() => true,
        Ok(_) => {
            return Some(format!(
                "{}: {module_path} exists but is not a file; replace it with a Rust module file",
                krate.name
            ));
        },
        Err(error) if error.kind() == ErrorKind::NotFound => false,
        Err(error) => {
            return Some(format!(
                "{}: {module_path} could not be inspected: {error}",
                krate.name
            ));
        },
    };

    match super::init::i18n_module_declaration_kind_in_source(&source) {
        Some(super::init::I18nModuleDeclaration::External(
            super::init::I18nModuleVisibility::Public,
        )) => None,
        Some(super::init::I18nModuleDeclaration::External(
            super::init::I18nModuleVisibility::Restricted,
        )) => Some(format!(
            "{}: {library_path} declares module `i18n` without public visibility; change it to `pub mod i18n;`",
            krate.name
        )),
        Some(super::init::I18nModuleDeclaration::Inline(_)) => Some(format!(
            "{}: {library_path} defines inline module `i18n`; move the inline module to i18n.rs or remove it, then declare `pub mod i18n;`",
            krate.name
        )),
        None if module_file_exists && library_target_defines_i18n_module => Some(format!(
            "{}: {module_path} exists but {library_path} directly calls `define_i18n_module!()`; remove {module_path}, or move the macro there and declare it with `pub mod i18n;`",
            krate.name
        )),
        None if module_file_exists => Some(format!(
            "{}: {library_path} does not declare module `i18n`; add `pub mod i18n;` so the generated i18n module is compiled",
            krate.name
        )),
        None => None,
    }
}

fn normalized_library_target_path(krate: &CrateInfo) -> Result<(PathBuf, PathBuf), String> {
    let manifest_path = krate.manifest_dir.join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("Cargo.toml for {} could not be read: {}", krate.name, error))?;
    let raw_path = manifest_library_path_value(&manifest);
    let raw_path = raw_path.as_deref().unwrap_or("src/lib.rs");
    let normalized = match normalize_library_target_path(raw_path) {
        Ok(path) => path,
        Err(reason) => {
            return Err(format!(
                "library target path for {} is invalid: Cargo.toml [lib].path {reason}: {raw_path}",
                krate.name
            ));
        },
    };
    let abs_path = krate.manifest_dir.join(&normalized);

    Ok((normalized, abs_path))
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

fn relative_to_crate(krate: &CrateInfo, path: &Path) -> String {
    path.strip_prefix(krate.manifest_dir.as_path())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Execute a generation-like command that uses the monolithic runner.
pub fn run_generation_command(
    workspace_args: WorkspaceArgs,
    action: GenerationAction,
    force_run: bool,
    dry_run: bool,
    verb: GenerationVerb,
) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(workspace_args)?;

    if !workspace.print_discovery(ui::Ui::print_header) {
        return workspace.require_non_empty_selection();
    }
    workspace.require_all_crates_valid()?;
    validate_generation_paths(&workspace.valid, true)?;

    let results = run_generation_for_crates(
        &workspace.workspace_info,
        &workspace.valid,
        &action,
        force_run,
        true,
    );
    let has_errors = render_generation_results_with_dry_run(&results, dry_run, verb);

    if has_errors {
        return Err(CliError::Other(
            "generation command failed; see diagnostics above".to_string(),
        ));
    }

    Ok(())
}

/// Render a list of `GenerateResult`s with custom success/error handlers.
///
/// Returns `true` when any errors were encountered.
pub fn render_generation_results(
    results: &[GenerateResult],
    on_success: impl Fn(&GenerateResult),
    on_error: impl Fn(&GenerateResult),
) -> bool {
    let mut has_errors = false;

    for result in results {
        if result.error.is_some() {
            has_errors = true;
            on_error(result);
        } else {
            on_success(result);
        }
    }

    has_errors
}

#[derive(Clone, Copy, Debug)]
pub enum GenerationVerb {
    Generate,
    Clean,
}

impl GenerationVerb {
    fn dry_run_label(self) -> &'static str {
        match self {
            GenerationVerb::Generate => "would be generated in",
            GenerationVerb::Clean => "would be cleaned in",
        }
    }

    fn print_changed(self, result: &GenerateResult) {
        match self {
            GenerationVerb::Generate => {
                ui::Ui::print_generated(
                    result.name.as_str(),
                    result.duration,
                    result.resource_count,
                );
            },
            GenerationVerb::Clean => {
                ui::Ui::print_cleaned(result.name.as_str(), result.duration, result.resource_count);
            },
        }
    }
}

/// Render generation-like results with the standard dry-run output.
///
/// Returns `true` when any errors were encountered.
pub fn render_generation_results_with_dry_run(
    results: &[GenerateResult],
    dry_run: bool,
    verb: GenerationVerb,
) -> bool {
    render_generation_results(
        results,
        |result| {
            if dry_run {
                if let Some(output) = &result.output {
                    print!("{}", output);
                } else if result.changed {
                    println!(
                        "{} {} ({} resources)",
                        format!("{} {}", result.name, verb.dry_run_label()).yellow(),
                        ui::Ui::format_duration(result.duration).green(),
                        result.resource_count.to_string().cyan()
                    );
                } else {
                    println!("{} {}", "Unchanged:".dimmed(), result.name.as_str().bold());
                }
            } else if result.changed {
                verb.print_changed(result);
            } else {
                println!("{} {}", "Unchanged:".dimmed(), result.name.as_str().bold());
            }
        },
        |result| {
            ui::Ui::print_generation_error(result.name.as_str(), result.error.as_ref().unwrap())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{CrateInfo, FluentParseMode, GenerationAction, WorkspaceInfo};
    use crate::test_fixtures::FakeRunnerBehavior;
    use std::cell::Cell;
    use std::fs;
    use std::path::PathBuf;
    use std::time::Duration;

    fn package(name: &str) -> es_fluent_runner::PackageName {
        es_fluent_runner::PackageName::try_new(name).expect("valid package name")
    }

    fn create_workspace_info(temp: &tempfile::TempDir) -> WorkspaceInfo {
        let manifest_dir = temp.path().to_path_buf();
        let src_dir = manifest_dir.join("src");
        let i18n_toml = manifest_dir.join("i18n.toml");
        let krate = CrateInfo {
            name: package("test-app"),
            manifest_dir: crate::core::ManifestDir::from_discovered(manifest_dir.clone()),
            src_dir: crate::core::SourceDir::from_discovered(src_dir),
            i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(i18n_toml),
            ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
                manifest_dir.join("i18n/en"),
            ),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        };

        WorkspaceInfo {
            root_dir: manifest_dir.clone(),
            target_dir: manifest_dir.join("target"),
            crates: vec![krate],
        }
    }

    fn write_i18n_workspace_member(root: &std::path::Path, name: &str) {
        let manifest_dir = root.join(name);
        fs::create_dir_all(manifest_dir.join("src")).expect("create src");
        fs::write(
            manifest_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write manifest");
        fs::write(manifest_dir.join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
        fs::write(
            manifest_dir.join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n config");
    }

    #[test]
    fn workspace_discovery_rejects_empty_package_filter_before_path_validation() {
        let result = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(PathBuf::from("/definitely/missing/path")),
            package: Some("  ".to_string()),
        });

        assert!(
            matches!(&result, Err(CliError::Other(message)) if message.contains("package filter must not be empty")),
            "unexpected result: {result:?}"
        );
    }

    #[test]
    fn library_i18n_module_error_mentions_direct_macro_when_stale_module_file_exists() {
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
        .expect("discover workspace");

        let error = library_i18n_module_declaration_setup_error(&workspace.crates[0])
            .expect("stale module file should be reported");

        assert!(
            error.contains("src/i18n.rs exists")
                && error.contains("src/lib.rs directly calls `define_i18n_module!()`")
                && error.contains("remove src/i18n.rs"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn read_changed_status_handles_missing_invalid_and_valid_json() {
        let temp = tempfile::tempdir().unwrap();
        let crate_name = "demo";
        let store = es_fluent_runner::RunnerMetadataStore::new(temp.path());
        let package_name = package(crate_name);
        let result_path = store.result_path(&package_name);
        fs::create_dir_all(result_path.parent().unwrap()).unwrap();

        assert!(!store.result_changed(&package_name));

        fs::write(&result_path, "{not-json").unwrap();
        assert!(!store.result_changed(&package_name));

        fs::write(&result_path, r#"{"changed":true}"#).unwrap();
        assert!(store.result_changed(&package_name));
    }

    #[test]
    fn render_generation_results_reports_error_presence() {
        let success = GenerateResult::success(
            package("ok-crate"),
            Duration::from_millis(10),
            1,
            None,
            false,
        );
        let failure = GenerateResult::failure(
            package("bad-crate"),
            Duration::from_millis(5),
            "boom".to_string(),
        );

        let success_calls = Cell::new(0usize);
        let error_calls = Cell::new(0usize);

        let has_errors = render_generation_results(
            &[success, failure],
            |_| success_calls.set(success_calls.get() + 1),
            |_| error_calls.set(error_calls.get() + 1),
        );

        assert!(has_errors);
        assert_eq!(success_calls.get(), 1);
        assert_eq!(error_calls.get(), 1);
    }

    #[test]
    fn generation_verb_labels_match_expected_text() {
        assert_eq!(
            GenerationVerb::Generate.dry_run_label(),
            "would be generated in"
        );
        assert_eq!(GenerationVerb::Clean.dry_run_label(), "would be cleaned in");
    }

    #[test]
    fn workspace_discover_supports_package_filtering() {
        let temp = crate::test_fixtures::create_test_crate_workspace_without_ftl();

        let all = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .unwrap();
        assert_eq!(all.crates.len(), 1);
        assert_eq!(all.valid.len(), 1);

        let filtered = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: Some("missing-crate".to_string()),
        })
        .unwrap();
        assert!(filtered.crates.is_empty());
        assert!(filtered.valid.is_empty());
    }

    #[test]
    fn workspace_discover_scopes_member_path_to_that_member_by_default() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
        )
        .expect("write workspace manifest");
        write_i18n_workspace_member(temp.path(), "a");
        write_i18n_workspace_member(temp.path(), "b");

        let all = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("discover workspace root");
        assert_eq!(
            all.crates
                .iter()
                .map(|krate| krate.name.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );

        let all_from_manifest = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().join("Cargo.toml")),
            package: None,
        })
        .expect("discover workspace root manifest");
        assert_eq!(
            all_from_manifest
                .crates
                .iter()
                .map(|krate| krate.name.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );

        let member = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().join("a")),
            package: None,
        })
        .expect("discover workspace member");
        assert_eq!(member.crates.len(), 1);
        assert_eq!(member.crates[0].name, "a");

        let nested_member = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().join("a/src")),
            package: None,
        })
        .expect("discover nested workspace member path");
        assert_eq!(nested_member.crates.len(), 1);
        assert_eq!(nested_member.crates[0].name, "a");

        let member_file = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().join("a/src/lib.rs")),
            package: None,
        })
        .expect("discover workspace member file path");
        assert_eq!(member_file.crates.len(), 1);
        assert_eq!(member_file.crates[0].name, "a");

        let explicit_package = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().join("a")),
            package: Some("b".to_string()),
        })
        .expect("discover explicit package from member path");
        assert_eq!(explicit_package.crates.len(), 1);
        assert_eq!(explicit_package.crates[0].name, "b");
    }

    #[cfg(unix)]
    #[test]
    fn workspace_discover_scopes_symlinked_member_path_by_lexical_location() {
        let temp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside tempdir");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
        )
        .expect("write workspace manifest");
        write_i18n_workspace_member(temp.path(), "a");
        write_i18n_workspace_member(temp.path(), "b");
        std::os::unix::fs::symlink(outside.path(), temp.path().join("a/src/external"))
            .expect("create symlink inside member");

        let selected = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().join("a/src/external")),
            package: None,
        })
        .expect("discover symlinked path inside workspace member");

        assert_eq!(selected.crates.len(), 1);
        assert_eq!(selected.crates[0].name, "a");
    }

    #[test]
    fn workspace_discover_member_path_without_i18n_does_not_select_siblings() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
        )
        .expect("write workspace manifest");

        let a_dir = temp.path().join("a");
        fs::create_dir_all(a_dir.join("src")).expect("create a src");
        fs::write(
            a_dir.join("Cargo.toml"),
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
        )
        .expect("write a manifest");
        fs::write(a_dir.join("src/lib.rs"), "pub fn marker() {}\n").expect("write a lib");

        write_i18n_workspace_member(temp.path(), "b");

        let selected = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(a_dir),
            package: None,
        })
        .expect("discover workspace member without i18n");

        assert!(
            selected.crates.is_empty(),
            "member path without i18n.toml should not select configured siblings"
        );
        assert_eq!(
            selected.empty_selection_message().as_deref(),
            Some("no crates with i18n.toml were found")
        );

        let selected_nested = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().join("a/src")),
            package: None,
        })
        .expect("discover nested workspace member path without i18n");

        assert!(
            selected_nested.crates.is_empty(),
            "nested member path without i18n.toml should not select configured siblings"
        );
    }

    #[test]
    fn workspace_discover_workspace_subdir_without_i18n_member_match_selects_no_crates() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"b\"]\nresolver = \"2\"\n",
        )
        .expect("write workspace manifest");
        fs::create_dir_all(temp.path().join("tools")).expect("create workspace subdir");
        write_i18n_workspace_member(temp.path(), "b");

        let selected = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().join("tools")),
            package: None,
        })
        .expect("discover workspace subdir");

        assert!(
            selected.crates.is_empty(),
            "workspace subdirectories should not silently widen to all configured crates"
        );
    }

    #[test]
    fn run_generation_for_crates_uses_cached_runner_and_reads_changed_status() {
        let temp = crate::test_fixtures::create_test_crate_workspace_without_ftl();
        let workspace = create_workspace_info(&temp);
        let krate = workspace.crates[0].clone();

        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::stdout("generated-from-fake-runner\n"),
        );

        let temp_dir =
            es_fluent_runner::RunnerMetadataStore::temp_for_workspace(&workspace.root_dir);
        let result_json = temp_dir.result_path(&krate.name);
        fs::create_dir_all(result_json.parent().unwrap()).expect("create metadata dir");
        fs::write(&result_json, r#"{"changed":true}"#).expect("write result json");

        let results = run_generation_for_crates(
            &workspace,
            std::slice::from_ref(&krate),
            &GenerationAction::Generate {
                mode: FluentParseMode::default(),
                dry_run: false,
            },
            false,
            false,
        );

        assert_eq!(results.len(), 1);
        assert!(results[0].error.is_none());
        assert!(results[0].changed);
        assert!(
            results[0]
                .output
                .as_ref()
                .expect("captured output")
                .contains("generated-from-fake-runner")
        );
    }

    #[test]
    fn run_generation_for_crates_links_only_requested_crates() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
        )
        .expect("write workspace manifest");

        let mut crates = Vec::new();
        for name in ["a", "b"] {
            let manifest_dir = temp.path().join(name);
            let src_dir = manifest_dir.join("src");
            let i18n_toml = manifest_dir.join("i18n.toml");
            fs::create_dir_all(&src_dir).expect("create src");
            fs::create_dir_all(manifest_dir.join("i18n/en")).expect("create i18n");
            fs::write(
                manifest_dir.join("Cargo.toml"),
                format!(
                    "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
                ),
            )
            .expect("write manifest");
            fs::write(src_dir.join("lib.rs"), "pub fn marker() {}\n").expect("write lib");
            fs::write(
                &i18n_toml,
                "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
            )
            .expect("write i18n config");
            fs::write(
                manifest_dir.join(format!("i18n/en/{name}.ftl")),
                "hello = Hello\n",
            )
            .expect("write ftl");

            crates.push(CrateInfo {
                name: package(name),
                manifest_dir: crate::core::ManifestDir::from_discovered(manifest_dir.clone()),
                src_dir: crate::core::SourceDir::from_discovered(src_dir),
                i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(i18n_toml),
                ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
                    manifest_dir.join("i18n/en"),
                ),
                has_lib_rs: true,
                fluent_features: Vec::new(),
            });
        }

        let workspace = WorkspaceInfo {
            root_dir: temp.path().to_path_buf(),
            target_dir: temp.path().join("target"),
            crates,
        };
        let krate = workspace.crates[0].clone();

        let temp_store = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(temp.path());
        let binary_path = crate::test_fixtures::fake_runner_binary_path(&workspace.target_dir);
        let mut crate_hashes = indexmap::IndexMap::new();
        crate_hashes.insert(
            krate.name.clone(),
            crate::generation::cache::compute_crate_inputs_hash(
                &krate.manifest_dir,
                &krate.src_dir,
                Some(&krate.i18n_config_path),
            ),
        );
        crate::test_fixtures::install_fake_runner_with_cache(
            &binary_path,
            &temp_store,
            temp.path(),
            &FakeRunnerBehavior::silent_success(),
            env!("CARGO_PKG_VERSION"),
            crate_hashes,
        );

        let results = run_generation_for_crates(
            &workspace,
            std::slice::from_ref(&krate),
            &GenerationAction::Generate {
                mode: FluentParseMode::default(),
                dry_run: true,
            },
            false,
            false,
        );

        assert_eq!(results.len(), 1);
        assert!(results[0].error.is_none(), "{:?}", results[0].error);

        let runner_manifest =
            fs::read_to_string(temp_store.base_dir().join("Cargo.toml")).expect("runner manifest");
        let runner_manifest: toml::Value =
            toml::from_str(&runner_manifest).expect("parse runner manifest");
        let dependencies = runner_manifest
            .get("dependencies")
            .and_then(toml::Value::as_table)
            .expect("dependencies table");
        assert!(dependencies.contains_key("a"));
        assert!(
            !dependencies.contains_key("b"),
            "runner should not link unrequested crates: {dependencies:?}"
        );
    }

    #[test]
    fn workspace_print_discovery_handles_empty_and_skipped_crates() {
        let empty = WorkspaceCrates {
            workspace_info: WorkspaceInfo {
                root_dir: PathBuf::from("."),
                target_dir: PathBuf::from("./target"),
                crates: Vec::new(),
            },
            crates: Vec::new(),
            valid: Vec::new(),
            skipped: Vec::new(),
            package_not_found: None,
            all_i18n_package_names: Vec::new(),
        };
        assert!(!empty.print_discovery(|| {}));

        let skipped_crate = CrateInfo {
            name: es_fluent_runner::PackageName::try_new("missing-lib")
                .expect("valid package name"),
            manifest_dir: crate::core::ManifestDir::from_discovered(PathBuf::from("/tmp/test")),
            src_dir: crate::core::SourceDir::from_discovered(PathBuf::from("/tmp/test/src")),
            i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(
                PathBuf::from("/tmp/test/i18n.toml"),
            ),
            ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(PathBuf::from(
                "/tmp/test/i18n/en",
            )),
            has_lib_rs: false,
            fluent_features: Vec::new(),
        };
        let non_empty = WorkspaceCrates {
            workspace_info: WorkspaceInfo {
                root_dir: PathBuf::from("."),
                target_dir: PathBuf::from("./target"),
                crates: vec![skipped_crate.clone()],
            },
            crates: vec![skipped_crate.clone()],
            valid: Vec::new(),
            skipped: vec![skipped_crate],
            package_not_found: None,
            all_i18n_package_names: vec!["missing-lib".to_string()],
        };
        assert!(non_empty.print_discovery(|| {}));
    }

    #[test]
    fn run_generation_for_crates_returns_failures_when_runner_preparation_fails() {
        let krate = CrateInfo {
            name: es_fluent_runner::PackageName::try_new("broken").expect("valid package name"),
            manifest_dir: crate::core::ManifestDir::from_discovered(PathBuf::from("/dev/null")),
            src_dir: crate::core::SourceDir::from_discovered(PathBuf::from("/dev/null/src")),
            i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(
                PathBuf::from("/dev/null/i18n.toml"),
            ),
            ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(PathBuf::from(
                "/dev/null/i18n/en",
            )),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        };
        let workspace = WorkspaceInfo {
            root_dir: PathBuf::from("/dev/null"),
            target_dir: PathBuf::from("/dev/null/target"),
            crates: vec![krate.clone()],
        };

        let results = run_generation_for_crates(
            &workspace,
            std::slice::from_ref(&krate),
            &GenerationAction::Generate {
                mode: FluentParseMode::default(),
                dry_run: false,
            },
            false,
            false,
        );

        assert_eq!(results.len(), 1);
        assert!(results[0].error.is_some());
    }

    #[test]
    fn run_generation_for_crates_handles_empty_output_and_dry_run_render_paths() {
        let temp = crate::test_fixtures::create_test_crate_workspace_without_ftl();
        let workspace = create_workspace_info(&temp);
        let krate = workspace.crates[0].clone();

        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::silent_success(),
        );

        let results = run_generation_for_crates(
            &workspace,
            std::slice::from_ref(&krate),
            &GenerationAction::Generate {
                mode: FluentParseMode::default(),
                dry_run: true,
            },
            false,
            false,
        );
        assert_eq!(results.len(), 1);
        assert!(results[0].error.is_none());
        assert!(
            results[0].output.is_none(),
            "empty runner output should map to None"
        );

        let dry_run_has_errors =
            render_generation_results_with_dry_run(&results, true, GenerationVerb::Generate);
        assert!(!dry_run_has_errors);

        let clean_result = GenerateResult::success(
            package("crate-clean"),
            Duration::from_millis(1),
            1,
            None,
            true,
        );
        let clean_has_errors =
            render_generation_results_with_dry_run(&[clean_result], false, GenerationVerb::Clean);
        assert!(!clean_has_errors);
    }
}

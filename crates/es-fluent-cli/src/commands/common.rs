use crate::core::{CliError, CrateInfo, FluentParseMode, GenerateResult};
use crate::generation::generate_for_crate;
use crate::utils::{
    count_ftl_resources, discover_crates, filter_crates_by_package, partition_by_lib_rs, ui,
};
use clap::Args;
use rayon::prelude::*;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Args)]
pub struct WorkspaceArgs {
    /// Path to the crate or workspace root (defaults to current directory).
    #[arg(short, long)]
    pub path: Option<PathBuf>,
    /// Package name to filter (if in a workspace, only process this package).
    #[arg(short = 'P', long)]
    pub package: Option<String>,
}

/// Common arguments for locale-based processing commands.
///
/// Used by format, check, and sync commands.
#[derive(Debug, Clone, Args)]
pub struct LocaleProcessingArgs {
    /// Process all locales, not just the fallback language.
    #[arg(long)]
    pub all: bool,

    /// Dry run - show what would change without making changes.
    #[arg(long)]
    pub dry_run: bool,
}

/// Represents a resolved set of crates for a command to operate on.
#[derive(Debug, Clone)]
pub struct WorkspaceCrates {
    /// The user-supplied (or default) root path.
    pub path: PathBuf,
    /// All crates discovered (after optional package filtering).
    pub crates: Vec<CrateInfo>,
    /// Crates that are eligible for operations (contain `lib.rs`).
    pub valid: Vec<CrateInfo>,
    /// Crates that were skipped (missing `lib.rs`).
    pub skipped: Vec<CrateInfo>,
}

impl WorkspaceCrates {
    /// Discover crates for a command, applying the common filtering and partitioning logic.
    pub fn discover(args: WorkspaceArgs) -> Result<Self, CliError> {
        let path = args.path.unwrap_or_else(|| PathBuf::from("."));
        let crates = filter_crates_by_package(discover_crates(&path)?, args.package.as_ref());
        let (valid_refs, skipped_refs) = partition_by_lib_rs(&crates);
        let valid = valid_refs.into_iter().cloned().collect();
        let skipped = skipped_refs.into_iter().cloned().collect();

        Ok(Self {
            path,
            crates,
            valid,
            skipped,
        })
    }

    /// Print a standardized discovery summary, including skipped crates.
    ///
    /// Returns `false` when no crates were discovered to allow early-exit flows.
    pub fn print_discovery(&self, header: impl Fn()) -> bool {
        header();

        if self.crates.is_empty() {
            ui::print_discovered(&[]);
            return false;
        }

        ui::print_discovered(&self.crates);

        for krate in &self.skipped {
            ui::print_missing_lib_rs(&krate.name);
        }

        true
    }
}

/// Run generation-like work in parallel for a set of crates.
///
/// This mirrors the pattern used by both `generate` and `clean` commands, where
/// each crate is processed concurrently and the results are aggregated.
pub fn parallel_generate(crates: &[CrateInfo], mode: &FluentParseMode) -> Vec<GenerateResult> {
    crates
        .par_iter()
        .map(|krate| {
            let start = Instant::now();
            let result = generate_for_crate(krate, mode);
            let duration = start.elapsed();
            let resource_count = result
                .as_ref()
                .ok()
                .map(|_| count_ftl_resources(&krate.ftl_output_dir, &krate.name))
                .unwrap_or(0);

            match result {
                Ok(()) => GenerateResult::success(krate.name.clone(), duration, resource_count),
                Err(e) => GenerateResult::failure(krate.name.clone(), duration, e.to_string()),
            }
        })
        .collect()
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

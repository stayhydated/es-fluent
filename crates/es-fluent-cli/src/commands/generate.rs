//! Generate command implementation.

use crate::commands::{
    GenerationVerb, WorkspaceArgs, WorkspaceCrates, parallel_generate,
    render_generation_results_with_dry_run,
};
use crate::core::{CliError, FluentParseMode, GenerationAction};
use crate::utils::ui;
use clap::Parser;

/// Arguments for the generate command.
#[derive(Parser)]
pub struct GenerateArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Parse mode for FTL generation
    #[arg(short, long, value_enum, default_value_t = FluentParseMode::default())]
    pub mode: FluentParseMode,

    /// Dry run - show what would be generated without making changes.
    #[arg(long)]
    pub dry_run: bool,

    /// Force rebuild of the runner, ignoring the staleness cache.
    #[arg(long)]
    pub force_run: bool,
}

/// Run the generate command.
pub fn run_generate(args: GenerateArgs) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(args.workspace)?;

    if !workspace.print_discovery(ui::print_header) {
        return Ok(());
    }

    let results = parallel_generate(
        &workspace.workspace_info,
        &workspace.valid,
        &GenerationAction::Generate {
            mode: args.mode,
            dry_run: args.dry_run,
        },
        args.force_run,
    );
    let has_errors =
        render_generation_results_with_dry_run(&results, args.dry_run, GenerationVerb::Generate);

    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::cache::{RunnerCache, compute_content_hash};
    use std::fs;
    use std::time::SystemTime;
    use tempfile::tempdir;

    fn create_test_crate_workspace() -> tempfile::TempDir {
        let temp = tempdir().unwrap();

        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::create_dir_all(temp.path().join("i18n/en")).unwrap();
        fs::write(
            temp.path().join("Cargo.toml"),
            r#"[package]
name = "test-app"
version = "0.1.0"
edition = "2024"
"#,
        )
        .unwrap();
        fs::write(temp.path().join("src/lib.rs"), "pub struct Demo;\n").unwrap();
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .unwrap();
        fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n").unwrap();

        temp
    }

    #[cfg(unix)]
    fn set_executable(path: &std::path::Path) {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).expect("set permissions");
    }

    #[cfg(not(unix))]
    fn set_executable(_path: &std::path::Path) {}

    fn setup_fake_runner_and_cache(temp: &tempfile::TempDir) {
        let binary_path = temp.path().join("target/debug/es-fluent-runner");
        fs::create_dir_all(binary_path.parent().unwrap()).expect("create target/debug");
        fs::write(&binary_path, "#!/bin/sh\necho generated\n").expect("write runner");
        set_executable(&binary_path);

        let src_dir = temp.path().join("src");
        let i18n_toml = temp.path().join("i18n.toml");
        let hash = compute_content_hash(&src_dir, Some(&i18n_toml));
        let mtime = fs::metadata(&binary_path)
            .and_then(|m| m.modified())
            .expect("runner mtime")
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("mtime duration")
            .as_secs();

        let temp_dir = es_fluent_derive_core::get_es_fluent_temp_dir(temp.path());
        fs::create_dir_all(&temp_dir).expect("create temp dir");
        let mut crate_hashes = indexmap::IndexMap::new();
        crate_hashes.insert("test-app".to_string(), hash);
        RunnerCache {
            crate_hashes,
            runner_mtime: mtime,
            cli_version: env!("CARGO_PKG_VERSION").to_string(),
        }
        .save(&temp_dir)
        .expect("save runner cache");
    }

    #[test]
    fn run_generate_returns_ok_when_package_filter_matches_nothing() {
        let temp = create_test_crate_workspace();
        let result = run_generate(GenerateArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: Some("missing-crate".to_string()),
            },
            mode: FluentParseMode::default(),
            dry_run: false,
            force_run: false,
        });

        assert!(result.is_ok());
    }

    #[test]
    fn run_generate_executes_with_fake_runner() {
        let temp = create_test_crate_workspace();
        setup_fake_runner_and_cache(&temp);

        let result = run_generate(GenerateArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            mode: FluentParseMode::default(),
            dry_run: false,
            force_run: false,
        });

        assert!(result.is_ok());
    }
}

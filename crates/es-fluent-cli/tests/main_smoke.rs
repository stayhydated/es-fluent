mod fixtures;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

const SUBCOMMANDS: &[&str] = &[
    "generate",
    "watch",
    "clean",
    "fmt",
    "check",
    "status",
    "sync",
    "add-locale",
    "tree",
];

#[test]
fn binary_help_command_succeeds() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: cargo es-fluent <COMMAND>"))
        .stdout(predicate::str::contains("generate"));

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("generate"));
}

#[test]
fn binary_direct_invocation_accepts_subcommand_help() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["generate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Usage: cargo es-fluent generate [OPTIONS]",
        ));

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["format", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Usage: cargo es-fluent fmt [OPTIONS]",
        ));

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["help", "generate"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Generate FTL files once for all crates with i18n.toml",
        ));

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["help", "es-fluent"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: cargo es-fluent <COMMAND>"));

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["help", "es-fluent", "generate"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Usage: cargo es-fluent generate [OPTIONS]",
        ));

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "help", "es-fluent"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: cargo es-fluent <COMMAND>"));

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "help", "es-fluent", "generate"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Usage: cargo es-fluent generate [OPTIONS]",
        ));
}

#[test]
fn binary_version_output_uses_binary_name() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("cargo-es-fluent "))
        .stdout(predicate::str::contains("cargo ").not());

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "--version"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cargo-es-fluent "));
}

#[test]
fn binary_subcommand_help_succeeds_for_every_command() {
    for subcommand in SUBCOMMANDS {
        Command::cargo_bin("cargo-es-fluent")
            .expect("binary exists")
            .args(["es-fluent", subcommand, "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Usage:"));
    }
}

#[test]
fn binary_generate_help_describes_workspace_wide_package_filter() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "generate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Workspace package name to process, even when --path points inside a different member",
        ));
}

#[test]
fn binary_sync_help_describes_create_target_scope() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "sync", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Create missing target locale directories for explicit --locale targets; cannot be used with --all",
        ));
}

#[test]
fn binary_sync_help_describes_dry_run_locale_directories_and_keys() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "sync", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "show locale directories and keys that would be synced",
        ));
}

#[test]
fn binary_clean_help_describes_dry_run_orphan_removals() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "clean", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "show locale-file changes and orphan removals without making changes",
        ));
}

#[test]
fn binary_clean_help_describes_orphaned_scan_scope() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "clean", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "scans non-fallback locales even without --all",
        ));
}

#[test]
fn binary_check_help_describes_command_scope() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "check", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Validate FTL files, Rust-derived keys, and locale setup",
        ));
}

#[test]
fn binary_check_help_describes_all_scope() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "check", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Include non-fallback validation, fallback-copy warnings, and orphan-file checks",
        ));
}

#[test]
fn action_wrapper_rejects_invalid_boolean_inputs() {
    let action = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("action.yml"),
    )
    .expect("read action.yml");

    assert!(action.contains("action_bool all \"$ES_FLUENT_ALL\""));
    assert!(action.contains("no_fallback_copy_check:"));
    assert!(action.contains("ES_FLUENT_NO_FALLBACK_COPY_CHECK"));
    assert!(
        action.contains("action_bool no_fallback_copy_check \"$ES_FLUENT_NO_FALLBACK_COPY_CHECK\"")
    );
    assert!(action.contains("args+=(--no-fallback-copy-check)"));
    assert!(action.contains("action_bool force_run \"$ES_FLUENT_FORCE_RUN\""));
    assert!(action.contains("must be 'true' or 'false'"));
}

#[test]
fn public_action_usage_points_at_repository_owner() {
    let readme =
        std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"))
            .expect("read README.md");
    let book_cli = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../book/src/cli.md"),
    )
    .expect("read book CLI docs");

    assert!(readme.contains("uses: stayhydated/es-fluent/crates/es-fluent-cli@"));
    assert!(!readme.contains("stayhydrated/es-fluent"));
    assert!(readme.contains("`no_fallback_copy_check`"));
    assert!(book_cli.contains("uses: stayhydated/es-fluent/crates/es-fluent-cli@"));
    assert!(!book_cli.contains("stayhydrated/es-fluent"));
    assert!(book_cli.contains("`no_fallback_copy_check`"));
    assert!(readme.contains("cargo es-fluent tree\ncargo es-fluent tree --all"));
    assert!(book_cli.contains("cargo es-fluent tree\ncargo es-fluent tree --all"));
}

#[test]
fn public_cli_docs_keep_common_usage_sentences_readable() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let docs = [
        ("cli README", manifest_dir.join("README.md")),
        ("book CLI", manifest_dir.join("../../book/src/cli.md")),
        (
            "CLI skill reference",
            manifest_dir.join("../../skills/use-es-fluent/references/cli-workflow.md"),
        ),
        ("root README", manifest_dir.join("../../README.md")),
    ];

    for (name, path) in docs {
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {name} at {}: {error}", path.display()));
        for awkward_split in [
            "If\n`--package`",
            "When fallback files use\nnamespaces",
            "not\nsymlinks or directories",
            "every\nselected crate",
        ] {
            assert!(
                !content.contains(awkward_split),
                "{name} should not split a common CLI usage sentence at {awkward_split:?}"
            );
        }
    }
}

#[test]
fn binary_check_help_describes_fallback_copy_scope() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "check", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Disable --all warnings for non-fallback messages that match the fallback locale; requires --all",
        ));
}

#[test]
fn binary_check_rejects_no_fallback_copy_check_without_all_before_workspace_discovery() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "check", "--no-fallback-copy-check"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("--no-fallback-copy-check"))
        .stderr(predicate::str::contains("--all"))
        .stderr(predicate::str::contains("requires"));

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            "/definitely/missing/path",
            "--no-fallback-copy-check",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "command_error");
    let help = json["issues"][0]["help"].as_str().expect("issue help");
    assert!(help.contains("--no-fallback-copy-check requires --all"));
    assert!(
        !help.contains("/definitely/missing/path"),
        "check should reject the flag combination before workspace discovery"
    );
}

#[test]
fn binary_status_help_describes_all_scope() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "status", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Include non-fallback formatting, sync, orphan-file, and validation checks",
        ));
}

#[test]
fn binary_tree_help_describes_hide_entry_detail_scope() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "tree", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Hide attributes under message and term entries",
        ))
        .stdout(predicate::str::contains(
            "Hide variables used by each message or term entry",
        ));
}

#[test]
fn binary_tree_help_describes_link_mode_scope() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "tree", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Text hyperlink target mode for message, attribute, and variable rows",
        ));
}

#[test]
fn binary_watch_help_matches_supported_generation_options() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "watch", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--mode <MODE>"))
        .stdout(predicate::str::contains(
            "aggressive overwrites existing translations",
        ))
        .stdout(predicate::str::contains("--dry-run").not())
        .stdout(predicate::str::contains("--force-run").not());
}

#[test]
fn binary_every_command_has_a_noninteractive_success_path() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let cases: &[(&str, &[&str])] = &[
        ("generate", &["generate", "--path", workspace, "--dry-run"]),
        ("watch", &["watch", "--help"]),
        ("clean", &["clean", "--path", workspace, "--dry-run"]),
        ("fmt", &["fmt", "--path", workspace]),
        (
            "check",
            &["check", "--path", workspace, "--package", "missing-package"],
        ),
        (
            "status",
            &["status", "--path", workspace, "--output", "json"],
        ),
        (
            "sync",
            &[
                "sync",
                "--path",
                workspace,
                "--locale",
                "fr-FR",
                "--create",
                "--dry-run",
            ],
        ),
        (
            "add-locale",
            &["add-locale", "--path", workspace, "--dry-run", "fr-FR"],
        ),
        ("tree", &["tree", "--path", workspace]),
    ];

    assert_eq!(
        cases.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
        SUBCOMMANDS
    );

    for (_, args) in cases {
        Command::cargo_bin("cargo-es-fluent")
            .expect("binary exists")
            .args(std::iter::once("es-fluent").chain(args.iter().copied()))
            .assert()
            .success();
    }
}

#[test]
fn binary_write_commands_reject_missing_package_filter() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let cases: &[&[&str]] = &[
        &[
            "generate",
            "--path",
            workspace,
            "--package",
            "missing-package",
        ],
        &["watch", "--path", workspace, "--package", "missing-package"],
        &["clean", "--path", workspace, "--package", "missing-package"],
        &["fmt", "--path", workspace, "--package", "missing-package"],
        &[
            "sync",
            "--path",
            workspace,
            "--package",
            "missing-package",
            "--all",
        ],
    ];

    for args in cases {
        Command::cargo_bin("cargo-es-fluent")
            .expect("binary exists")
            .args(std::iter::once("es-fluent").chain(args.iter().copied()))
            .assert()
            .failure()
            .stderr(predicate::str::contains("missing-package"));
    }
}

#[test]
fn binary_fmt_json_reports_missing_package_filter() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            workspace,
            "--package",
            "missing-package",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["dry_run"], false);
    assert_eq!(json["formatted_count"], 0);
    assert_eq!(json["unchanged_count"], 0);
    assert_eq!(json["error_count"], 1);
    assert!(json["files"].as_array().is_some_and(Vec::is_empty));
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("missing-package"))
    );
}

#[test]
fn binary_generate_with_invalid_path_fails() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "generate",
            "--path",
            "/definitely/missing/path",
        ])
        .assert()
        .failure();
}

#[test]
fn binary_fmt_json_invalid_path_includes_requested_path() {
    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            "/definitely/missing/path",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"][0].as_str().is_some_and(|message| {
        message.contains("Failed to canonicalize root directory")
            && message.contains("/definitely/missing/path")
    }));
}

#[test]
fn binary_fmt_path_inside_workspace_member_scopes_to_that_member() {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    for name in ["a", "b"] {
        let crate_dir = temp.path().join(name);
        std::fs::create_dir_all(crate_dir.join("src")).expect("create src");
        std::fs::create_dir_all(crate_dir.join("i18n/en")).expect("create fallback locale");
        std::fs::write(
            crate_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write crate manifest");
        std::fs::write(crate_dir.join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
        std::fs::write(
            crate_dir.join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n config");
        std::fs::write(
            crate_dir.join(format!("i18n/en/{name}.ftl")),
            "hello = Hello\n",
        )
        .expect("write ftl");
    }

    let nested_member_path = temp
        .path()
        .join("a/src")
        .to_str()
        .expect("nested path")
        .to_string();
    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            &nested_member_path,
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    let files = json["files"].as_array().expect("files array");
    assert_eq!(files.len(), 1);
    let path = files[0]["path"].as_str().expect("file path");
    assert_eq!(path, "a/i18n/en/a.ftl");
    assert!(
        !path.contains(temp.path().to_string_lossy().as_ref()),
        "fmt JSON file paths should be relative: {path}"
    );

    #[cfg(unix)]
    {
        let outside = fixtures::tempdir();
        let symlinked_member_path = temp.path().join("a/src/external");
        std::os::unix::fs::symlink(outside.path(), &symlinked_member_path)
            .expect("create symlink inside member");
        let symlinked_member_path = symlinked_member_path
            .to_str()
            .expect("symlinked member path")
            .to_string();
        let output = Command::cargo_bin("cargo-es-fluent")
            .expect("binary exists")
            .args([
                "es-fluent",
                "fmt",
                "--path",
                &symlinked_member_path,
                "--output",
                "json",
            ])
            .assert()
            .success()
            .stderr(predicate::str::is_empty())
            .get_output()
            .stdout
            .clone();

        let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
        let files = json["files"].as_array().expect("files array");
        assert_eq!(files.len(), 1);
        let path = files[0]["path"].as_str().expect("file path");
        assert_eq!(path, "a/i18n/en/a.ftl");
    }

    let nested_member_file = temp
        .path()
        .join("a/src/lib.rs")
        .to_str()
        .expect("nested file path")
        .to_string();
    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            &nested_member_file,
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    let files = json["files"].as_array().expect("files array");
    assert_eq!(files.len(), 1);
    let path = files[0]["path"].as_str().expect("file path");
    assert_eq!(path, "a/i18n/en/a.ftl");
    assert!(
        !path.contains(temp.path().to_string_lossy().as_ref()),
        "fmt JSON file paths should be relative: {path}"
    );

    let workspace_manifest = temp
        .path()
        .join("Cargo.toml")
        .to_str()
        .expect("workspace manifest path")
        .to_string();
    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            &workspace_manifest,
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    let files = json["files"].as_array().expect("files array");
    assert_eq!(files.len(), 2);
    let paths = files
        .iter()
        .map(|file| file["path"].as_str().expect("file path"))
        .collect::<Vec<_>>();
    assert!(paths.contains(&"a/i18n/en/a.ftl"));
    assert!(paths.contains(&"b/i18n/en/b.ftl"));
    assert!(
        paths
            .iter()
            .all(|path| !path.contains(temp.path().to_string_lossy().as_ref())),
        "fmt JSON file paths should be relative: {paths:?}"
    );

    std::fs::create_dir_all(temp.path().join("tools")).expect("create workspace subdir");
    let workspace_subdir = temp
        .path()
        .join("tools")
        .to_str()
        .expect("workspace subdir")
        .to_string();
    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            &workspace_subdir,
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(json["files"].as_array().is_some_and(Vec::is_empty));
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("no crates with i18n.toml were found"))
    );
}

fn create_workspace_with_invalid_i18n_sibling() -> assert_fs::TempDir {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    for name in ["a", "b"] {
        let crate_dir = temp.path().join(name);
        std::fs::create_dir_all(crate_dir.join("src")).expect("create src");
        std::fs::create_dir_all(crate_dir.join("i18n/en")).expect("create fallback locale");
        std::fs::write(
            crate_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write crate manifest");
        std::fs::write(crate_dir.join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
        std::fs::write(
            crate_dir.join(format!("i18n/en/{name}.ftl")),
            "hello = Hello\n",
        )
        .expect("write ftl");
    }

    std::fs::write(
        temp.path().join("a/i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write valid i18n config");
    std::fs::write(temp.path().join("b/i18n.toml"), "not = [valid\n")
        .expect("write invalid sibling i18n config");

    temp
}

#[test]
fn binary_member_path_ignores_invalid_i18n_toml_in_unselected_sibling() {
    let temp = create_workspace_with_invalid_i18n_sibling();
    let member_path = temp.path().join("a/src");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            member_path.to_str().expect("member path"),
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 0);
    assert_eq!(json["crates"].as_array().expect("crates array").len(), 1);
    assert_eq!(json["crates"][0]["name"], "a");
}

fn create_workspace_with_shared_locale_root() -> assert_fs::TempDir {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    for name in ["a", "b"] {
        let crate_dir = temp.path().join(name);
        std::fs::create_dir_all(crate_dir.join("src")).expect("create src");
        std::fs::write(
            crate_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write crate manifest");
        std::fs::write(crate_dir.join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    }

    std::fs::create_dir_all(temp.path().join("a/i18n/en")).expect("create fallback locale");
    std::fs::create_dir_all(temp.path().join("a/i18n/fr")).expect("create target locale");
    std::fs::write(
        temp.path().join("a/i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write a i18n config");
    std::fs::write(
        temp.path().join("b/i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"../a/i18n\"\n",
    )
    .expect("write b i18n config");
    for (locale, name, key) in [
        ("en", "a", "hello = Hello\n"),
        ("en", "b", "bye = Bye\n"),
        ("fr", "a", "hello = Bonjour\n"),
        ("fr", "b", "bye = Salut\n"),
    ] {
        std::fs::write(temp.path().join(format!("a/i18n/{locale}/{name}.ftl")), key)
            .expect("write shared locale ftl");
    }

    temp
}

#[test]
fn binary_clean_orphaned_package_filter_preserves_unselected_sibling_files() {
    let temp = create_workspace_with_shared_locale_root();
    let sibling_file = temp.path().join("a/i18n/fr/b.ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--package",
            "a",
            "--orphaned",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Would remove orphaned file: b.ftl").not());

    assert!(
        sibling_file.exists(),
        "package-scoped orphan cleanup must preserve configured sibling FTL files"
    );
}

#[test]
fn binary_generate_package_filter_does_not_link_unselected_crates() {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    for name in ["a", "b"] {
        std::fs::create_dir_all(temp.path().join(format!("{name}/src"))).expect("create src");
        std::fs::create_dir_all(temp.path().join(format!("{name}/i18n/en"))).expect("create i18n");
        std::fs::write(
            temp.path().join(format!("{name}/Cargo.toml")),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write manifest");
        std::fs::write(
            temp.path().join(format!("{name}/i18n.toml")),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n config");
        std::fs::write(
            temp.path().join(format!("{name}/i18n/en/{name}.ftl")),
            "hello = Hello\n",
        )
        .expect("write ftl");
    }
    std::fs::write(temp.path().join("a/src/lib.rs"), "pub fn marker() {}\n").expect("write a lib");
    std::fs::write(temp.path().join("b/src/lib.rs"), "this is not rust\n").expect("write b lib");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "generate",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--package",
            "a",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Discovered 1 crate(s)"))
        .stderr(predicate::str::contains("could not compile `b`").not());
}

#[test]
fn binary_status_package_filter_does_not_link_unselected_crates() {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    for name in ["a", "b"] {
        std::fs::create_dir_all(temp.path().join(format!("{name}/src"))).expect("create src");
        std::fs::create_dir_all(temp.path().join(format!("{name}/i18n/en"))).expect("create i18n");
        std::fs::write(
            temp.path().join(format!("{name}/Cargo.toml")),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write manifest");
        std::fs::write(
            temp.path().join(format!("{name}/i18n.toml")),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n config");
        std::fs::write(
            temp.path().join(format!("{name}/i18n/en/{name}.ftl")),
            "hello = Hello\n",
        )
        .expect("write ftl");
    }
    std::fs::write(temp.path().join("a/src/lib.rs"), "pub fn marker() {}\n").expect("write a lib");
    std::fs::write(temp.path().join("b/src/lib.rs"), "this is not rust\n").expect("write b lib");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--package",
            "a",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("could not compile `b`").not())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["crates_discovered"], 1);
    assert_eq!(json["crates_checked"], 1);
    assert_eq!(json["clean"], true);
    assert_eq!(json["generation_errors"], Value::Array(Vec::new()));
}

#[test]
fn binary_status_all_json_counts_same_sync_locale_in_multiple_workspace_crates() {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    for name in ["a", "b"] {
        std::fs::create_dir_all(temp.path().join(format!("{name}/src"))).expect("create src");
        std::fs::create_dir_all(temp.path().join(format!("{name}/i18n/en"))).expect("create en");
        std::fs::create_dir_all(temp.path().join(format!("{name}/i18n/fr"))).expect("create fr");
        std::fs::write(
            temp.path().join(format!("{name}/Cargo.toml")),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write manifest");
        std::fs::write(
            temp.path().join(format!("{name}/src/lib.rs")),
            "pub fn marker() {}\n",
        )
        .expect("write lib");
        std::fs::write(
            temp.path().join(format!("{name}/i18n.toml")),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write config");
        std::fs::write(
            temp.path().join(format!("{name}/i18n/en/{name}.ftl")),
            "hello = Hello\nworld = World\n",
        )
        .expect("write fallback");
        std::fs::write(
            temp.path().join(format!("{name}/i18n/fr/{name}.ftl")),
            "hello = Bonjour\n",
        )
        .expect("write incomplete fr");
    }

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["clean"], false);
    assert_eq!(json["missing_synced_keys"], 2);
    assert_eq!(json["locales_need_sync"], 2);
}

#[test]
fn binary_fmt_dry_run_json_reports_preview_mode_without_writing() {
    let temp = fixtures::create_workspace();
    let ftl_path = temp.path().join("i18n/en/test-app.ftl");
    std::fs::write(&ftl_path, "z-last = Z\na-first = A\n").expect("write unsorted ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--dry-run",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["formatted_count"], 1);
    assert_eq!(json["files"][0]["changed"], true);
    assert_eq!(
        std::fs::read_to_string(&ftl_path).expect("read ftl"),
        "z-last = Z\na-first = A\n"
    );
}

#[test]
fn binary_fmt_reports_binary_only_crate_as_notice_without_skipping() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create i18n");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"bin-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write Cargo.toml");
    std::fs::write(temp.path().join("src/main.rs"), "fn main() {}\n").expect("write main");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n.toml");
    let ftl_path = temp.path().join("i18n/en/bin-app.ftl");
    std::fs::write(&ftl_path, "z-last = Z\na-first = A\n").expect("write unsorted ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Notice bin-app (missing library target)",
        ))
        .stdout(predicate::str::contains("Skipping bin-app").not())
        .stdout(predicate::str::contains("Formatted:"));

    assert_eq!(
        std::fs::read_to_string(&ftl_path).expect("read formatted ftl"),
        "a-first = A\nz-last = Z\n"
    );
}

#[test]
fn binary_fmt_all_json_reports_noncanonical_locale_directory() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::create_dir_all(temp.path().join("i18n/en-us")).expect("create bad locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"fmt-locale\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write Cargo.toml");
    std::fs::write(temp.path().join("src/main.rs"), "fn main() {}\n").expect("write main");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n.toml");
    std::fs::write(
        temp.path().join("i18n/en/fmt-locale.ftl"),
        "hello = Hello\n",
    )
    .expect("write fallback ftl");
    std::fs::write(
        temp.path().join("i18n/en-us/fmt-locale.ftl"),
        "hello = Hello\n",
    )
    .expect("write bad locale ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("en-us") && message.contains("en-US"))
    );
}

#[test]
fn binary_generate_rejects_fallback_locale_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback dir");
    std::fs::write(temp.path().join("i18n/en"), "not a directory\n").expect("write fallback file");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "generate",
            "--path",
            temp.path().to_str().expect("workspace path"),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("generation path"))
        .stderr(predicate::str::contains("fallback locale path 'en'"));

    assert!(temp.path().join("i18n/en").is_file());
}

#[test]
fn binary_generate_rejects_assets_dir_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "generate",
            "--path",
            temp.path().to_str().expect("workspace path"),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("generation path"))
        .stderr(predicate::str::contains("assets_dir"));

    assert!(temp.path().join("i18n").is_file());
}

#[cfg(unix)]
#[test]
fn binary_generate_rejects_symlinked_runner_metadata_dir_without_writing_target() {
    let temp = fixtures::create_workspace();
    let outside = fixtures::tempdir();
    std::os::unix::fs::symlink(outside.path(), temp.path().join(".es-fluent"))
        .expect("create .es-fluent symlink");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "generate",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--dry-run",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(".es-fluent"))
        .stderr(predicate::str::contains("symlink"));

    assert!(!outside.path().join("Cargo.toml").exists());
    assert!(!outside.path().join("src/main.rs").exists());
    assert!(temp.path().join(".es-fluent").is_symlink());
}

#[test]
fn binary_generate_rejects_fallback_ftl_path_as_directory_before_runner() {
    let temp = fixtures::create_workspace();
    let ftl_path = temp.path().join("i18n/en/test-app.ftl");
    std::fs::remove_file(&ftl_path).expect("remove ftl file");
    std::fs::create_dir(&ftl_path).expect("create ftl directory");
    std::fs::write(temp.path().join("src/lib.rs"), "this is not rust\n").expect("break Rust");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "generate",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--dry-run",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("generation path"))
        .stderr(predicate::str::contains("fallback locale FTL layout"))
        .stderr(predicate::str::contains("Expected FTL path"))
        .stderr(predicate::str::contains("could not compile").not());

    assert!(
        !temp.path().join(".es-fluent").exists(),
        "generate should reject invalid FTL paths before runner metadata"
    );
    assert!(
        !temp.path().join("target").exists(),
        "generate should reject invalid FTL paths before Cargo runs"
    );
}

#[test]
fn binary_status_json_reports_locale_named_asset_path_without_all() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback ftl");
    std::fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["clean"], false);
    assert_eq!(json["crates_checked"], 0);
    assert_eq!(json["format_errors"], Value::Array(Vec::new()));
    assert!(json["setup_errors"][0].as_str().is_some_and(|message| {
        message.contains("locale path 'fr'")
            && message.contains("i18n/fr")
            && !message.contains(temp.path().to_str().expect("workspace path"))
    }));
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "status should not prepare the runner cache after setup errors"
    );
    assert!(
        !temp.path().join("target").exists(),
        "status should not run Cargo after setup errors"
    );
}

#[test]
fn binary_status_json_reports_locale_named_asset_path_as_setup_error() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback ftl");
    std::fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["clean"], false);
    assert!(
        json["setup_errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("locale path 'fr'"))
    );
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "status should not prepare the runner cache after setup errors"
    );
    assert!(
        !temp.path().join("target").exists(),
        "status should not run Cargo after setup errors"
    );
}

#[test]
fn binary_status_json_reports_assets_dir_path_as_file_without_runner() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["clean"], false);
    assert_eq!(json["crates_checked"], 0);
    assert_eq!(json["format_errors"], Value::Array(Vec::new()));
    assert!(json["setup_errors"][0].as_str().is_some_and(|message| {
        message.contains("Assets path")
            && message.contains("i18n")
            && message.contains("not a directory")
            && !message.contains(temp.path().to_str().expect("workspace path"))
    }));
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "status should not prepare runner metadata after assets_dir setup errors"
    );
    assert!(
        !temp.path().join("target").exists(),
        "status should not run Cargo after assets_dir setup errors"
    );
}

#[test]
fn binary_status_json_reports_missing_fallback_locale_as_setup_error() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n")).expect("create assets dir");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["clean"], false);
    assert_eq!(json["crates_checked"], 0);
    assert!(json["setup_errors"][0].as_str().is_some_and(|message| {
        message.contains("fallback locale directory 'en'")
            && message.contains("missing or not a directory")
    }));
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "status should not prepare the runner cache after setup errors"
    );
    assert!(
        !temp.path().join("target").exists(),
        "status should not run Cargo after setup errors"
    );
}

#[test]
fn binary_status_json_reports_ftl_path_directory_as_setup_error() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en/test-app.ftl"))
        .expect("create ftl directory");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["clean"], false);
    assert_eq!(json["crates_checked"], 0);
    assert_eq!(json["format_errors"], Value::Array(Vec::new()));
    assert_eq!(json["validation_errors"], 0);
    assert!(
        json["setup_errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("Expected FTL path to be a file")
                && message.contains("test-app.ftl"))
    );
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "status should not prepare the runner cache after setup errors"
    );
    assert!(
        !temp.path().join("target").exists(),
        "status should not run Cargo after setup errors"
    );
}

#[test]
fn binary_status_json_reports_binary_only_crate_as_setup_error() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"binary-only-status\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"binary-only-status\"\npath = \"src/main.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/main.rs"), "fn main() {}\n").expect("write main");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        temp.path().join("i18n/en/binary-only-status.ftl"),
        "hello = Hello\n",
    )
    .expect("write fallback ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["clean"], false);
    assert_eq!(json["crates_checked"], 0);
    assert_eq!(json["validation_errors"], 0);
    assert!(
        json["setup_errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("no Cargo library target"))
    );
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "status should not prepare the runner cache after setup errors"
    );
    assert!(
        !temp.path().join("target").exists(),
        "status should not run Cargo after setup errors"
    );
}

#[test]
fn binary_status_json_reports_invalid_i18n_config_as_setup_error() {
    let temp = fixtures::tempdir();
    let outside_name = format!(
        "{}-status-outside-assets",
        temp.path()
            .file_name()
            .expect("temp name")
            .to_string_lossy()
    );
    let outside = temp
        .path()
        .parent()
        .expect("temp parent")
        .join(&outside_name);
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(outside.join("i18n/en")).expect("create outside assets");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        format!("fallback_language = \"en\"\nassets_dir = \"../{outside_name}/i18n\"\n"),
    )
    .expect("write invalid config");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["crates_discovered"], 0);
    assert_eq!(json["clean"], false);
    assert_eq!(
        json["setup_errors"].as_array().expect("setup errors").len(),
        1
    );
    assert!(json["setup_errors"][0].as_str().is_some_and(|message| {
        message.contains("Invalid assets_dir") && message.contains("crate root")
    }));

    let _ = std::fs::remove_dir_all(outside);
}

#[test]
fn binary_json_read_commands_report_invalid_i18n_config_as_json() {
    let temp = fixtures::tempdir();
    let outside_name = format!(
        "{}-read-json-outside-assets",
        temp.path()
            .file_name()
            .expect("temp name")
            .to_string_lossy()
    );
    let outside = temp
        .path()
        .parent()
        .expect("temp parent")
        .join(&outside_name);
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(outside.join("i18n/en")).expect("create outside assets");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        format!("fallback_language = \"en\"\nassets_dir = \"../{outside_name}/i18n\"\n"),
    )
    .expect("write invalid config");

    let cases: &[(&str, &[&str])] = &[
        ("check", &["check"]),
        ("sync", &["sync", "--all"]),
        ("tree", &["tree"]),
    ];

    for (command, command_args) in cases {
        let output = Command::cargo_bin("cargo-es-fluent")
            .expect("binary exists")
            .arg("es-fluent")
            .args(*command_args)
            .args([
                "--path",
                temp.path().to_str().expect("workspace path"),
                "--output",
                "json",
            ])
            .assert()
            .failure()
            .stderr(predicate::str::is_empty())
            .get_output()
            .stdout
            .clone();
        let json: Value = serde_json::from_slice(&output).expect("stdout is JSON only");

        match *command {
            "check" => {
                assert_eq!(json["error_count"], 1);
                assert_eq!(json["issues"][0]["kind"], "setup_error");
                assert_eq!(json["issues"][0]["source"], "workspace");
                assert!(json["issues"][0]["help"].as_str().is_some_and(|message| {
                    message.contains("Invalid assets_dir") && message.contains("crate root")
                }));
            },
            "sync" => {
                assert_eq!(json["error_count"], 1);
                assert!(json["errors"][0].as_str().is_some_and(|message| {
                    message.contains("Invalid assets_dir") && message.contains("crate root")
                }));
            },
            "tree" => {
                assert_eq!(json["error_count"], 1);
                assert_eq!(json["errors"][0]["crate_name"], "workspace");
                assert!(
                    json["errors"][0]["message"]
                        .as_str()
                        .is_some_and(|message| {
                            message.contains("Invalid assets_dir") && message.contains("crate root")
                        })
                );
            },
            _ => unreachable!("covered commands"),
        }
    }

    let _ = std::fs::remove_dir_all(outside);
}

#[test]
fn binary_status_all_json_reports_noncanonical_locale_directory_as_setup_error() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::create_dir_all(temp.path().join("i18n/en-us")).expect("create bad locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback ftl");
    std::fs::write(
        temp.path().join("i18n/en-us/test-app.ftl"),
        "hello = Hello\n",
    )
    .expect("write noncanonical locale ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["clean"], false);
    assert!(
        json["setup_errors"]
            .as_array()
            .expect("setup errors array")
            .iter()
            .any(|message| message
                .as_str()
                .is_some_and(|message| message.contains("en-us") && message.contains("en-US")))
    );
}

#[test]
fn binary_status_all_json_reports_orphans_outside_validation_errors() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create target locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub struct Demo;\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/fr/orphan.ftl"), "orphan = Orphan\n")
        .expect("write orphan ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["clean"], false);
    assert_eq!(json["validation_errors"], 0);
    assert_eq!(json["validation_warnings"], 0);
    assert!(json["orphaned_files"].as_array().is_some_and(|files| {
        files.len() == 1
            && files[0]
                .as_str()
                .is_some_and(|path| path.ends_with("i18n/fr/orphan.ftl"))
    }));
}

#[test]
fn binary_check_json_reports_locale_named_asset_path_without_all() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback ftl");
    std::fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "validation_execution");
    assert_eq!(json["crates_checked"], 0);
    assert!(json["issues"][0]["help"].as_str().is_some_and(|message| {
        message.contains("Locale path 'fr'")
            && message.contains("i18n/fr")
            && !message.contains(temp.path().to_str().expect("workspace path"))
    }));
}

#[test]
fn binary_check_all_json_reports_locale_named_asset_path_as_error() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback ftl");
    std::fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "validation_execution");
    assert!(json["issues"][0]["help"].as_str().is_some_and(|message| {
        message.contains("Locale path 'fr'")
            && message.contains("i18n/fr")
            && !message.contains(temp.path().to_str().expect("workspace path"))
    }));
}

#[test]
fn binary_check_all_json_reports_assets_dir_path_as_one_error() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "validation_execution");
    assert!(json["issues"][0]["help"].as_str().is_some_and(|message| {
        message.contains("assets_dir for test-app")
            && message.contains("i18n")
            && !message.contains(temp.path().to_str().expect("workspace path"))
    }));
}

#[test]
fn binary_check_json_reports_setup_error_before_uncompilable_rust() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"bad-check\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "this is not rust\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["crates_discovered"], 1);
    assert_eq!(json["crates_checked"], 0);
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "validation_execution");
    assert!(
        json["issues"][0]["help"]
            .as_str()
            .is_some_and(|message| message.contains("assets_dir for bad-check"))
    );
    assert!(
        !json["issues"][0]["help"]
            .as_str()
            .is_some_and(|message| message.contains("could not compile"))
    );
}

#[test]
fn binary_check_json_reports_ftl_path_directory_before_runner() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en/bad-check-ftl.ftl"))
        .expect("create ftl directory");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"bad-check-ftl\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "this is not rust\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["crates_discovered"], 1);
    assert_eq!(json["crates_checked"], 0);
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "validation_execution");
    assert!(json["issues"][0]["help"].as_str().is_some_and(|message| {
        message.contains("FTL file layout")
            && message.contains("Expected FTL path")
            && message.contains("i18n/en/bad-check-ftl.ftl")
            && !message.contains(temp.path().to_str().expect("workspace path"))
    }));
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "check should not prepare runner metadata after FTL setup errors"
    );
    assert!(
        !temp.path().join("target").exists(),
        "check should not run Cargo after FTL setup errors"
    );
}

#[test]
fn binary_check_all_json_reports_noncanonical_locale_dir_before_uncompilable_rust() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::create_dir_all(temp.path().join("i18n/en-us")).expect("create bad locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"bad-check-locale\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "this is not rust\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        temp.path().join("i18n/en/bad-check-locale.ftl"),
        "hello = Hello\n",
    )
    .expect("write fallback ftl");
    std::fs::write(
        temp.path().join("i18n/en-us/bad-check-locale.ftl"),
        "hello = Hello\n",
    )
    .expect("write noncanonical locale ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["crates_discovered"], 1);
    assert_eq!(json["crates_checked"], 0);
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "validation_execution");
    assert!(
        json["issues"][0]["help"]
            .as_str()
            .is_some_and(|message| message.contains("en-us") && message.contains("en-US"))
    );
    assert!(
        !json["issues"][0]["help"]
            .as_str()
            .is_some_and(|message| message.contains("could not compile"))
    );
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "check should not prepare runner metadata after locale setup errors"
    );
    assert!(
        !temp.path().join("target").exists(),
        "check should not run Cargo after locale setup errors"
    );
}

#[test]
fn binary_check_json_reports_valid_crate_orphans_with_other_setup_errors() {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    std::fs::create_dir_all(temp.path().join("a/src")).expect("create a src");
    std::fs::create_dir_all(temp.path().join("a/i18n/en")).expect("create a fallback locale");
    std::fs::create_dir_all(temp.path().join("a/i18n/fr")).expect("create a target locale");
    std::fs::write(
        temp.path().join("a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write a manifest");
    std::fs::write(temp.path().join("a/src/lib.rs"), "pub fn marker() {}\n").expect("write a lib");
    std::fs::write(
        temp.path().join("a/i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write a config");
    std::fs::write(temp.path().join("a/i18n/en/a.ftl"), "hello = Hello\n")
        .expect("write a fallback ftl");
    std::fs::write(temp.path().join("a/i18n/fr/a.ftl"), "hello = Bonjour\n")
        .expect("write a target ftl");
    std::fs::write(
        temp.path().join("a/i18n/fr/orphan.ftl"),
        "orphan = Orphan\n",
    )
    .expect("write orphan ftl");

    std::fs::create_dir_all(temp.path().join("b/src")).expect("create b src");
    std::fs::write(
        temp.path().join("b/Cargo.toml"),
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write b manifest");
    std::fs::write(temp.path().join("b/src/lib.rs"), "this is not rust\n").expect("write b lib");
    std::fs::write(
        temp.path().join("b/i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write b config");
    std::fs::write(temp.path().join("b/i18n"), "not a directory\n").expect("write b assets file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["crates_discovered"], 2);
    assert_eq!(json["crates_checked"], 1);
    assert!(
        json["issues"].as_array().is_some_and(|issues| {
            issues.iter().any(|issue| {
                issue["kind"] == "validation_execution"
                    && issue["help"]
                        .as_str()
                        .is_some_and(|message| message.contains("assets_dir for b"))
            }) && issues.iter().any(|issue| {
                issue["kind"] == "orphaned_file"
                    && issue["source"]
                        .as_str()
                        .is_some_and(|source| source.ends_with("a/i18n/fr/orphan.ftl"))
            })
        }),
        "expected setup and orphan issues, got {json}"
    );
    assert!(
        !json["issues"].as_array().is_some_and(|issues| {
            issues.iter().any(|issue| {
                issue["help"]
                    .as_str()
                    .is_some_and(|message| message.contains("could not compile"))
            })
        }),
        "setup-invalid crate b should not be linked into the runner, got {json}"
    );
}

#[test]
fn binary_check_json_reports_missing_fallback_locale_as_json() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "validation_execution");
    assert!(json["issues"][0]["help"].as_str().is_some_and(|message| {
        message.contains("fallback locale directory 'en'")
            && message.contains("i18n/en")
            && !message.contains(temp.path().to_str().expect("workspace path"))
    }));
}

#[cfg(unix)]
#[test]
fn binary_check_json_reports_symlinked_fallback_locale_as_json() {
    let temp = fixtures::create_workspace();
    let outside = fixtures::tempdir();
    std::fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");
    std::fs::create_dir_all(outside.path().join("en")).expect("create outside locale");
    std::os::unix::fs::symlink(outside.path().join("en"), temp.path().join("i18n/en"))
        .expect("create fallback locale symlink");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "validation_execution");
    assert!(json["issues"][0]["help"].as_str().is_some_and(|message| {
        message.contains("fallback locale directory 'en'")
            && message.contains("i18n/en")
            && !message.contains("Locale path 'en'")
            && !message.contains(temp.path().to_str().expect("workspace path"))
    }));
}

#[test]
fn binary_fmt_rejects_configured_assets_dir_outside_crate() {
    let temp = fixtures::tempdir();
    let outside_name = format!(
        "{}-configured-outside-assets",
        temp.path()
            .file_name()
            .expect("temp name")
            .to_string_lossy()
    );
    let outside = temp
        .path()
        .parent()
        .expect("temp parent")
        .join(&outside_name);
    let outside_assets = outside.join("i18n/en");
    let outside_ftl = outside_assets.join("asset-config-escape.ftl");
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(&outside_assets).expect("create outside assets");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"asset-config-escape\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        format!("fallback_language = \"en\"\nassets_dir = \"../{outside_name}/i18n\"\n"),
    )
    .expect("write escaping i18n config");
    std::fs::write(&outside_ftl, "b = B\na = A\n").expect("write outside ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid assets_dir"))
        .stderr(predicate::str::contains("crate root"));

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"][0].as_str().is_some_and(
        |message| message.contains("Invalid assets_dir") && message.contains("crate root")
    ));

    let outside_content = std::fs::read_to_string(&outside_ftl).expect("read outside ftl");
    assert_eq!(outside_content, "b = B\na = A\n");
    let _ = std::fs::remove_dir_all(outside);
}

#[cfg(unix)]
#[test]
fn binary_fmt_rejects_symlinked_assets_dir_outside_crate() {
    let temp = fixtures::tempdir();
    let outside = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(outside.path().join("en")).expect("create outside fallback");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"symlink-assets\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        outside.path().join("en/symlink-assets.ftl"),
        "z = Z\na = A\n",
    )
    .expect("write outside ftl");
    std::os::unix::fs::symlink(outside.path(), temp.path().join("i18n"))
        .expect("create assets symlink");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"][0].as_str().is_some_and(
        |message| message.contains("Invalid assets_dir") && message.contains("crate root")
    ));

    let outside_content = std::fs::read_to_string(outside.path().join("en/symlink-assets.ftl"))
        .expect("read outside ftl");
    assert_eq!(outside_content, "z = Z\na = A\n");
}

#[cfg(unix)]
#[test]
fn binary_fmt_rejects_symlinked_assets_dir_inside_crate_without_formatting_target() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("real-i18n/en")).expect("create real fallback");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"symlink-assets-inside\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    let ftl_path = temp.path().join("real-i18n/en/symlink-assets-inside.ftl");
    std::fs::write(&ftl_path, "z = Z\na = A\n").expect("write unsorted ftl");
    std::os::unix::fs::symlink(temp.path().join("real-i18n"), temp.path().join("i18n"))
        .expect("create in-crate assets symlink");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"][0].as_str().is_some_and(
        |message| message.contains("Invalid assets_dir") && message.contains("not symlinks")
    ));

    let content = std::fs::read_to_string(&ftl_path).expect("read ftl");
    assert_eq!(content, "z = Z\na = A\n");
}

#[cfg(unix)]
#[test]
fn binary_fmt_rejects_symlinked_fallback_locale_without_formatting_external_file() {
    let temp = fixtures::tempdir();
    let outside = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n")).expect("create assets dir");
    std::fs::create_dir_all(outside.path().join("en")).expect("create outside fallback");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    let outside_ftl = outside.path().join("en/test-app.ftl");
    std::fs::write(&outside_ftl, "z = Z\na = A\n").expect("write outside ftl");
    std::os::unix::fs::symlink(outside.path().join("en"), temp.path().join("i18n/en"))
        .expect("create fallback locale symlink");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["errors"][0].as_str().is_some_and(
            |message| message.contains("FTL directories") && message.contains("symlinks")
        )
    );

    let outside_content = std::fs::read_to_string(&outside_ftl).expect("read outside ftl");
    assert_eq!(outside_content, "z = Z\na = A\n");
}

#[test]
fn binary_fmt_json_rejects_fallback_locale_path_as_file() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n")).expect("create assets dir");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/en"), "not a directory\n").expect("write fallback file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["files"][0]["error"]
            .as_str()
            .is_some_and(|error| error.contains("not a directory"))
    );
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|error| error.contains("not a directory") && error.contains("i18n/en"))
    );
    assert!(
        !json["files"][0]["path"]
            .as_str()
            .is_some_and(|path| path.contains(temp.path().to_string_lossy().as_ref())),
        "fmt JSON file paths should be workspace-relative"
    );
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|error| error.contains(temp.path().to_string_lossy().as_ref())),
        "fmt JSON errors should not include absolute workspace paths"
    );
}

#[test]
fn binary_fmt_json_reports_file_parse_errors_in_top_level_errors() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        temp.path().join("i18n/en/test-app.ftl"),
        "hello = { $name\n",
    )
    .expect("write invalid ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["files"][0]["error"]
            .as_str()
            .is_some_and(|error| error.contains("parse errors"))
    );
    assert!(json["errors"][0].as_str().is_some_and(
        |error| error.contains("parse errors") && error.contains("i18n/en/test-app.ftl")
    ));
    assert_eq!(json["files"][0]["path"], "i18n/en/test-app.ftl");
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|error| error.contains(temp.path().to_string_lossy().as_ref())),
        "fmt JSON parse errors should not include absolute workspace paths"
    );
}

#[test]
fn binary_fmt_json_rejects_ftl_path_as_directory() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en/test-app.ftl"))
        .expect("create ftl directory");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("Expected FTL path to be a file")
                && message.contains("test-app.ftl"))
    );
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains(temp.path().to_string_lossy().as_ref())),
        "fmt JSON setup errors should not include absolute workspace paths"
    );
}

#[test]
fn binary_fmt_json_rejects_assets_dir_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["files"][0]["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("i18n"))
    );
    assert!(
        json["files"][0]["error"]
            .as_str()
            .is_some_and(|error| error.contains("assets_dir for test-app"))
    );
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|error| error.contains("assets_dir for test-app"))
    );
}

#[test]
fn binary_fmt_all_json_rejects_locale_named_asset_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["files"][0]["error"]
            .as_str()
            .is_some_and(|error| error.contains("locale directory 'fr'"))
    );
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|error| error.contains("locale directory 'fr'"))
    );
}

#[test]
fn binary_fmt_json_keeps_successful_files_with_mixed_workspace_errors() {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    for name in ["a", "b"] {
        std::fs::create_dir_all(temp.path().join(format!("{name}/src"))).expect("create src");
        std::fs::create_dir_all(temp.path().join(format!("{name}/i18n/en"))).expect("create en");
        std::fs::write(
            temp.path().join(format!("{name}/Cargo.toml")),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write manifest");
        std::fs::write(
            temp.path().join(format!("{name}/src/lib.rs")),
            "pub fn marker() {}\n",
        )
        .expect("write lib");
        std::fs::write(
            temp.path().join(format!("{name}/i18n.toml")),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write config");
    }
    std::fs::write(temp.path().join("a/i18n/en/a.ftl"), "z = Z\na = A\n")
        .expect("write unsorted ftl");
    std::fs::write(temp.path().join("b/i18n/en/b.ftl"), "hello = Hello\n")
        .expect("write fallback ftl");
    std::fs::write(temp.path().join("b/i18n/fr"), "not a directory\n").expect("write locale file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["formatted_count"], 1);
    assert_eq!(json["error_count"], 1);
    assert!(
        json["files"]
            .as_array()
            .is_some_and(|files| files.iter().any(|file| file["changed"] == true
                && file["path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("a.ftl"))))
    );
    assert!(
        json["files"]
            .as_array()
            .is_some_and(|files| files.iter().any(|file| file["error"]
                .as_str()
                .is_some_and(|error| error.contains("locale directory 'fr'"))))
    );
    assert!(json["errors"][0].as_str().is_some_and(
        |error| error.contains("locale directory 'fr'") && error.contains("b/i18n/fr")
    ));
}

#[test]
fn binary_sync_all_rejects_locale_named_asset_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("locale path")
                && message.contains("fr for test-app")
                && message.contains("not directories"))
    );
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains(temp.path().to_string_lossy().as_ref())),
        "sync JSON all-locale setup errors should not include absolute workspace paths"
    );
}

#[test]
fn binary_sync_all_rejects_assets_dir_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("assets_dir for test-app")
                && message.contains("not a directory"))
    );
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains(temp.path().to_string_lossy().as_ref())),
        "sync JSON assets_dir errors should not include absolute workspace paths"
    );

    assert!(temp.path().join("i18n").is_file());
}

#[test]
fn binary_sync_explicit_json_rejects_assets_dir_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr-FR",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("assets_dir for test-app")
                && message.contains("not a directory"))
    );
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains(temp.path().to_string_lossy().as_ref())),
        "sync JSON explicit-target assets_dir errors should not include absolute workspace paths"
    );

    assert!(temp.path().join("i18n").is_file());
}

#[test]
fn binary_sync_dry_run_json_reports_preview_mode_without_writing() {
    let temp = fixtures::create_workspace();

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr-FR",
            "--create",
            "--dry-run",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["keys_added"], 1);
    assert_eq!(json["locales_affected"], 1);
    assert_eq!(json["results"][0]["locale"], "fr-FR");
    assert_eq!(json["results"][0]["locale_created"], true);
    assert!(!temp.path().join("i18n/fr-FR").exists());
}

#[test]
fn binary_sync_and_add_locale_support_binary_only_file_workflows_without_runner() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create target locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"binary-only-files\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"binary-only-files\"\npath = \"src/main.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/main.rs"), "fn main() {}\n").expect("write main");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        temp.path().join("i18n/en/binary-only-files.ftl"),
        "hello = Hello\nbye = Bye\n",
    )
    .expect("write fallback ftl");
    std::fs::write(
        temp.path().join("i18n/fr/binary-only-files.ftl"),
        "hello = Bonjour\n",
    )
    .expect("write target ftl");
    let workspace = temp.path().to_str().expect("workspace path");

    let sync_output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            workspace,
            "--locale",
            "fr",
            "--dry-run",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&sync_output).expect("sync stdout is JSON only");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["keys_added"], 1);
    assert_eq!(json["locales_affected"], 1);
    assert_eq!(json["results"][0]["locale"], "fr");
    assert_eq!(json["results"][0]["added_keys"][0], "bye");
    assert_eq!(
        std::fs::read_to_string(temp.path().join("i18n/fr/binary-only-files.ftl"))
            .expect("read target ftl"),
        "hello = Bonjour\n"
    );
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "sync should not prepare runner metadata for binary-only file workflows"
    );
    assert!(
        !temp.path().join("target").exists(),
        "sync should not run Cargo for binary-only file workflows"
    );

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            workspace,
            "--dry-run",
            "es",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Fluent FTL Add Locale"))
        .stdout(predicate::str::contains(
            "Would create locale directory for es",
        ))
        .stdout(predicate::str::contains("Would add 2 key(s)"))
        .stdout(predicate::str::contains("+ hello = Hello"))
        .stdout(predicate::str::contains("+ bye = Bye"))
        .stderr(predicate::str::is_empty());

    assert!(!temp.path().join("i18n/es").exists());
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "add-locale should not prepare runner metadata for binary-only file workflows"
    );
    assert!(
        !temp.path().join("target").exists(),
        "add-locale should not run Cargo for binary-only file workflows"
    );
}

#[test]
fn binary_tree_json_rejects_fallback_locale_path_as_file() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n")).expect("create assets dir");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/en"), "not a directory\n").expect("write fallback file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["errors"][0]["crate_name"], "test-app");
    assert!(
        json["errors"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("locale directory 'en'")
                && message.contains("not a directory"))
    );
}

#[test]
fn binary_tree_all_json_rejects_missing_fallback_locale_directory() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create non-fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        temp.path().join("i18n/fr/test-app.ftl"),
        "hello = Bonjour\n",
    )
    .expect("write non-fallback ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["crates"], Value::Array(Vec::new()));
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["errors"][0]["crate_name"], "test-app");
    assert!(
        json["errors"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("locale directory 'en'")
                && message.contains("missing or not a directory"))
    );
}

#[test]
fn binary_tree_json_rejects_ftl_path_as_directory() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en/test-app.ftl"))
        .expect("create ftl directory");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["errors"][0]["crate_name"], "test-app");
    assert!(
        json["errors"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("Expected FTL path to be a file")
                && message.contains("test-app.ftl"))
    );
}

#[test]
fn binary_tree_text_rust_links_rejects_ftl_path_directory_before_runner() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en/test-app.ftl"))
        .expect("create ftl directory");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "this is not rust\n").expect("write bad lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .env("FORCE_HYPERLINK", "1")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--link-mode",
            "rust",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Expected FTL path to be a file"))
        .stderr(predicate::str::contains("test-app.ftl"))
        .stderr(predicate::str::contains("could not compile").not());

    assert!(
        !temp.path().join(".es-fluent").exists(),
        "tree should reject invalid FTL paths before runner metadata"
    );
    assert!(
        !temp.path().join("target").exists(),
        "tree should reject invalid FTL paths before Cargo runs"
    );
}

#[test]
fn binary_tree_json_rejects_assets_dir_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["errors"][0]["crate_name"], "test-app");
    assert!(
        json["errors"][0]["message"].as_str().is_some_and(
            |message| message.contains("assets_dir") && message.contains("not a directory")
        )
    );
}

#[cfg(unix)]
#[test]
fn binary_tree_json_rejects_symlinked_assets_dir() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::create_dir_all(temp.path().join("real-i18n/en")).expect("create real locale");
    std::fs::write(
        temp.path().join("real-i18n/en/test-app.ftl"),
        "hello = Hello\n",
    )
    .expect("write real ftl");
    std::os::unix::fs::symlink(temp.path().join("real-i18n"), temp.path().join("i18n"))
        .expect("create assets symlink");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["errors"][0]["crate_name"], "workspace");
    assert!(
        json["errors"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("assets_dir") && message.contains("symlink"))
    );
}

#[cfg(unix)]
#[test]
fn binary_tree_json_rejects_symlinked_fallback_locale_dir() {
    let temp = fixtures::create_workspace();
    let outside = fixtures::tempdir();
    std::fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");
    std::fs::create_dir_all(outside.path().join("en")).expect("create outside locale");
    std::fs::write(outside.path().join("en/test-app.ftl"), "hello = Hello\n")
        .expect("write outside ftl");
    std::os::unix::fs::symlink(outside.path().join("en"), temp.path().join("i18n/en"))
        .expect("create fallback locale symlink");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["errors"][0]["crate_name"], "test-app");
    assert!(
        json["errors"][0]["message"]
            .as_str()
            .is_some_and(
                |message| message.contains("locale directory 'en'") && message.contains("symlink")
            )
    );
}

#[test]
fn binary_tree_all_json_rejects_locale_named_asset_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["errors"][0]["crate_name"], "test-app");
    assert!(
        json["errors"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("locale directory 'fr'")
                && message.contains("not a directory"))
    );
}

#[test]
fn binary_tree_json_honors_attribute_and_variable_filters() {
    let temp = fixtures::create_workspace();
    std::fs::write(
        temp.path().join("i18n/en/test-app.ftl"),
        "hello = Hello { $name }\n    .title = Title { $name }\n",
    )
    .expect("write ftl with attributes and variables");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
            "--no-attributes",
            "--no-variables",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    let entry = &json["crates"][0]["locales"][0]["files"][0]["entries"][0];
    assert_eq!(entry["id"], "hello");
    assert!(
        entry["attributes"]
            .as_array()
            .expect("attributes")
            .is_empty()
    );
    assert!(entry["variables"].as_array().expect("variables").is_empty());
}

#[test]
fn binary_tree_json_no_attributes_hides_attribute_only_variables() {
    let temp = fixtures::create_workspace();
    std::fs::write(
        temp.path().join("i18n/en/test-app.ftl"),
        "hello = Hello { $name }\n    .title = Title { $title }\n",
    )
    .expect("write ftl with distinct value and attribute variables");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
            "--no-attributes",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    let entry = &json["crates"][0]["locales"][0]["files"][0]["entries"][0];
    assert_eq!(entry["id"], "hello");
    assert!(
        entry["attributes"]
            .as_array()
            .expect("attributes")
            .is_empty()
    );
    assert_eq!(entry["variables"], serde_json::json!(["name"]));
}

#[test]
fn binary_tree_json_reports_ftl_parse_errors_without_failing() {
    let temp = fixtures::create_workspace();
    std::fs::write(
        temp.path().join("i18n/en/test-app.ftl"),
        "hello = { $name\n",
    )
    .expect("write invalid ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 0);
    let file = &json["crates"][0]["locales"][0]["files"][0];
    assert_eq!(file["parse_error"], true);
    assert!(file["entries"].as_array().expect("entries").is_empty());
}

#[test]
fn binary_tree_json_is_file_only_even_with_rust_link_mode() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create i18n");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "this is not rust\n").expect("write bad lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n").expect("write ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--link-mode",
            "rust",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 0);
    assert_eq!(
        json["crates"][0]["locales"][0]["files"][0]["entries"][0]["id"],
        "hello"
    );
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "tree JSON should not prepare runner metadata"
    );
    assert!(
        !temp.path().join("target").exists(),
        "tree JSON should not run Cargo for Rust links"
    );
}

#[test]
fn binary_tree_json_rejects_invalid_link_mode_before_workspace_discovery() {
    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            "/definitely/missing/path",
            "--link-mode",
            "bad",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["errors"][0]["crate_name"], "workspace");
    let message = json["errors"][0]["message"]
        .as_str()
        .expect("tree error message");
    assert!(message.contains("invalid link mode 'bad'"));
    assert!(
        !message.contains("/definitely/missing/path"),
        "tree should reject invalid link modes before workspace discovery"
    );
}

#[test]
fn binary_tree_text_shows_empty_locale_directories() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"empty-tree\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("empty-tree"))
        .stdout(predicate::str::contains("en"));
}

#[test]
fn binary_tree_text_rust_mode_inspects_binary_only_crate_without_runner() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"binary-only-tree\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"binary-only-tree\"\npath = \"src/main.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/main.rs"), "fn main() {}\n").expect("write main");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        temp.path().join("i18n/en/binary-only-tree.ftl"),
        "hello = Hello\n",
    )
    .expect("write ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .env("FORCE_HYPERLINK", "1")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--link-mode",
            "rust",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("binary-only-tree"))
        .stdout(predicate::str::contains("hello"));

    assert!(
        !temp.path().join(".es-fluent").exists(),
        "tree should not prepare runner metadata when no selected crate has a library target"
    );
    assert!(
        !temp.path().join("target").exists(),
        "tree should not run Cargo when no selected crate has a library target"
    );
}

#[test]
fn binary_sync_rejects_create_with_all() {
    let temp = fixtures::create_workspace();

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all",
            "--create",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--all"))
        .stderr(predicate::str::contains("--create"))
        .stderr(predicate::str::contains("cannot be used"));
}

#[test]
fn binary_sync_rejects_create_without_locale_before_workspace_discovery() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "sync", "--create"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("--create"))
        .stderr(predicate::str::contains("--locale"))
        .stderr(predicate::str::contains("requires"));
}

#[test]
fn binary_sync_json_rejects_missing_target_selection_before_workspace_discovery() {
    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            "/definitely/missing/path",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"][0].as_str().is_some_and(|message| {
        message.contains("no target locales specified") && !message.contains("canonicalize")
    }));
}

#[test]
fn binary_sync_json_rejects_target_selection_conflicts_before_workspace_discovery() {
    let missing_path = "/definitely/missing/sync-target-selection";
    let cases = [
        (
            &[
                "sync",
                "--path",
                missing_path,
                "--all",
                "--create",
                "--output",
                "json",
            ][..],
            "--create requires explicit --locale targets",
        ),
        (
            &[
                "sync",
                "--path",
                missing_path,
                "--create",
                "--output",
                "json",
            ][..],
            "--create requires explicit --locale targets",
        ),
        (
            &[
                "sync",
                "--path",
                missing_path,
                "--all",
                "--locale",
                "fr-FR",
                "--output",
                "json",
            ][..],
            "--all cannot be combined with --locale",
        ),
    ];

    for (args, expected) in cases {
        let output = Command::cargo_bin("cargo-es-fluent")
            .expect("binary exists")
            .arg("es-fluent")
            .args(args)
            .assert()
            .failure()
            .stderr(predicate::str::is_empty())
            .get_output()
            .stdout
            .clone();

        let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
        assert_eq!(json["error_count"], 1);
        let message = json["errors"][0].as_str().expect("sync error message");
        assert!(
            message.contains(expected),
            "expected {expected:?} in {message:?}"
        );
        assert!(
            !message.contains(missing_path),
            "sync should reject target selection before workspace discovery: {message}"
        );
    }
}

#[test]
fn binary_sync_text_rejects_missing_target_selection_without_stdout() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "sync"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("no target locales specified"));
}

#[test]
fn binary_sync_create_rejects_target_locale_path_as_file() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback ftl");
    std::fs::write(temp.path().join("i18n/fr-FR"), "not a directory\n")
        .expect("write target locale file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr-FR",
            "--create",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"][0].as_str().is_some_and(|message| {
        message.contains("target locale directory 'fr-FR'")
            && message.contains("test-app")
            && message.contains("not a directory")
    }));
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains(temp.path().to_string_lossy().as_ref())),
        "sync JSON target locale errors should not include absolute workspace paths"
    );

    assert!(temp.path().join("i18n/fr-FR").is_file());
}

#[test]
fn binary_sync_rejects_target_locale_path_as_file_without_create() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback ftl");
    std::fs::write(temp.path().join("i18n/fr"), "not a directory\n")
        .expect("write target locale file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"][0].as_str().is_some_and(
        |message| message.contains("target locale path")
            && message.contains("fr for test-app")
            && message.contains("not directories")
            && !message.contains("--create")
    ));
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains(temp.path().to_string_lossy().as_ref())),
        "sync JSON target locale path errors should not include absolute workspace paths"
    );

    assert!(temp.path().join("i18n/fr").is_file());
}

#[cfg(unix)]
#[test]
fn binary_sync_rejects_symlinked_target_locale_without_fallback_files() {
    let temp = fixtures::tempdir();
    let outside = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::create_dir_all(outside.path().join("fr")).expect("create outside locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::os::unix::fs::symlink(outside.path().join("fr"), temp.path().join("i18n/fr"))
        .expect("create target locale symlink");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"][0].as_str().is_some_and(
        |message| message.contains("target locale path")
            && message.contains("fr for test-app")
            && message.contains("not directories")
            && !message.contains("--create")
    ));
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains(temp.path().to_string_lossy().as_ref())),
        "sync JSON target locale symlink errors should not include absolute workspace paths"
    );
    assert!(temp.path().join("i18n/fr").is_symlink());
    assert!(
        std::fs::read_dir(outside.path().join("fr"))
            .expect("read outside locale")
            .next()
            .is_none(),
        "sync must not write through the target locale symlink"
    );
}

#[test]
fn binary_add_locale_ignores_unrelated_noncanonical_locale_dir() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::create_dir_all(temp.path().join("i18n/en-us"))
        .expect("create unrelated noncanonical locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback ftl");
    std::fs::write(temp.path().join("i18n/en-us/test-app.ftl"), "hello = Hi\n")
        .expect("write unrelated ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--dry-run",
            "fr-FR",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Would create locale directory for fr-FR",
        ))
        .stderr(predicate::str::is_empty());

    assert!(!temp.path().join("i18n/fr-FR").exists());
}

#[test]
fn binary_add_locale_rejects_root_assets_locales_hidden_by_project_dir_ignores() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \".\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "bin",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Fluent FTL Add Locale"))
        .stderr(predicate::str::contains("cannot create requested locale"))
        .stderr(predicate::str::contains("bin"))
        .stderr(predicate::str::contains("all-locale scans"));

    assert!(!temp.path().join("bin").exists());
}

#[test]
fn binary_clean_orphaned_rejects_missing_fallback_locale_directory() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        temp.path().join("i18n/fr/test-app.ftl"),
        "hello = Bonjour\n",
    )
    .expect("write non-fallback ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--orphaned",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("fallback locale directory"))
        .stderr(predicate::str::contains("refusing"))
        .stderr(predicate::str::contains("scan"))
        .stderr(predicate::str::contains("orphaned"))
        .stderr(predicate::str::contains("files"));

    assert!(temp.path().join("i18n/fr/test-app.ftl").exists());
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "clean --orphaned should reject missing fallback before preparing runner metadata"
    );
    assert!(
        !temp.path().join("target").exists(),
        "clean --orphaned should reject missing fallback before running Cargo"
    );
}

#[test]
fn binary_clean_orphaned_rejects_fallback_locale_path_as_file() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/en"), "not a directory\n").expect("write fallback file");
    std::fs::write(
        temp.path().join("i18n/fr/test-app.ftl"),
        "hello = Bonjour\n",
    )
    .expect("write non-fallback ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--orphaned",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("fallback locale directory"))
        .stderr(predicate::str::contains("not a directory"));

    assert!(temp.path().join("i18n/fr/test-app.ftl").exists());
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "clean --orphaned should reject fallback files before preparing runner metadata"
    );
    assert!(
        !temp.path().join("target").exists(),
        "clean --orphaned should reject fallback files before running Cargo"
    );
}

#[cfg(unix)]
#[test]
fn binary_clean_orphaned_rejects_symlinked_fallback_locale_directory() {
    let temp = fixtures::tempdir();
    let outside = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
    std::fs::create_dir_all(outside.path().join("en")).expect("create outside fallback");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(outside.path().join("en/test-app.ftl"), "hello = Hello\n")
        .expect("write outside fallback ftl");
    std::fs::write(temp.path().join("i18n/fr/orphan.ftl"), "orphan = Orphan\n")
        .expect("write orphan");
    std::os::unix::fs::symlink(outside.path().join("en"), temp.path().join("i18n/en"))
        .expect("create fallback locale symlink");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--orphaned",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("fallback locale directory"))
        .stderr(predicate::str::contains("symlink"))
        .stderr(predicate::str::contains("refusing"))
        .stderr(predicate::str::contains("orphaned"))
        .stderr(predicate::str::contains("files"));

    assert!(temp.path().join("i18n/fr/orphan.ftl").exists());
    assert!(temp.path().join("i18n/en").is_symlink());
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "clean --orphaned should reject symlinked fallback before preparing runner metadata"
    );
    assert!(
        !temp.path().join("target").exists(),
        "clean --orphaned should reject symlinked fallback before running Cargo"
    );
}

#[test]
fn binary_clean_orphaned_rejects_ftl_path_directory_before_runner() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en/test-app.ftl"))
        .expect("create ftl directory");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/fr/orphan.ftl"), "orphan = Orphan\n")
        .expect("write orphan");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--orphaned",
            "--dry-run",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Expected FTL path"))
        .stderr(predicate::str::contains("non-file path"))
        .stderr(predicate::str::contains("refusing"))
        .stderr(predicate::str::contains("orphaned"))
        .stderr(predicate::str::contains("files"));

    assert!(temp.path().join("i18n/fr/orphan.ftl").exists());
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "clean --orphaned should reject FTL layout errors before preparing runner metadata"
    );
    assert!(
        !temp.path().join("target").exists(),
        "clean --orphaned should reject FTL layout errors before running Cargo"
    );
}

#[cfg(unix)]
#[test]
fn binary_clean_orphaned_rejects_symlinked_namespace_without_removing_external_file() {
    let temp = fixtures::tempdir();
    let outside = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create en locale");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
    std::fs::create_dir_all(outside.path().join("namespace")).expect("create outside namespace");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback");
    std::fs::write(
        outside.path().join("namespace/orphan.ftl"),
        "orphan = Outside\n",
    )
    .expect("write outside orphan");
    std::os::unix::fs::symlink(
        outside.path().join("namespace"),
        temp.path().join("i18n/fr/test-app"),
    )
    .expect("create namespace symlink");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--orphaned",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("FTL directories"))
        .stderr(predicate::str::contains("symlinks"))
        .stderr(predicate::str::contains("refusing"))
        .stderr(predicate::str::contains("scan"))
        .stderr(predicate::str::contains("orphaned"))
        .stderr(predicate::str::contains("files"));

    assert!(
        outside.path().join("namespace/orphan.ftl").exists(),
        "clean --orphaned must not remove files outside the locale tree through symlinks"
    );
    assert!(temp.path().join("i18n/fr/test-app").is_symlink());
}

#[test]
fn binary_clean_orphaned_rejects_locale_named_asset_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--orphaned",
            "--dry-run",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("locale path"))
        .stderr(predicate::str::contains("fr for test-app"))
        .stderr(predicate::str::contains("refusing"))
        .stderr(predicate::str::contains("scan"))
        .stderr(predicate::str::contains("orphaned"))
        .stderr(predicate::str::contains("files"));

    assert!(temp.path().join("i18n/fr").is_file());
}

#[test]
fn binary_clean_orphaned_binary_only_does_not_prepare_runner() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create en locale");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"binary-only-clean\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"binary-only-clean\"\npath = \"src/main.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/main.rs"), "fn main() {}\n").expect("write main");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        temp.path().join("i18n/en/binary-only-clean.ftl"),
        "hello = Hello\n",
    )
    .expect("write fallback");
    std::fs::write(temp.path().join("i18n/fr/orphan.ftl"), "orphan = Orphan\n")
        .expect("write orphan");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--orphaned",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Notice binary-only-clean (missing library target)",
        ))
        .stdout(predicate::str::contains("Skipping binary-only-clean").not())
        .stdout(predicate::str::contains("Would remove orphaned file"));

    assert!(!temp.path().join(".es-fluent").exists());
    assert!(!temp.path().join("target").exists());
    assert!(temp.path().join("i18n/fr/orphan.ftl").exists());
}

#[test]
fn binary_clean_orphaned_all_binary_only_fails_before_file_cleanup() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create en locale");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"binary-only-clean-all\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"binary-only-clean-all\"\npath = \"src/main.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/main.rs"), "fn main() {}\n").expect("write main");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        temp.path().join("i18n/en/binary-only-clean-all.ftl"),
        "hello = Hello\n",
    )
    .expect("write fallback");
    std::fs::write(temp.path().join("i18n/fr/orphan.ftl"), "orphan = Orphan\n")
        .expect("write orphan");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all",
            "--orphaned",
            "--dry-run",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("library target"));

    assert!(
        temp.path().join("i18n/fr/orphan.ftl").exists(),
        "clean --all --orphaned should fail before file-only orphan cleanup when clean cannot run"
    );
    assert!(!temp.path().join(".es-fluent").exists());
    assert!(!temp.path().join("target").exists());
}

#[test]
fn binary_clean_orphaned_scans_non_fallback_locale_without_all() {
    let temp = fixtures::create_workspace();
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
    let orphan = temp.path().join("i18n/fr/orphan.ftl");
    std::fs::write(&orphan, "orphan = Orphan\n").expect("write orphan");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--orphaned",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Would remove orphaned file"))
        .stdout(predicate::str::contains("i18n/fr/orphan.ftl"));

    assert!(
        orphan.exists(),
        "dry-run orphan scan should not remove files"
    );
}

#[test]
fn binary_clean_all_accepts_relative_assets_dir() {
    let temp = fixtures::create_workspace();
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
    std::fs::write(
        temp.path().join("i18n/fr/test-app.ftl"),
        "hello = Bonjour\n",
    )
    .expect("write fr ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all",
            "--dry-run",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Invalid assets_dir").not());
}

#[test]
fn binary_clean_rejects_assets_dir_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("generation path"))
        .stderr(predicate::str::contains("assets_dir"));

    assert!(temp.path().join("i18n").is_file());
}

#[test]
fn binary_clean_uses_clean_header() {
    let temp = fixtures::create_workspace();

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--package",
            "missing-package",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Fluent FTL Cleaner"))
        .stdout(predicate::str::contains("Fluent FTL Generator").not())
        .stderr(predicate::str::contains("missing-package"));
}

#[test]
fn binary_clean_package_filter_does_not_link_unselected_crates() {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    for name in ["a", "b"] {
        std::fs::create_dir_all(temp.path().join(format!("{name}/src"))).expect("create src");
        std::fs::create_dir_all(temp.path().join(format!("{name}/i18n/en"))).expect("create i18n");
        std::fs::write(
            temp.path().join(format!("{name}/Cargo.toml")),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write manifest");
        std::fs::write(
            temp.path().join(format!("{name}/i18n.toml")),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n config");
        std::fs::write(
            temp.path().join(format!("{name}/i18n/en/{name}.ftl")),
            "hello = Hello\n",
        )
        .expect("write ftl");
    }
    std::fs::write(temp.path().join("a/src/lib.rs"), "pub fn marker() {}\n").expect("write a lib");
    std::fs::write(temp.path().join("b/src/lib.rs"), "this is not rust\n").expect("write b lib");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--package",
            "a",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Discovered 1 crate(s)"))
        .stderr(predicate::str::contains("could not compile `b`").not());
}

#[test]
fn binary_watch_uses_watch_header() {
    let temp = fixtures::create_workspace();

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "watch",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--package",
            "missing-package",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Fluent FTL Watch"))
        .stdout(predicate::str::contains("Fluent FTL Generator").not())
        .stderr(predicate::str::contains("missing-package"));
}

#[test]
fn binary_watch_rejects_fallback_locale_path_as_file_before_runner() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");
    std::fs::write(temp.path().join("i18n/en"), "not a directory\n").expect("write fallback file");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "watch",
            "--path",
            temp.path().to_str().expect("workspace path"),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Fluent FTL Watch"))
        .stderr(predicate::str::contains("generation path"))
        .stderr(predicate::str::contains("fallback locale path 'en'"));

    assert!(
        !temp.path().join(".es-fluent").exists(),
        "watch should reject invalid generation paths before runner metadata"
    );
}

#[test]
fn binary_clean_orphaned_rejects_assets_dir_path_as_file_before_runner() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--orphaned",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Unchanged").not())
        .stderr(predicate::str::contains("generation path"))
        .stderr(predicate::str::contains("assets_dir"));

    assert!(temp.path().join("i18n").is_file());
}

#[test]
fn binary_sync_requires_locale_in_every_selected_crate() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("a/src")).expect("create a src");
    std::fs::create_dir_all(temp.path().join("a/i18n/en")).expect("create a en");
    std::fs::create_dir_all(temp.path().join("a/i18n/fr")).expect("create a fr");
    std::fs::create_dir_all(temp.path().join("b/src")).expect("create b src");
    std::fs::create_dir_all(temp.path().join("b/i18n/en")).expect("create b en");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");
    std::fs::write(
        temp.path().join("a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write a manifest");
    std::fs::write(
        temp.path().join("b/Cargo.toml"),
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write b manifest");
    std::fs::write(temp.path().join("a/src/lib.rs"), "pub fn a() {}\n").expect("write a lib");
    std::fs::write(temp.path().join("b/src/lib.rs"), "pub fn b() {}\n").expect("write b lib");
    std::fs::write(
        temp.path().join("a/i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write a config");
    std::fs::write(
        temp.path().join("b/i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write b config");
    std::fs::write(
        temp.path().join("a/i18n/en/a.ftl"),
        "hello = Hello\nworld = World\n",
    )
    .expect("write a fallback");
    std::fs::write(temp.path().join("a/i18n/fr/a.ftl"), "hello = Bonjour\n").expect("write a fr");
    std::fs::write(
        temp.path().join("b/i18n/en/b.ftl"),
        "hello = Hello\nworld = World\n",
    )
    .expect("write b fallback");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr",
            "--dry-run",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("fr for b"))
        .stderr(predicate::str::contains("--create"));

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr",
            "--dry-run",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("fr for b") && message.contains("--create"))
    );
}

#[test]
fn binary_sync_json_preflights_workspace_before_reporting_successful_results() {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    for name in ["a", "b"] {
        std::fs::create_dir_all(temp.path().join(format!("{name}/src"))).expect("create src");
        std::fs::create_dir_all(temp.path().join(format!("{name}/i18n/en"))).expect("create en");
        std::fs::create_dir_all(temp.path().join(format!("{name}/i18n/fr"))).expect("create fr");
        std::fs::write(
            temp.path().join(format!("{name}/Cargo.toml")),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write manifest");
        std::fs::write(
            temp.path().join(format!("{name}/src/lib.rs")),
            "pub fn marker() {}\n",
        )
        .expect("write lib");
        std::fs::write(
            temp.path().join(format!("{name}/i18n.toml")),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write config");
        std::fs::write(
            temp.path().join(format!("{name}/i18n/en/{name}.ftl")),
            "hello = Hello\nworld = World\n",
        )
        .expect("write fallback");
    }
    std::fs::write(temp.path().join("a/i18n/fr/a.ftl"), "hello = Bonjour\n")
        .expect("write incomplete a fr");
    std::fs::write(temp.path().join("b/i18n/fr/b.ftl"), "broken = { $name\n")
        .expect("write invalid b fr");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr",
            "--dry-run",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["keys_added"], 0);
    assert_eq!(json["locales_affected"], 0);
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"][0].as_str().is_some_and(
        |message| message.contains("Refusing to sync") && message.contains("parse errors")
    ));
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("b/i18n/fr/b.ftl"))
    );
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains(temp.path().to_string_lossy().as_ref())),
        "sync JSON parse errors should not include absolute workspace paths"
    );
    assert_eq!(json["results"], Value::Array(Vec::new()));
    assert_eq!(
        std::fs::read_to_string(temp.path().join("a/i18n/fr/a.ftl")).expect("read a fr"),
        "hello = Bonjour\n"
    );
}

#[test]
fn binary_sync_rejects_target_namespace_parent_file_before_partial_write() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en/test-app")).expect("create en namespace");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        temp.path().join("i18n/en/test-app.ftl"),
        "hello = Hello\nworld = World\n",
    )
    .expect("write fallback main");
    std::fs::write(
        temp.path().join("i18n/en/test-app/ui.ftl"),
        "button = Button\n",
    )
    .expect("write fallback namespace");

    let target_main = temp.path().join("i18n/fr/test-app.ftl");
    std::fs::write(&target_main, "hello = Bonjour\n").expect("write incomplete fr main");
    std::fs::write(temp.path().join("i18n/fr/test-app"), "not a directory\n")
        .expect("write target namespace blocker");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Refusing to sync"))
        .stderr(predicate::str::contains("parent"))
        .stderr(predicate::str::contains("path"))
        .stderr(predicate::str::contains("not a directory"));

    assert_eq!(
        std::fs::read_to_string(target_main).expect("read fr main"),
        "hello = Bonjour\n",
        "sync should reject blocked namespace paths before writing earlier target files"
    );
}

#[test]
fn binary_sync_rejects_target_ftl_directory_before_partial_write() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en/test-app")).expect("create en namespace");
    std::fs::create_dir_all(temp.path().join("i18n/fr/test-app")).expect("create fr namespace");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        temp.path().join("i18n/en/test-app.ftl"),
        "hello = Hello\nworld = World\n",
    )
    .expect("write fallback main");
    std::fs::write(
        temp.path().join("i18n/en/test-app/ui.ftl"),
        "button = Button\n",
    )
    .expect("write fallback namespace");

    let target_main = temp.path().join("i18n/fr/test-app.ftl");
    std::fs::write(&target_main, "hello = Bonjour\n").expect("write incomplete fr main");
    std::fs::create_dir_all(temp.path().join("i18n/fr/test-app/ui.ftl"))
        .expect("create target ftl directory");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Refusing to sync"))
        .stderr(predicate::str::contains("target"))
        .stderr(predicate::str::contains("FTL path"))
        .stderr(predicate::str::contains("not a file"));

    assert_eq!(
        std::fs::read_to_string(target_main).expect("read fr main"),
        "hello = Bonjour\n",
        "sync should reject target FTL directories before writing earlier target files"
    );
}

#[cfg(unix)]
#[test]
fn binary_sync_rejects_target_ftl_symlink_without_writing_external_file() {
    let temp = fixtures::tempdir();
    let outside = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create en");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        temp.path().join("i18n/en/test-app.ftl"),
        "hello = Hello\nworld = World\n",
    )
    .expect("write fallback main");
    let outside_ftl = outside.path().join("test-app.ftl");
    std::fs::write(&outside_ftl, "hello = Outside\n").expect("write outside target");
    std::os::unix::fs::symlink(&outside_ftl, temp.path().join("i18n/fr/test-app.ftl"))
        .expect("create target FTL symlink");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Refusing to sync"))
        .stderr(predicate::str::contains("target FTL"))
        .stderr(predicate::str::contains("paths"))
        .stderr(predicate::str::contains("symlinks"));

    assert_eq!(
        std::fs::read_to_string(&outside_ftl).expect("read outside target"),
        "hello = Outside\n",
        "sync must not write through target FTL symlinks"
    );
    assert!(temp.path().join("i18n/fr/test-app.ftl").is_symlink());
}

#[test]
fn binary_sync_json_counts_same_locale_in_multiple_workspace_crates() {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    for name in ["a", "b"] {
        std::fs::create_dir_all(temp.path().join(format!("{name}/src"))).expect("create src");
        std::fs::create_dir_all(temp.path().join(format!("{name}/i18n/en"))).expect("create en");
        std::fs::create_dir_all(temp.path().join(format!("{name}/i18n/fr"))).expect("create fr");
        std::fs::write(
            temp.path().join(format!("{name}/Cargo.toml")),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write manifest");
        std::fs::write(
            temp.path().join(format!("{name}/src/lib.rs")),
            "pub fn marker() {}\n",
        )
        .expect("write lib");
        std::fs::write(
            temp.path().join(format!("{name}/i18n.toml")),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write config");
        std::fs::write(
            temp.path().join(format!("{name}/i18n/en/{name}.ftl")),
            "hello = Hello\nworld = World\n",
        )
        .expect("write fallback");
        std::fs::write(
            temp.path().join(format!("{name}/i18n/fr/{name}.ftl")),
            "hello = Bonjour\n",
        )
        .expect("write incomplete fr");
    }

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr",
            "--dry-run",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["keys_added"], 2);
    assert_eq!(json["locales_affected"], 2);
    assert_eq!(json["results"].as_array().expect("results").len(), 2);
}

#[test]
fn binary_json_commands_reject_empty_package_filter_before_workspace_discovery() {
    let missing_path = "/definitely/missing/empty-package-filter";
    let cases = [
        (
            "fmt",
            vec![
                "fmt",
                "--path",
                missing_path,
                "--package",
                " ",
                "--output",
                "json",
            ],
        ),
        (
            "check",
            vec![
                "check",
                "--path",
                missing_path,
                "--package",
                " ",
                "--output",
                "json",
            ],
        ),
        (
            "sync",
            vec![
                "sync",
                "--path",
                missing_path,
                "--package",
                " ",
                "--all",
                "--output",
                "json",
            ],
        ),
        (
            "tree",
            vec![
                "tree",
                "--path",
                missing_path,
                "--package",
                " ",
                "--output",
                "json",
            ],
        ),
        (
            "status",
            vec![
                "status",
                "--path",
                missing_path,
                "--package",
                " ",
                "--output",
                "json",
            ],
        ),
    ];

    for (command_name, args) in cases {
        let output = Command::cargo_bin("cargo-es-fluent")
            .expect("binary exists")
            .arg("es-fluent")
            .args(args)
            .assert()
            .failure()
            .stderr(predicate::str::is_empty())
            .get_output()
            .stdout
            .clone();

        let json: Value = serde_json::from_slice(&output)
            .unwrap_or_else(|error| panic!("{command_name} stdout is not JSON: {error}"));
        let json_text = json.to_string();
        assert!(
            json_text.contains("package filter must not be empty"),
            "{command_name} should report the empty package filter, got {json_text}"
        );
        assert!(
            !json_text.contains(missing_path),
            "{command_name} should reject --package before workspace discovery, got {json_text}"
        );
    }
}

#[test]
fn binary_package_filter_existing_unconfigured_package_reports_configured_crate_selection() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("plain/src")).expect("create plain src");
    std::fs::create_dir_all(temp.path().join("localized/src")).expect("create localized src");
    std::fs::create_dir_all(temp.path().join("localized/i18n/en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"plain\", \"localized\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");
    std::fs::write(
        temp.path().join("plain/Cargo.toml"),
        "[package]\nname = \"plain\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write plain manifest");
    std::fs::write(temp.path().join("plain/src/lib.rs"), "pub fn marker() {}\n")
        .expect("write plain lib");
    std::fs::write(
        temp.path().join("localized/Cargo.toml"),
        "[package]\nname = \"localized\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write localized manifest");
    std::fs::write(
        temp.path().join("localized/src/lib.rs"),
        "pub fn marker() {}\n",
    )
    .expect("write localized lib");
    std::fs::write(
        temp.path().join("localized/i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write localized config");
    std::fs::write(
        temp.path().join("localized/i18n/en/localized.ftl"),
        "hello = Hello\n",
    )
    .expect("write localized ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--package",
            "plain",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["crates_discovered"], 0);
    assert_eq!(
        json["workspace_warnings"],
        Value::Array(vec![Value::String(
            "no configured crate found matching package filter 'plain'".to_string()
        )])
    );
}

#[test]
fn binary_package_filter_trims_surrounding_whitespace() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            workspace,
            "--package",
            " test-app ",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 0);
    assert_eq!(json["crates"][0]["name"], "test-app");
}

#[test]
fn binary_text_commands_report_missing_package_filter() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let cases: &[&[&str]] = &[&["check", "--path", workspace, "--package", "missing-package"]];

    for args in cases {
        Command::cargo_bin("cargo-es-fluent")
            .expect("binary exists")
            .args(std::iter::once("es-fluent").chain(args.iter().copied()))
            .assert()
            .success()
            .stdout(predicate::str::contains("missing-package"))
            .stdout(
                predicate::str::contains("workspace warning").or(predicate::str::contains(
                    "WARNING: no configured crate found matching package filter",
                )),
            );
    }

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            workspace,
            "--package",
            "missing-package",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("missing-package"));

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
            "--path",
            workspace,
            "--package",
            "missing-package",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("missing-package"));
}

#[test]
fn binary_check_reports_when_all_selected_crates_are_ignored() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            workspace,
            "--ignore",
            "test-app",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "all selected crates were ignored by --ignore",
        ))
        .stdout(predicate::str::contains("No crates with i18n.toml found.").not());

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            workspace,
            "--ignore",
            "test-app",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["crates_discovered"], 1);
    assert_eq!(json["crates_checked"], 0);
    assert_eq!(
        json["workspace_warnings"],
        Value::Array(vec![Value::String(
            "all selected crates were ignored by --ignore".to_string()
        )])
    );
}

#[test]
fn binary_check_accepts_comma_separated_ignore_with_spaces() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("a/src")).expect("create a src");
    std::fs::create_dir_all(temp.path().join("a/i18n/en")).expect("create a en");
    std::fs::create_dir_all(temp.path().join("b/src")).expect("create b src");
    std::fs::create_dir_all(temp.path().join("b/i18n/en")).expect("create b en");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");
    for name in ["a", "b"] {
        std::fs::write(
            temp.path().join(name).join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write member manifest");
        std::fs::write(
            temp.path().join(name).join("src/lib.rs"),
            "pub fn marker() {}\n",
        )
        .expect("write lib");
        std::fs::write(
            temp.path().join(name).join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n config");
        std::fs::write(
            temp.path().join(name).join(format!("i18n/en/{name}.ftl")),
            "hello = Hello\n",
        )
        .expect("write ftl");
    }

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--ignore",
            "a, b",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["crates_discovered"], 2);
    assert_eq!(json["crates_checked"], 0);
}

#[test]
fn binary_check_json_rejects_empty_comma_separated_ignore_entries() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            workspace,
            "--ignore",
            "test-app,",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["crates_discovered"], 0);
    assert_eq!(json["crates_checked"], 0);
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "command_error");
    assert!(
        json["issues"][0]["help"]
            .as_str()
            .is_some_and(|message| message.contains("ignore values must not be empty"))
    );
}

#[test]
fn binary_check_json_rejects_empty_ignore_before_workspace_discovery() {
    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            "/definitely/missing/path",
            "--ignore",
            "test-app,",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["crates_discovered"], 0);
    assert_eq!(json["crates_checked"], 0);
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "command_error");
    assert!(json["issues"][0]["help"].as_str().is_some_and(|message| {
        message.contains("ignore values must not be empty") && !message.contains("canonicalize")
    }));
}

#[test]
fn binary_check_json_reports_unknown_ignore_as_json() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let assert = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            workspace,
            "--ignore",
            "missing-package",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty());
    let output = assert.get_output().stdout.clone();
    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["crates_discovered"], 1);
    assert_eq!(json["crates_checked"], 0);
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "command_error");
    assert!(
        json["issues"][0]["help"]
            .as_str()
            .is_some_and(|message| message.contains("missing-package"))
    );
}

#[test]
fn binary_check_text_does_not_print_header_for_invalid_ignore_arguments() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            workspace,
            "--ignore",
            "test-app,",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("ignore values must not be empty"))
        .stderr(predicate::str::contains("Fluent FTL Checker").not());

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            workspace,
            "--ignore",
            "missing-package",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "Unknown crates passed to --ignore: 'missing-package'",
        ))
        .stderr(predicate::str::contains("Fluent FTL Checker").not());
}

#[test]
fn binary_add_locale_uses_add_locale_text_labels() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            workspace,
            "--dry-run",
            "fr-FR",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Fluent FTL Add Locale"))
        .stdout(predicate::str::contains("Would add"))
        .stdout(predicate::str::contains("Fluent FTL Sync").not())
        .stdout(predicate::str::contains("Would sync").not());
}

#[test]
fn binary_add_locale_help_describes_dry_run_added_keys() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "add-locale", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "show locale directories and keys that would be added",
        ));
}

#[test]
fn binary_add_locale_reports_add_locale_wording_for_target_parse_errors() {
    let temp = fixtures::create_workspace();
    std::fs::create_dir_all(temp.path().join("i18n/fr-FR")).expect("create target locale");
    std::fs::write(
        temp.path().join("i18n/fr-FR/test-app.ftl"),
        "hello = { $name\n",
    )
    .expect("write invalid target ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "fr-FR",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Fluent FTL Add Locale"))
        .stderr(predicate::str::contains("Refusing to add locale data"))
        .stderr(predicate::str::contains("parse errors"))
        .stderr(predicate::str::contains("Refusing to sync").not());
}

#[test]
fn binary_sync_and_add_locale_do_not_print_headers_for_invalid_locale_arguments() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            workspace,
            "--create",
            "--locale",
            "zh-cn",
            "--dry-run",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "locale 'zh-cn' must use canonical BCP-47 form 'zh-CN'",
        ))
        .stderr(predicate::str::contains("Fluent FTL Sync").not());

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            workspace,
            "zh-cn",
            "--dry-run",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "locale 'zh-cn' must use canonical BCP-47 form 'zh-CN'",
        ))
        .stderr(predicate::str::contains("Fluent FTL Add Locale").not());
}

#[test]
fn binary_add_locale_reports_requested_locale_for_fallback_target() {
    let temp = fixtures::create_workspace();

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "en",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Fluent FTL Add Locale"))
        .stderr(predicate::str::contains(
            "requested locale must not be the fallback locale",
        ))
        .stderr(predicate::str::contains("target locale").not());
}

#[test]
fn binary_sync_accepts_comma_separated_locales_with_spaces() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            workspace,
            "--locale",
            "fr-FR, zh-CN",
            "--create",
            "--dry-run",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["locales_affected"], 2);
    assert!(json["results"].as_array().is_some_and(|results| {
        results
            .iter()
            .any(|result| result["locale"] == "fr-FR" && result["locale_created"] == true)
    }));
    assert!(json["results"].as_array().is_some_and(|results| {
        results
            .iter()
            .any(|result| result["locale"] == "zh-CN" && result["locale_created"] == true)
    }));
    assert!(!temp.path().join("i18n/fr-FR").exists());
    assert!(!temp.path().join("i18n/zh-CN").exists());
}

#[test]
fn binary_sync_json_rejects_empty_comma_separated_locale_entries() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            workspace,
            "--locale",
            "fr-FR,",
            "--create",
            "--dry-run",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");

    assert_eq!(json["dry_run"], true);
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"].as_array().is_some_and(|errors| {
        errors.iter().any(|error| {
            error
                .as_str()
                .is_some_and(|message| message.contains("locale values must not be empty"))
        })
    }));
    assert!(!temp.path().join("i18n/fr-FR").exists());
}

#[test]
fn binary_sync_deduplicates_explicit_locale_targets() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            workspace,
            "--locale",
            "fr-FR, fr-FR",
            "--create",
            "--dry-run",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["locales_affected"], 1);
    let results = json["results"].as_array().expect("results array");
    assert_eq!(
        results
            .iter()
            .filter(|result| result["locale"] == "fr-FR")
            .count(),
        1
    );
    assert!(!temp.path().join("i18n/fr-FR").exists());
}

#[test]
fn binary_add_locale_accepts_comma_separated_locales_with_spaces() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            workspace,
            "fr-FR, zh-CN",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Created locale directory for fr-FR",
        ))
        .stdout(predicate::str::contains(
            "Created locale directory for zh-CN",
        ));

    assert!(temp.path().join("i18n/fr-FR/test-app.ftl").is_file());
    assert!(temp.path().join("i18n/zh-CN/test-app.ftl").is_file());
}

#[test]
fn binary_add_locale_deduplicates_explicit_locale_targets() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            workspace,
            "fr-FR",
            "fr-FR",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).expect("stdout should be UTF-8");
    assert_eq!(
        stdout.matches("Created locale directory for fr-FR").count(),
        1
    );
    assert!(temp.path().join("i18n/fr-FR/test-app.ftl").is_file());
}

#[test]
fn binary_add_locale_rejects_empty_comma_separated_locale_entries() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "add-locale", "--path", workspace, "fr-FR,"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("locale values must not be empty"))
        .stderr(predicate::str::contains("remove empty entries"))
        .stderr(predicate::str::contains("comma-separated"));

    assert!(!temp.path().join("i18n/fr-FR").exists());
}

#[test]
fn binary_add_locale_rejects_missing_package_filter() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            workspace,
            "--package",
            "missing-package",
            "fr-FR",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot create requested locale"))
        .stderr(predicate::str::contains("missing-package"))
        .stderr(predicate::str::contains("target locale").not());

    assert!(!temp.path().join("i18n/fr-FR").exists());
}

#[test]
fn binary_add_locale_creates_empty_locale_directory() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n config");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "fr-FR",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Created locale directory for fr-FR",
        ))
        .stdout(predicate::str::contains("All locales are in sync").not());

    assert!(temp.path().join("i18n/fr-FR").is_dir());
}

#[test]
fn binary_add_locale_rerun_reports_add_locale_noop() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "add-locale", "--path", workspace, "fr-FR"])
        .assert()
        .success();

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "add-locale", "--path", workspace, "fr-FR"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Fluent FTL Add Locale"))
        .stdout(predicate::str::contains(
            "No locale directories or keys needed to be added.",
        ))
        .stdout(predicate::str::contains("All locales are in sync").not());
}

#[test]
fn binary_add_locale_rejects_missing_fallback_locale_directory() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create non-fallback locale");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n config");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "de-DE",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("fallback locale directory"))
        .stderr(predicate::str::contains("test-"))
        .stderr(predicate::str::contains("app:"));

    assert!(!temp.path().join("i18n/de-DE").exists());
}

#[test]
fn binary_add_locale_rejects_assets_dir_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "fr-FR",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("assets_dir for test-app"))
        .stderr(predicate::str::contains("not a directory"))
        .stderr(predicate::str::contains("fallback locale directory").not());

    assert!(temp.path().join("i18n").is_file());
    assert!(!temp.path().join("i18n/fr-FR").exists());
}

#[test]
fn binary_add_locale_rejects_fallback_locale_path_as_file() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n")).expect("create assets dir");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n config");
    std::fs::write(temp.path().join("i18n/en"), "not a directory\n").expect("write fallback file");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "fr-FR",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("fallback locale directory"))
        .stderr(predicate::str::contains("not a directory"));

    assert!(!temp.path().join("i18n/fr-FR").exists());
}

#[test]
fn binary_add_locale_rejects_requested_locale_path_as_file() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n config");
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback ftl");
    std::fs::write(temp.path().join("i18n/fr-FR"), "not a directory\n")
        .expect("write target locale file");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "fr-FR",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Fluent FTL Add Locale"))
        .stderr(predicate::str::contains(
            "requested locale directory 'fr-FR'",
        ))
        .stderr(predicate::str::contains("test-app"))
        .stderr(predicate::str::contains("not a directory"))
        .stderr(predicate::str::contains("target locale").not());

    assert!(temp.path().join("i18n/fr-FR").is_file());
}

#[test]
fn binary_add_locale_reports_requested_locale_ftl_for_target_ftl_directories() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::create_dir_all(temp.path().join("i18n/fr-FR/test-app.ftl"))
        .expect("create requested locale FTL directory");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n config");
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "fr-FR",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Fluent FTL Add Locale"))
        .stderr(predicate::str::contains("Refusing to add locale data"))
        .stderr(predicate::str::contains("requested"))
        .stderr(predicate::str::contains("FTL path"))
        .stderr(predicate::str::contains("not a"))
        .stderr(predicate::str::contains("file"))
        .stderr(predicate::str::contains("Refusing to sync").not())
        .stderr(predicate::str::contains("target FTL").not());

    assert!(temp.path().join("i18n/fr-FR/test-app.ftl").is_dir());
}

#[test]
fn binary_add_locale_reports_requested_locale_parent_for_namespace_parent_files() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en/test-app"))
        .expect("create fallback namespace");
    std::fs::create_dir_all(temp.path().join("i18n/fr-FR")).expect("create requested locale");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n config");
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback main ftl");
    std::fs::write(
        temp.path().join("i18n/en/test-app/ui.ftl"),
        "button = Button\n",
    )
    .expect("write fallback namespaced ftl");
    let target_main = temp.path().join("i18n/fr-FR/test-app.ftl");
    std::fs::write(&target_main, "hello = Bonjour\n").expect("write requested locale main ftl");
    std::fs::write(temp.path().join("i18n/fr-FR/test-app"), "not a directory\n")
        .expect("write requested locale namespace blocker");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "fr-FR",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Fluent FTL Add Locale"))
        .stderr(predicate::str::contains("Refusing to add locale data"))
        .stderr(predicate::str::contains("requested"))
        .stderr(predicate::str::contains("parent path"))
        .stderr(predicate::str::contains("not a directory"))
        .stderr(predicate::str::contains("Refusing to sync").not())
        .stderr(predicate::str::contains("target parent").not());

    assert_eq!(
        std::fs::read_to_string(target_main).expect("read requested locale main ftl"),
        "hello = Bonjour\n",
        "add-locale should reject blocked namespace paths before writing earlier requested files"
    );
}

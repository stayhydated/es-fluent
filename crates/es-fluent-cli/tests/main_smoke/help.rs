use crate::*;

#[test]
fn binary_help_command_succeeds() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: cargo es-fluent <COMMAND>"))
        .stdout(predicate::str::contains("generate"))
        .stdout(predicate::str::contains("[alias: format]").not());

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
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("unrecognized subcommand 'format'"));

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["help", "generate"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Generate FTL files once for selected crates with i18n.toml",
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
fn binary_rejects_retired_all_option() {
    for subcommand in ["clean", "fmt", "check", "status", "sync", "tree"] {
        Command::cargo_bin("cargo-es-fluent")
            .expect("binary exists")
            .args(["es-fluent", subcommand, "--all"])
            .assert()
            .failure()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains("unexpected argument '--all'"));
    }
}

#[test]
fn binary_sync_help_describes_create_target_scope() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "sync", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Create missing target locale directories for explicit --locale targets; cannot be used with --all-locales",
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
            "scans non-fallback locales even without --all-locales",
        ));
}

#[test]
fn binary_clean_help_describes_inventory_authoritative_scope() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Remove FTL entries and package-owned files absent from Rust inventory",
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
fn binary_check_help_describes_filter_conflict() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "check", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cannot be used with --package"));
}

#[test]
fn action_wrapper_rejects_invalid_boolean_inputs() {
    let action = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("action.yml"),
    )
    .expect("read action.yml");

    assert!(action.contains("all_locales:"));
    assert!(!action.contains("\n  all:\n"));
    assert!(action.contains("action_bool all_locales \"$ES_FLUENT_ALL_LOCALES\""));
    assert!(!action.contains("ES_FLUENT_ALL:"));
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
    assert!(readme.contains("cargo es-fluent tree\ncargo es-fluent tree --all-locales"));
    assert!(book_cli.contains("cargo es-fluent tree\ncargo es-fluent tree --all-locales"));
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
            "Disable fallback-copy warnings during --all-locales checks; requires --all-locales",
        ));
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
            "Text-output hyperlink target mode for message, attribute, and variable rows",
        ))
        .stdout(predicate::str::contains(
            "cannot be used with --output json",
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
    std::fs::remove_file(temp.path().join("i18n/en/test-app.ftl"))
        .expect("remove inventory-stale fixture FTL");
    let workspace = temp.path().to_str().expect("workspace path");

    let cases: &[(&str, &[&str])] = &[
        ("generate", &["generate", "--path", workspace, "--dry-run"]),
        ("watch", &["watch", "--help"]),
        ("clean", &["clean", "--path", workspace, "--dry-run"]),
        ("fmt", &["fmt", "--path", workspace]),
        ("check", &["check", "--path", workspace]),
        (
            "status",
            &["status", "--path", workspace, "--output", "json"],
        ),
        ("doctor", &["doctor", "--help"]),
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

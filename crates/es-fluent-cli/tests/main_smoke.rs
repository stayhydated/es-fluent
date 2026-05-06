mod fixtures;

use assert_cmd::Command;
use predicates::prelude::*;

const SUBCOMMANDS: &[&str] = &[
    "generate",
    "init",
    "watch",
    "clean",
    "fmt",
    "check",
    "doctor",
    "status",
    "sync",
    "add-locale",
    "tree",
];

#[test]
fn binary_help_command_succeeds() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("generate"));
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
fn binary_every_command_has_a_noninteractive_success_path() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");
    let init_temp = assert_fs::TempDir::new().expect("init tempdir");
    let init_path = init_temp.path().to_str().expect("init path");

    let cases: &[(&str, &[&str])] = &[
        (
            "generate",
            &[
                "generate",
                "--path",
                workspace,
                "--package",
                "missing-package",
            ],
        ),
        (
            "init",
            &[
                "init",
                "--path",
                init_path,
                "--locales",
                "fr-FR",
                "--build-rs",
                "--dry-run",
            ],
        ),
        (
            "watch",
            &["watch", "--path", workspace, "--package", "missing-package"],
        ),
        (
            "clean",
            &["clean", "--path", workspace, "--package", "missing-package"],
        ),
        (
            "fmt",
            &["fmt", "--path", workspace, "--package", "missing-package"],
        ),
        (
            "check",
            &["check", "--path", workspace, "--package", "missing-package"],
        ),
        (
            "doctor",
            &[
                "doctor",
                "--path",
                workspace,
                "--package",
                "missing-package",
            ],
        ),
        (
            "status",
            &[
                "status",
                "--path",
                workspace,
                "--package",
                "missing-package",
            ],
        ),
        (
            "sync",
            &[
                "sync",
                "--path",
                workspace,
                "--package",
                "missing-package",
                "--locale",
                "fr-FR",
            ],
        ),
        (
            "add-locale",
            &[
                "add-locale",
                "--path",
                workspace,
                "--package",
                "missing-package",
                "fr-FR",
            ],
        ),
        (
            "tree",
            &["tree", "--path", workspace, "--package", "missing-package"],
        ),
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
fn binary_generate_with_missing_package_filter_succeeds() {
    let temp = fixtures::create_workspace();
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "generate",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--package",
            "missing-package",
        ])
        .assert()
        .success();
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

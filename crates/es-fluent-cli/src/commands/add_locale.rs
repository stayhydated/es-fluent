//! Add-locale command implementation.

use super::common::OutputFormat;
use super::common::WorkspaceArgs;
use super::sync::{SyncArgs, SyncTextMode};
use crate::core::CliError;
use clap::Parser;

/// Arguments for the add-locale command.
#[derive(Debug, Parser)]
pub struct AddLocaleArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Locale(s) to create and seed from the fallback language. Can be passed as separate
    /// arguments or comma-separated.
    #[arg(value_name = "LANG", required = true, value_delimiter = ',')]
    pub locale: Vec<String>,

    /// Dry run - show locale directories and keys that would be added without making changes.
    #[arg(long)]
    pub dry_run: bool,
}

/// Run the add-locale command.
pub fn run_add_locale(args: AddLocaleArgs) -> Result<(), CliError> {
    super::sync::run_sync_with_text_mode(
        SyncArgs {
            workspace: args.workspace,
            locale: args.locale,
            all_locales: false,
            create: true,
            dry_run: args.dry_run,
            output: OutputFormat::Text,
        },
        SyncTextMode::AddLocale,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::common::WorkspaceArgs;
    use fs_err as fs;

    #[test]
    fn run_add_locale_creates_and_seeds_missing_locale() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[(
            "en",
            "hello = Hello\nworld = World\n",
        )]);

        let result = run_add_locale(AddLocaleArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["fr-FR".to_string()],
            dry_run: false,
        });

        assert!(result.is_ok());
        let content = fs::read_to_string(temp.path().join("i18n/fr-FR/test-app.ftl"))
            .expect("read created locale file");
        assert!(content.contains("hello = Hello"));
        assert!(content.contains("world = World"));
    }

    #[test]
    fn run_add_locale_creates_empty_locale_when_fallback_has_no_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("create src");
        fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
        fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n",
        )
        .expect("write manifest");
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write config");

        let result = run_add_locale(AddLocaleArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["fr-FR".to_string()],
            dry_run: false,
        });

        assert!(result.is_ok());
        assert!(temp.path().join("i18n/fr-FR").is_dir());
    }

    #[test]
    #[cfg(unix)]
    fn run_add_locale_rolls_back_created_locales_when_commit_fails() {
        use std::os::unix::fs::PermissionsExt as _;

        let temp = tempfile::tempdir().expect("workspace tempdir");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
        )
        .expect("write workspace manifest");

        for name in ["a", "b"] {
            let root = temp.path().join(name);
            fs::create_dir_all(root.join("src")).expect("create src");
            fs::create_dir_all(root.join("i18n/en")).expect("create fallback locale");
            fs::write(
                root.join("Cargo.toml"),
                format!(
                    "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
                ),
            )
            .expect("write manifest");
            fs::write(root.join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
            fs::write(
                root.join("i18n.toml"),
                "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
            )
            .expect("write config");
        }
        fs::write(temp.path().join("b/i18n/en/b.ftl"), "hello = Hello\n")
            .expect("write second fallback");
        let blocked_parent = temp.path().join("b/i18n");
        fs::set_permissions(&blocked_parent, std::fs::Permissions::from_mode(0o555))
            .expect("make second assets directory read-only");

        let result = run_add_locale(AddLocaleArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["fr-FR".to_string()],
            dry_run: false,
        });

        fs::set_permissions(&blocked_parent, std::fs::Permissions::from_mode(0o755))
            .expect("restore second assets permissions");
        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("add-locale transaction failed") && message.contains("rolled back"))
        );
        assert!(!temp.path().join("a/i18n/fr-FR").exists());
        assert!(!temp.path().join("b/i18n/fr-FR").exists());
    }

    #[test]
    fn run_add_locale_errors_when_package_filter_matches_nothing() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[(
            "en",
            "hello = Hello\nworld = World\n",
        )]);

        let result = run_add_locale(AddLocaleArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: Some("missing-package".to_string()),
            },
            locale: vec!["fr-FR".to_string()],
            dry_run: false,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("missing-package"))
        );
        assert!(!temp.path().join("i18n/fr-FR").exists());
    }

    #[test]
    fn run_add_locale_errors_when_fallback_locale_directory_is_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("create src");
        fs::create_dir_all(temp.path().join("i18n/fr")).expect("create non-fallback locale");
        fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n",
        )
        .expect("write manifest");
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write config");

        let result = run_add_locale(AddLocaleArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["de-DE".to_string()],
            dry_run: false,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("fallback locale directory"))
        );
        assert!(!temp.path().join("i18n/de-DE").exists());
    }

    #[test]
    fn run_add_locale_errors_when_requested_locale_path_is_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("create src");
        fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
        fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n",
        )
        .expect("write manifest");
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write config");
        fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
            .expect("write fallback ftl");
        fs::write(temp.path().join("i18n/fr-FR"), "not a directory\n")
            .expect("write target locale file");

        let result = run_add_locale(AddLocaleArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["fr-FR".to_string()],
            dry_run: false,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("requested locale directory") && message.contains("fr-FR"))
        );
        assert!(temp.path().join("i18n/fr-FR").is_file());
    }

    #[test]
    fn run_add_locale_rejects_root_assets_locales_hidden_by_project_dir_ignores() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("create src");
        fs::create_dir_all(temp.path().join("en")).expect("create fallback locale");
        fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
        )
        .expect("write manifest");
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \".\"\n",
        )
        .expect("write config");
        fs::write(temp.path().join("en/test-app.ftl"), "hello = Hello\n")
            .expect("write fallback ftl");

        let result = run_add_locale(AddLocaleArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["bin".to_string()],
            dry_run: false,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("cannot create requested locale") && message.contains("bin for test-app"))
        );
        assert!(!temp.path().join("bin").exists());
    }

    #[test]
    fn run_add_locale_allows_existing_root_assets_locale_hidden_from_all_scans() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("create src");
        fs::create_dir_all(temp.path().join("en")).expect("create fallback locale");
        fs::create_dir_all(temp.path().join("bin")).expect("create existing target locale");
        fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
        )
        .expect("write manifest");
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \".\"\n",
        )
        .expect("write config");
        fs::write(
            temp.path().join("en/test-app.ftl"),
            "hello = Hello\nworld = World\n",
        )
        .expect("write fallback ftl");
        fs::write(temp.path().join("bin/test-app.ftl"), "hello = Hello\n")
            .expect("write existing target ftl");

        let result = run_add_locale(AddLocaleArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["bin".to_string()],
            dry_run: false,
        });

        assert!(result.is_ok());
        let content =
            fs::read_to_string(temp.path().join("bin/test-app.ftl")).expect("read target ftl");
        assert!(content.contains("world = World"));
    }

    #[test]
    fn run_add_locale_rejects_noncanonical_locale() {
        let temp =
            crate::test_fixtures::create_workspace_with_locales(&[("en", "hello = Hello\n")]);

        let result = run_add_locale(AddLocaleArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["fr-fr".to_string()],
            dry_run: false,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("canonical BCP-47"))
        );
    }

    #[test]
    fn run_add_locale_rejects_fallback_locale() {
        let temp =
            crate::test_fixtures::create_workspace_with_locales(&[("en", "hello = Hello\n")]);

        let result = run_add_locale(AddLocaleArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["en".to_string()],
            dry_run: false,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("fallback locale"))
        );
    }
}

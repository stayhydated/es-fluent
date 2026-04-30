//! Add-locale command implementation.

use super::common::OutputFormat;
use super::common::WorkspaceArgs;
use super::sync::{SyncArgs, run_sync};
use crate::core::CliError;
use clap::Parser;

/// Arguments for the add-locale command.
#[derive(Debug, Parser)]
pub struct AddLocaleArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Locale(s) to create and seed from the fallback language.
    #[arg(value_name = "LANG", required = true, value_delimiter = ',')]
    pub locale: Vec<String>,

    /// Dry run - show what would be created without making changes.
    #[arg(long)]
    pub dry_run: bool,
}

/// Run the add-locale command.
pub fn run_add_locale(args: AddLocaleArgs) -> Result<(), CliError> {
    run_sync(SyncArgs {
        workspace: args.workspace,
        locale: args.locale,
        all: false,
        create: true,
        dry_run: args.dry_run,
        output: OutputFormat::Text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::common::WorkspaceArgs;
    use crate::test_fixtures::create_workspace_with_locales;
    use fs_err as fs;

    #[test]
    fn run_add_locale_creates_and_seeds_missing_locale() {
        let temp = create_workspace_with_locales(&[("en", "hello = Hello\nworld = World\n")]);

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
    fn run_add_locale_rejects_noncanonical_locale() {
        let temp = create_workspace_with_locales(&[("en", "hello = Hello\n")]);

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
}

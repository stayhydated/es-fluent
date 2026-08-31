use super::*;

#[test]
fn run_sync_dry_run_does_not_write_missing_keys() {
    let temp = crate::test_fixtures::create_workspace_with_locales(&[
        ("en", "hello = Hello\nworld = World\n"),
        ("es", "hello = Hola\n"),
    ]);
    let es_path = temp.path().join("i18n/es/test-app.ftl");
    let before = fs::read_to_string(&es_path).expect("read before");

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: vec!["es".to_string()],
        all_locales: false,
        create: false,
        dry_run: true,
        output: OutputFormat::Text,
    });

    assert!(result.is_ok());
    let after = fs::read_to_string(&es_path).expect("read after");
    assert_eq!(before, after, "dry-run should not modify locale files");
}

#[test]
fn run_sync_writes_missing_keys_for_target_locale() {
    let temp = crate::test_fixtures::create_workspace_with_locales(&[
        ("en", "hello = Hello\nworld = World\n"),
        ("es", "hello = Hola\n"),
    ]);
    let es_path = temp.path().join("i18n/es/test-app.ftl");

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: vec!["es".to_string()],
        all_locales: false,
        create: false,
        dry_run: false,
        output: OutputFormat::Text,
    });

    assert!(result.is_ok());
    let es_content = fs::read_to_string(&es_path).expect("read synced es");
    assert!(es_content.contains("world = World"));
}

#[test]
#[cfg(unix)]
fn run_sync_rolls_back_selected_packages_when_commit_fails() {
    use std::os::unix::fs::PermissionsExt as _;

    let temp = tempfile::tempdir().expect("workspace tempdir");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");
    write_sync_workspace_crate(temp.path(), "a", "hello = Hello\nworld = World\n");
    write_sync_workspace_crate(temp.path(), "b", "hello = Hello\n");
    fs::create_dir_all(temp.path().join("a/i18n/fr")).expect("create first target locale");
    fs::create_dir_all(temp.path().join("b/i18n/fr")).expect("create second target locale");
    let first_target = temp.path().join("a/i18n/fr/a.ftl");
    let second_target = temp.path().join("b/i18n/fr/b.ftl");
    fs::write(&first_target, "hello = Bonjour\n").expect("write first target");
    let first_before = fs::read_to_string(&first_target).expect("read first before");
    let blocked_parent = second_target.parent().expect("second target parent");
    fs::set_permissions(blocked_parent, std::fs::Permissions::from_mode(0o555))
        .expect("make second target directory read-only");

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: vec!["fr".to_string()],
        all_locales: false,
        create: false,
        dry_run: false,
        output: OutputFormat::Text,
    });

    fs::set_permissions(blocked_parent, std::fs::Permissions::from_mode(0o755))
        .expect("restore second target permissions");
    assert!(
        matches!(result, Err(CliError::Other(message)) if message.contains("sync transaction failed") && message.contains("rolled back"))
    );
    assert_eq!(
        fs::read_to_string(&first_target).expect("read first after rollback"),
        first_before
    );
    assert!(!second_target.exists());
}

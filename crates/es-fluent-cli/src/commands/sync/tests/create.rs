use super::*;

#[test]
fn run_sync_create_fails_when_no_crates_match_filter() {
    let temp = crate::test_fixtures::create_workspace_with_locales(&[(
        "en",
        "hello = Hello\nworld = World\n",
    )]);

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: Some("missing-package".to_string()),
        },
        locale: vec!["fr-FR".to_string()],
        all_locales: false,
        create: true,
        dry_run: false,
        output: OutputFormat::Text,
    });

    assert!(matches!(result, Err(CliError::Other(message)) if message.contains("missing-package")));
    assert!(!temp.path().join("i18n/fr-FR").exists());
}

#[test]
fn run_sync_create_rejects_assets_dir_path_as_file() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: vec!["fr-FR".to_string()],
        all_locales: false,
        create: true,
        dry_run: true,
        output: OutputFormat::Text,
    });

    assert!(
        matches!(result, Err(CliError::Other(message)) if message.contains("assets_dir for test-app") && message.contains("missing or not a directory"))
    );
    assert!(temp.path().join("i18n").is_file());
}

#[test]
fn run_sync_create_rejects_fallback_target_locale() {
    let temp = crate::test_fixtures::create_workspace_with_locales(&[("en", "hello = Hello\n")]);

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: vec!["en".to_string()],
        all_locales: false,
        create: true,
        dry_run: false,
        output: OutputFormat::Text,
    });

    assert!(matches!(result, Err(CliError::Other(message)) if message.contains("fallback locale")));
}

#[test]
fn run_sync_create_rejects_root_assets_locales_hidden_by_project_dir_ignores() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("src")).expect("create src");
    fs::create_dir_all(temp.path().join("en")).expect("create fallback locale");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \".\"\n",
    )
    .expect("write config");
    fs::write(temp.path().join("en/test-app.ftl"), "hello = Hello\n").expect("write fallback ftl");

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: vec!["bin".to_string()],
        all_locales: false,
        create: true,
        dry_run: false,
        output: OutputFormat::Text,
    });

    assert!(
        matches!(result, Err(CliError::Other(message)) if message.contains("cannot create target locale") && message.contains("bin for test-app"))
    );
    assert!(
        !temp.path().join("bin").exists(),
        "sync --create must not create locales hidden from --all-locales scans"
    );
}

#[test]
fn run_sync_create_allows_existing_root_assets_locale_hidden_from_all_scans() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("src")).expect("create src");
    fs::create_dir_all(temp.path().join("en")).expect("create fallback locale");
    fs::create_dir_all(temp.path().join("bin")).expect("create existing target locale");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
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

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: vec!["bin".to_string()],
        all_locales: false,
        create: true,
        dry_run: false,
        output: OutputFormat::Text,
    });

    assert!(result.is_ok());
    let content =
        fs::read_to_string(temp.path().join("bin/test-app.ftl")).expect("read target ftl");
    assert!(content.contains("world = World"));
}

#[test]
fn run_sync_rejects_create_with_all_locales() {
    let temp = crate::test_fixtures::create_workspace_with_locales(&[
        ("en", "hello = Hello\n"),
        ("es", "hello = Hola\n"),
    ]);

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: Vec::new(),
        all_locales: true,
        create: true,
        dry_run: false,
        output: OutputFormat::Text,
    });

    assert!(
        matches!(result, Err(CliError::Other(message)) if message.contains("--create conflicts with --all-locales"))
    );
}

#[test]
fn run_sync_create_writes_missing_target_locale() {
    let temp = crate::test_fixtures::create_workspace_with_locales(&[(
        "en",
        "hello = Hello\nworld = World\n",
    )]);
    let fr_path = temp.path().join("i18n/fr-FR/test-app.ftl");

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: vec!["fr-FR".to_string()],
        all_locales: false,
        create: true,
        dry_run: false,
        output: OutputFormat::Text,
    });

    assert!(result.is_ok());
    let fr_content = fs::read_to_string(&fr_path).expect("read created locale");
    assert!(fr_content.contains("hello = Hello"));
    assert!(fr_content.contains("world = World"));
}

use super::*;

#[test]
fn relative_sync_message_strips_workspace_paths_for_json_errors() {
    let temp = tempfile::tempdir().expect("tempdir");
    let message = format!(
        "target locale directory 'fr' is not a directory for test-app: {}",
        temp.path().join("i18n/fr").display()
    );

    let normalized = relative_sync_message(&message, temp.path());

    assert_eq!(
        normalized,
        "target locale directory 'fr' is not a directory for test-app: i18n/fr"
    );
}

#[test]
fn test_extract_message_keys() {
    let content = r#"hello = Hello
world = World"#;
    let resource = parser::parse(content.to_string()).unwrap();
    let keys = crate::ftl::extract_message_keys(&resource);

    assert!(keys.contains("hello"));
    assert!(keys.contains("world"));
    assert_eq!(keys.len(), 2);
}

#[test]
fn run_sync_returns_err_when_no_locales_specified() {
    let temp = crate::test_fixtures::create_workspace_with_locales(&[
        ("en", "hello = Hello\nworld = World\n"),
        ("es", "hello = Hola\n"),
    ]);

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: Vec::new(),
        all_locales: false,
        create: false,
        dry_run: false,
        output: OutputFormat::Text,
    });

    assert!(result.is_err());
}

#[test]
fn run_sync_fails_when_no_crates_match_filter() {
    let temp = crate::test_fixtures::create_workspace_with_locales(&[
        ("en", "hello = Hello\nworld = World\n"),
        ("es", "hello = Hola\n"),
    ]);

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: Some("missing-package".to_string()),
        },
        locale: vec!["es".to_string()],
        all_locales: false,
        create: false,
        dry_run: false,
        output: OutputFormat::Text,
    });

    assert!(
        matches!(result, Err(CliError::Other(message)) if message.contains("cannot sync locales") && message.contains("missing-package"))
    );
}

#[test]
fn run_sync_rejects_missing_target_selection_before_workspace_discovery() {
    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(std::path::PathBuf::from("/definitely/missing/path")),
            package: None,
        },
        locale: Vec::new(),
        all_locales: false,
        create: false,
        dry_run: false,
        output: OutputFormat::Text,
    });

    assert!(
        matches!(result, Err(CliError::Other(message)) if message.contains("no target locales specified"))
    );
}

#[test]
fn run_sync_rejects_target_selection_conflicts_before_workspace_discovery() {
    let cases = [
        (
            true,
            true,
            Vec::new(),
            "--create conflicts with --all-locales",
        ),
        (
            false,
            true,
            Vec::new(),
            "--create requires explicit --locale targets",
        ),
        (
            true,
            false,
            vec!["fr-FR".to_string()],
            "--all-locales cannot be combined with --locale",
        ),
    ];

    for (all_locales, create, locale, expected) in cases {
        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(std::path::PathBuf::from("/definitely/missing/path")),
                package: None,
            },
            locale,
            all_locales,
            create,
            dry_run: false,
            output: OutputFormat::Text,
        });

        assert!(
            matches!(&result, Err(CliError::Other(message)) if message.contains(expected)),
            "expected {expected:?}, got {result:?}"
        );
    }
}

#[test]
fn run_sync_fails_for_unknown_locale() {
    let temp = crate::test_fixtures::create_workspace_with_locales(&[
        ("en", "hello = Hello\nworld = World\n"),
        ("es", "hello = Hola\n"),
    ]);

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: vec!["zz-unknown".to_string()],
        all_locales: false,
        create: false,
        dry_run: false,
        output: OutputFormat::Text,
    });

    assert!(
        matches!(result, Err(CliError::Other(message)) if message.contains("zz-unknown for test-app"))
    );
}

#[test]
fn run_sync_trims_comma_separated_locale_values() {
    let temp = crate::test_fixtures::create_workspace_with_locales(&[
        ("en", "hello = Hello\nworld = World\n"),
        ("fr", "hello = Bonjour\n"),
    ]);

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: vec![" fr ".to_string()],
        all_locales: false,
        create: false,
        dry_run: true,
        output: OutputFormat::Text,
    });

    assert!(result.is_ok());
    let content = fs::read_to_string(temp.path().join("i18n/fr/test-app.ftl"))
        .expect("target FTL should remain readable");
    assert!(!content.contains("world = World"));
}

#[test]
fn run_sync_rejects_empty_comma_separated_locale_values() {
    let temp = crate::test_fixtures::create_workspace_with_locales(&[
        ("en", "hello = Hello\nworld = World\n"),
        ("fr", "hello = Bonjour\n"),
    ]);

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: vec![" ".to_string()],
        all_locales: false,
        create: false,
        dry_run: true,
        output: OutputFormat::Text,
    });

    assert!(
        matches!(result, Err(CliError::Other(message)) if message.contains("locale values must not be empty"))
    );
}

#[test]
fn run_sync_rejects_noncanonical_locale_values_with_form_hint() {
    let temp = crate::test_fixtures::create_workspace_with_locales(&[
        ("en", "hello = Hello\nworld = World\n"),
        ("fr", "hello = Bonjour\n"),
    ]);

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: vec!["iw".to_string()],
        all_locales: false,
        create: false,
        dry_run: true,
        output: OutputFormat::Text,
    });

    assert!(matches!(
        result,
        Err(CliError::Other(message))
            if message.contains("locale 'iw' must use canonical BCP-47 form 'he'")
                && !message.contains("casing")
    ));
}

#[test]
fn run_sync_rejects_explicit_target_locale_path_as_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("src")).expect("create src");
    fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n config");
    fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback ftl");
    fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write target locale file");

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: vec!["fr".to_string()],
        all_locales: false,
        create: false,
        dry_run: true,
        output: OutputFormat::Text,
    });

    assert!(
        matches!(result, Err(CliError::Other(message)) if message.contains("target locale path") && message.contains("fr for test-app") && message.contains("not directories"))
    );
    assert!(temp.path().join("i18n/fr").is_file());
}

#[test]
fn run_sync_all_rejects_locale_named_asset_path_as_file() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: Vec::new(),
        all_locales: true,
        create: false,
        dry_run: true,
        output: OutputFormat::Text,
    });

    assert!(
        matches!(result, Err(CliError::Other(message)) if message.contains("locale path") && message.contains("fr for test-app") && message.contains("not directories"))
    );
    assert!(temp.path().join("i18n/fr").is_file());
}

#[test]
fn run_sync_all_rejects_missing_assets_dir() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: Vec::new(),
        all_locales: true,
        create: false,
        dry_run: true,
        output: OutputFormat::Text,
    });

    assert!(
        matches!(result, Err(CliError::Other(message)) if message.contains("assets_dir for test-app") && message.contains("missing or not a directory"))
    );
}

#[test]
fn run_sync_all_rejects_assets_dir_path_as_file() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: Vec::new(),
        all_locales: true,
        create: false,
        dry_run: true,
        output: OutputFormat::Text,
    });

    assert!(
        matches!(result, Err(CliError::Other(message)) if message.contains("assets_dir for test-app") && message.contains("missing or not a directory"))
    );
    assert!(temp.path().join("i18n").is_file());
}

#[test]
fn run_sync_explicit_target_rejects_assets_dir_path_as_file() {
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
        create: false,
        dry_run: true,
        output: OutputFormat::Text,
    });

    assert!(
        matches!(result, Err(CliError::Other(message)) if message.contains("assets_dir for test-app") && message.contains("missing or not a directory"))
    );
    assert!(temp.path().join("i18n").is_file());
}

#[test]
fn run_sync_rejects_fallback_target_locale() {
    let temp = crate::test_fixtures::create_workspace_with_locales(&[
        ("en", "hello = Hello\nworld = World\n"),
        ("es", "hello = Hola\n"),
    ]);

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: vec!["en".to_string()],
        all_locales: false,
        create: false,
        dry_run: false,
        output: OutputFormat::Text,
    });

    assert!(matches!(result, Err(CliError::Other(message)) if message.contains("fallback locale")));
}

#[test]
fn run_sync_explicit_target_ignores_unrelated_noncanonical_locale_dir() {
    let temp = crate::test_fixtures::create_workspace_with_locales(&[(
        "en",
        "hello = Hello\nworld = World\n",
    )]);
    fs::create_dir_all(temp.path().join("i18n/en-us")).expect("create unrelated bad locale");

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
        result.is_ok(),
        "explicit sync targets should not scan unrelated locale dirs: {result:?}"
    );
    assert!(
        !temp.path().join("i18n/fr-FR").exists(),
        "dry-run explicit sync should preview creation without writing"
    );
}

#[test]
fn run_sync_all_processes_non_fallback_locales() {
    let temp = crate::test_fixtures::create_workspace_with_locales(&[
        ("en", "hello = Hello\nworld = World\n"),
        ("es", "hello = Hola\n"),
    ]);
    fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr");
    fs::write(temp.path().join("i18n/fr/test-app.ftl"), "hello = Salut\n").expect("write fr");

    let result = run_sync(SyncArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        locale: Vec::new(),
        all_locales: true,
        create: false,
        dry_run: false,
        output: OutputFormat::Text,
    });

    assert!(result.is_ok());
    let fr_content = fs::read_to_string(temp.path().join("i18n/fr/test-app.ftl")).expect("read fr");
    assert!(fr_content.contains("world = World"));
}

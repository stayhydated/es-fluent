use super::*;

#[test]
fn run_sync_requires_explicit_locale_in_every_selected_crate() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("a/src")).expect("create a src");
    fs::create_dir_all(temp.path().join("a/i18n/en")).expect("create a en");
    fs::create_dir_all(temp.path().join("a/i18n/fr")).expect("create a fr");
    fs::create_dir_all(temp.path().join("b/src")).expect("create b src");
    fs::create_dir_all(temp.path().join("b/i18n/en")).expect("create b en");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");
    fs::write(
        temp.path().join("a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write a manifest");
    fs::write(
        temp.path().join("b/Cargo.toml"),
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write b manifest");
    fs::write(temp.path().join("a/src/lib.rs"), "pub fn a() {}\n").expect("write a lib");
    fs::write(temp.path().join("b/src/lib.rs"), "pub fn b() {}\n").expect("write b lib");
    fs::write(
        temp.path().join("a/i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write a config");
    fs::write(
        temp.path().join("b/i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write b config");
    fs::write(
        temp.path().join("a/i18n/en/a.ftl"),
        "hello = Hello\nworld = World\n",
    )
    .expect("write a fallback");
    fs::write(temp.path().join("a/i18n/fr/a.ftl"), "hello = Bonjour\n").expect("write a fr");
    fs::write(
        temp.path().join("b/i18n/en/b.ftl"),
        "hello = Hello\nworld = World\n",
    )
    .expect("write b fallback");

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
        matches!(result, Err(CliError::Other(message)) if message.contains("fr for b") && message.contains("--create"))
    );
    assert!(
        !temp.path().join("b/i18n/fr").exists(),
        "sync without --create must not create the missing locale"
    );
}

#[test]
fn run_sync_create_preflights_selected_workspace_before_writing() {
    let temp = tempfile::tempdir().expect("workspace tempdir");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");
    write_sync_workspace_crate(temp.path(), "a", "hello = Hello\n");
    write_sync_workspace_crate(temp.path(), "b", "hello = Hello\n");
    fs::write(temp.path().join("b/i18n/fr-FR"), "not a directory\n")
        .expect("write target locale blocker");

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

    assert!(
        matches!(result, Err(CliError::Other(message)) if message.contains("target locale directory") && message.contains("b"))
    );
    assert!(
        !temp.path().join("a/i18n/fr-FR").exists(),
        "sync --create should not write earlier crates before preflighting later crates"
    );
    assert!(temp.path().join("b/i18n/fr-FR").is_file());
}

#[test]
fn collect_affected_locale_targets_deduplicates_namespaced_file_results() {
    let temp = crate::test_fixtures::create_workspace_with_locales(&[
        ("en", "hello = Hello\nworld = World\n"),
        ("es", "hello = Hola\n"),
    ]);
    fs::create_dir_all(temp.path().join("i18n/en/test-app")).expect("create en namespace dir");
    fs::write(
        temp.path().join("i18n/en/test-app/ui.ftl"),
        "button = Button\n",
    )
    .expect("write en namespaced fallback");

    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: None,
    })
    .expect("discover workspace");
    let krate = workspace.crates.first().expect("crate");
    let targets = HashSet::from(["es".to_string()]);

    let results = locale::sync_crate(krate, Some(&targets), true, false).expect("sync crate");

    assert_eq!(
        results
            .iter()
            .filter(|result| result.keys_added > 0)
            .count(),
        2,
        "both locale files should report changes"
    );
    let paths = results
        .iter()
        .filter_map(|result| result.path.as_deref())
        .collect::<HashSet<_>>();
    assert_eq!(paths.len(), 2, "each result should identify its FTL file");
    assert_eq!(
        collect_affected_locale_targets(krate.name.as_str(), results.iter()).len(),
        1
    );
}

#[test]
fn collect_affected_locale_targets_counts_the_same_locale_in_different_crates() {
    let results = [
        locale::SyncLocaleResult {
            locale: "fr".to_string(),
            path: Some(std::path::PathBuf::from("i18n/fr/a.ftl")),
            locale_created: false,
            keys_added: 1,
            added_keys: vec!["hello".to_string()],
            diff_info: None,
        },
        locale::SyncLocaleResult {
            locale: "fr".to_string(),
            path: None,
            locale_created: true,
            keys_added: 0,
            added_keys: Vec::new(),
            diff_info: None,
        },
    ];

    let mut affected = HashSet::new();
    affected.extend(collect_affected_locale_targets("a", results.iter()));
    affected.extend(collect_affected_locale_targets("b", results.iter()));

    assert_eq!(affected.len(), 2);
    assert!(affected.contains(&("a".to_string(), "fr".to_string())));
    assert!(affected.contains(&("b".to_string(), "fr".to_string())));
}

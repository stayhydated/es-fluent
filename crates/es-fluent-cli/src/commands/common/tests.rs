use super::*;
use crate::core::{CrateInfo, FluentParseMode, GenerationAction, WorkspaceInfo};
use crate::test_fixtures::FakeRunnerBehavior;
use std::cell::Cell;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

fn package(name: &str) -> es_fluent_runner::PackageName {
    es_fluent_runner::PackageName::try_new(name).expect("valid package name")
}

fn create_workspace_info(temp: &tempfile::TempDir) -> WorkspaceInfo {
    let manifest_dir = temp.path().to_path_buf();
    let src_dir = manifest_dir.join("src");
    let i18n_toml = manifest_dir.join("i18n.toml");
    let krate = CrateInfo {
        name: package("test-app"),
        manifest_dir: crate::core::ManifestDir::from_discovered(manifest_dir.clone()),
        src_dir: crate::core::SourceDir::from_discovered(src_dir),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(i18n_toml),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
            manifest_dir.join("i18n/en"),
        ),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };

    WorkspaceInfo {
        root_dir: manifest_dir.clone(),
        target_dir: manifest_dir.join("target"),
        crates: vec![krate],
    }
}

fn write_i18n_workspace_member(root: &std::path::Path, name: &str) {
    let manifest_dir = root.join(name);
    fs::create_dir_all(manifest_dir.join("src")).expect("create src");
    fs::write(
        manifest_dir.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
        ),
    )
    .expect("write manifest");
    fs::write(manifest_dir.join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    fs::write(
        manifest_dir.join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n config");
}

#[test]
fn workspace_discovery_rejects_empty_package_filter_before_path_validation() {
    let result = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(PathBuf::from("/definitely/missing/path")),
        package: Some("  ".to_string()),
    });

    assert!(
        matches!(&result, Err(CliError::Other(message)) if message.contains("package filter must not be empty")),
        "unexpected result: {result:?}"
    );
}

#[test]
fn read_changed_status_handles_missing_invalid_and_valid_json() {
    let temp = tempfile::tempdir().unwrap();
    let crate_name = "demo";
    let store = es_fluent_runner::RunnerMetadataStore::new(temp.path());
    let package_name = package(crate_name);
    let result_path = store.result_path(&package_name);
    fs::create_dir_all(result_path.parent().unwrap()).unwrap();

    assert!(!store.result_changed(&package_name));

    fs::write(&result_path, "{not-json").unwrap();
    assert!(!store.result_changed(&package_name));

    fs::write(&result_path, r#"{"changed":true}"#).unwrap();
    assert!(store.result_changed(&package_name));
}

#[test]
fn render_generation_results_reports_error_presence() {
    let success = GenerateResult::success(
        package("ok-crate"),
        Duration::from_millis(10),
        1,
        None,
        false,
    );
    let failure = GenerateResult::failure(
        package("bad-crate"),
        Duration::from_millis(5),
        "boom".to_string(),
    );

    let success_calls = Cell::new(0usize);
    let error_calls = Cell::new(0usize);

    let has_errors = render_generation_results(
        &[success, failure],
        |_| success_calls.set(success_calls.get() + 1),
        |_| error_calls.set(error_calls.get() + 1),
    );

    assert!(has_errors);
    assert_eq!(success_calls.get(), 1);
    assert_eq!(error_calls.get(), 1);
}

#[test]
fn generation_verb_labels_match_expected_text() {
    assert_eq!(
        GenerationVerb::Generate.dry_run_label(),
        "would be generated in"
    );
    assert_eq!(GenerationVerb::Clean.dry_run_label(), "would be cleaned in");
}

#[test]
fn workspace_discover_supports_package_filtering() {
    let temp = crate::test_fixtures::create_test_crate_workspace_without_ftl();

    let all = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: None,
    })
    .unwrap();
    assert_eq!(all.crates.len(), 1);
    assert_eq!(all.valid.len(), 1);

    let filtered = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: Some("missing-crate".to_string()),
    })
    .unwrap();
    assert!(filtered.crates.is_empty());
    assert!(filtered.valid.is_empty());
}

#[test]
fn workspace_discover_scopes_member_path_to_that_member_by_default() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");
    write_i18n_workspace_member(temp.path(), "a");
    write_i18n_workspace_member(temp.path(), "b");

    let all = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: None,
    })
    .expect("discover workspace root");
    assert_eq!(
        all.crates
            .iter()
            .map(|krate| krate.name.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"]
    );

    let all_from_manifest = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().join("Cargo.toml")),
        package: None,
    })
    .expect("discover workspace root manifest");
    assert_eq!(
        all_from_manifest
            .crates
            .iter()
            .map(|krate| krate.name.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"]
    );

    let member = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().join("a")),
        package: None,
    })
    .expect("discover workspace member");
    assert_eq!(member.crates.len(), 1);
    assert_eq!(member.crates[0].name, "a");

    let nested_member = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().join("a/src")),
        package: None,
    })
    .expect("discover nested workspace member path");
    assert_eq!(nested_member.crates.len(), 1);
    assert_eq!(nested_member.crates[0].name, "a");

    let member_file = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().join("a/src/lib.rs")),
        package: None,
    })
    .expect("discover workspace member file path");
    assert_eq!(member_file.crates.len(), 1);
    assert_eq!(member_file.crates[0].name, "a");

    let explicit_package = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().join("a")),
        package: Some("b".to_string()),
    })
    .expect("discover explicit package from member path");
    assert_eq!(explicit_package.crates.len(), 1);
    assert_eq!(explicit_package.crates[0].name, "b");
}

#[cfg(unix)]
#[test]
fn workspace_discover_scopes_symlinked_member_path_by_lexical_location() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().canonicalize().expect("canonical tempdir");
    let outside = tempfile::tempdir().expect("outside tempdir");
    fs::write(
        workspace_root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");
    write_i18n_workspace_member(&workspace_root, "a");
    write_i18n_workspace_member(&workspace_root, "b");
    std::os::unix::fs::symlink(outside.path(), workspace_root.join("a/src/external"))
        .expect("create symlink inside member");

    let selected = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(workspace_root.join("a/src/external")),
        package: None,
    })
    .expect("discover symlinked path inside workspace member");

    assert_eq!(selected.crates.len(), 1);
    assert_eq!(selected.crates[0].name, "a");
}

#[test]
fn workspace_discover_member_path_without_i18n_does_not_select_siblings() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    let a_dir = temp.path().join("a");
    fs::create_dir_all(a_dir.join("src")).expect("create a src");
    fs::write(
        a_dir.join("Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write a manifest");
    fs::write(a_dir.join("src/lib.rs"), "pub fn marker() {}\n").expect("write a lib");

    write_i18n_workspace_member(temp.path(), "b");

    let selected = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(a_dir),
        package: None,
    })
    .expect("discover workspace member without i18n");

    assert!(
        selected.crates.is_empty(),
        "member path without i18n.toml should not select configured siblings"
    );
    assert_eq!(
        selected.empty_selection_message().as_deref(),
        Some("no crates with i18n.toml were found")
    );

    let selected_nested = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().join("a/src")),
        package: None,
    })
    .expect("discover nested workspace member path without i18n");

    assert!(
        selected_nested.crates.is_empty(),
        "nested member path without i18n.toml should not select configured siblings"
    );
}

#[test]
fn workspace_discover_workspace_subdir_without_i18n_member_match_selects_no_crates() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");
    fs::create_dir_all(temp.path().join("tools")).expect("create workspace subdir");
    write_i18n_workspace_member(temp.path(), "b");

    let selected = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().join("tools")),
        package: None,
    })
    .expect("discover workspace subdir");

    assert!(
        selected.crates.is_empty(),
        "workspace subdirectories should not silently widen to all configured crates"
    );
}

#[test]
fn run_generation_for_crates_uses_cached_runner_and_reads_changed_status() {
    let temp = crate::test_fixtures::create_test_crate_workspace_without_ftl();
    let workspace = create_workspace_info(&temp);
    let krate = workspace.crates[0].clone();

    crate::test_fixtures::setup_fake_runner_and_cache(
        &temp,
        FakeRunnerBehavior::stdout("generated-from-fake-runner\n"),
    );

    let temp_dir = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(&workspace.root_dir);
    let result_json = temp_dir.result_path(&krate.name);
    fs::create_dir_all(result_json.parent().unwrap()).expect("create metadata dir");
    fs::write(&result_json, r#"{"changed":true}"#).expect("write result json");

    let results = run_generation_for_crates(
        &workspace,
        std::slice::from_ref(&krate),
        &GenerationAction::Generate {
            mode: FluentParseMode::default(),
            dry_run: false,
        },
        false,
        false,
    );

    assert_eq!(results.len(), 1);
    assert!(results[0].error.is_none());
    assert!(results[0].changed);
    assert!(
        results[0]
            .output
            .as_ref()
            .expect("captured output")
            .contains("generated-from-fake-runner")
    );
}

#[test]
fn run_generation_for_crates_links_only_requested_crates() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    let mut crates = Vec::new();
    for name in ["a", "b"] {
        let manifest_dir = temp.path().join(name);
        let src_dir = manifest_dir.join("src");
        let i18n_toml = manifest_dir.join("i18n.toml");
        fs::create_dir_all(&src_dir).expect("create src");
        fs::create_dir_all(manifest_dir.join("i18n/en")).expect("create i18n");
        fs::write(
            manifest_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write manifest");
        fs::write(src_dir.join("lib.rs"), "pub fn marker() {}\n").expect("write lib");
        fs::write(
            &i18n_toml,
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n config");
        fs::write(
            manifest_dir.join(format!("i18n/en/{name}.ftl")),
            "hello = Hello\n",
        )
        .expect("write ftl");

        crates.push(CrateInfo {
            name: package(name),
            manifest_dir: crate::core::ManifestDir::from_discovered(manifest_dir.clone()),
            src_dir: crate::core::SourceDir::from_discovered(src_dir),
            library_target_path: None,
            custom_build_target_path: None,
            i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(i18n_toml),
            ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
                manifest_dir.join("i18n/en"),
            ),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        });
    }

    let workspace = WorkspaceInfo {
        root_dir: temp.path().to_path_buf(),
        target_dir: temp.path().join("target"),
        crates,
    };
    let krate = workspace.crates[0].clone();

    let temp_store = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(temp.path());
    let binary_path =
        crate::test_fixtures::fake_runner_binary_path_for_workspace(&workspace.root_dir);
    let mut crate_hashes = indexmap::IndexMap::new();
    crate_hashes.insert(
        krate.name.clone(),
        crate::generation::cache::compute_crate_inputs_hash(
            &krate.manifest_dir,
            &krate.src_dir,
            Some(&krate.i18n_config_path),
            krate.custom_build_target_path.as_deref(),
        )
        .expect("test fixture has a determinate source graph"),
    );
    crate::test_fixtures::install_fake_runner_with_cache(
        &binary_path,
        &temp_store,
        temp.path(),
        &FakeRunnerBehavior::silent_success(),
        env!("CARGO_PKG_VERSION"),
        crate_hashes,
    );

    let results = run_generation_for_crates(
        &workspace,
        std::slice::from_ref(&krate),
        &GenerationAction::Generate {
            mode: FluentParseMode::default(),
            dry_run: true,
        },
        false,
        false,
    );

    assert_eq!(results.len(), 1);
    assert!(results[0].error.is_none(), "{:?}", results[0].error);

    let runner_manifest =
        fs::read_to_string(temp_store.base_dir().join("Cargo.toml")).expect("runner manifest");
    let runner_manifest: toml::Value =
        toml::from_str(&runner_manifest).expect("parse runner manifest");
    let dependencies = runner_manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .expect("dependencies table");
    assert!(dependencies.contains_key("a"));
    assert!(
        !dependencies.contains_key("b"),
        "runner should not link unrequested crates: {dependencies:?}"
    );
}

#[test]
#[cfg(unix)]
fn run_generation_for_crates_rolls_back_all_packages_when_commit_fails() {
    use std::os::unix::fs::PermissionsExt as _;

    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    let mut crates = Vec::new();
    for name in ["a", "b"] {
        let manifest_dir = temp.path().join(name);
        let src_dir = manifest_dir.join("src");
        let i18n_toml = manifest_dir.join("i18n.toml");
        fs::create_dir_all(&src_dir).expect("create src");
        fs::create_dir_all(manifest_dir.join("i18n/en")).expect("create i18n");
        fs::write(
            manifest_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write manifest");
        fs::write(src_dir.join("lib.rs"), "pub fn marker() {}\n").expect("write lib");
        fs::write(
            &i18n_toml,
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n config");
        crates.push(CrateInfo {
            name: package(name),
            manifest_dir: crate::core::ManifestDir::from_discovered(manifest_dir.clone()),
            src_dir: crate::core::SourceDir::from_discovered(src_dir),
            library_target_path: None,
            custom_build_target_path: None,
            i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(i18n_toml),
            ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
                manifest_dir.join("i18n/en"),
            ),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        });
    }

    let first_path = temp.path().join("a/i18n/en/a.ftl");
    fs::write(&first_path, "first = Original\n").expect("write first original");
    let blocked_dir = temp.path().join("b/i18n/en/blocked");
    fs::create_dir(&blocked_dir).expect("create blocked directory");
    let second_path = blocked_dir.join("b.ftl");

    let mut first_transaction = es_fluent_runner::FileTransaction::default();
    first_transaction
        .plan_write(&first_path, b"first = Updated\n".to_vec())
        .expect("plan first write");
    let mut second_transaction = es_fluent_runner::FileTransaction::default();
    second_transaction
        .plan_write(&second_path, b"second = Updated\n".to_vec())
        .expect("plan second write");
    fs::set_permissions(&blocked_dir, std::fs::Permissions::from_mode(0o555))
        .expect("make second target read-only");

    let workspace = WorkspaceInfo {
        root_dir: temp.path().to_path_buf(),
        target_dir: temp.path().join("target"),
        crates,
    };
    let temp_store = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(temp.path());
    for (krate, transaction) in workspace
        .crates
        .iter()
        .zip([first_transaction, second_transaction])
    {
        temp_store
            .write_result(
                &krate.name,
                &es_fluent_runner::RunnerResult {
                    changed: true,
                    transaction,
                },
            )
            .expect("write runner result");
    }

    let binary_path =
        crate::test_fixtures::fake_runner_binary_path_for_workspace(&workspace.root_dir);
    let crate_hashes = workspace
        .crates
        .iter()
        .map(|krate| {
            (
                krate.name.clone(),
                crate::generation::cache::compute_crate_inputs_hash(
                    &krate.manifest_dir,
                    &krate.src_dir,
                    Some(&krate.i18n_config_path),
                    krate.custom_build_target_path.as_deref(),
                )
                .expect("test fixture has a determinate source graph"),
            )
        })
        .collect();
    crate::test_fixtures::install_fake_runner_with_cache(
        &binary_path,
        &temp_store,
        temp.path(),
        &FakeRunnerBehavior::silent_success(),
        env!("CARGO_PKG_VERSION"),
        crate_hashes,
    );

    let results = run_generation_for_crates(
        &workspace,
        &workspace.crates,
        &GenerationAction::Generate {
            mode: FluentParseMode::default(),
            dry_run: false,
        },
        false,
        false,
    );

    fs::set_permissions(&blocked_dir, std::fs::Permissions::from_mode(0o755))
        .expect("restore target permissions");
    assert!(results.iter().all(|result| result.error.is_some()));
    assert_eq!(
        fs::read_to_string(first_path).expect("read first after rollback"),
        "first = Original\n"
    );
    assert!(!second_path.exists());
}

#[test]
fn workspace_print_discovery_handles_empty_and_skipped_crates() {
    let empty = WorkspaceCrates {
        workspace_info: WorkspaceInfo {
            root_dir: PathBuf::from("."),
            target_dir: PathBuf::from("./target"),
            crates: Vec::new(),
        },
        crates: Vec::new(),
        valid: Vec::new(),
        skipped: Vec::new(),
        package_not_found: None,
        all_i18n_package_names: Vec::new(),
    };
    assert!(!empty.print_discovery(|| {}));

    let skipped_crate = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("missing-lib").expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(PathBuf::from("/tmp/test")),
        src_dir: crate::core::SourceDir::from_discovered(PathBuf::from("/tmp/test/src")),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(PathBuf::from(
            "/tmp/test/i18n.toml",
        )),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(PathBuf::from(
            "/tmp/test/i18n/en",
        )),
        has_lib_rs: false,
        fluent_features: Vec::new(),
    };
    let non_empty = WorkspaceCrates {
        workspace_info: WorkspaceInfo {
            root_dir: PathBuf::from("."),
            target_dir: PathBuf::from("./target"),
            crates: vec![skipped_crate.clone()],
        },
        crates: vec![skipped_crate.clone()],
        valid: Vec::new(),
        skipped: vec![skipped_crate],
        package_not_found: None,
        all_i18n_package_names: vec!["missing-lib".to_string()],
    };
    assert!(non_empty.print_discovery(|| {}));
}

#[test]
fn run_generation_for_crates_returns_failures_when_runner_preparation_fails() {
    let krate = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("broken").expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(PathBuf::from("/dev/null")),
        src_dir: crate::core::SourceDir::from_discovered(PathBuf::from("/dev/null/src")),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(PathBuf::from(
            "/dev/null/i18n.toml",
        )),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(PathBuf::from(
            "/dev/null/i18n/en",
        )),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };
    let workspace = WorkspaceInfo {
        root_dir: PathBuf::from("/dev/null"),
        target_dir: PathBuf::from("/dev/null/target"),
        crates: vec![krate.clone()],
    };

    let results = run_generation_for_crates(
        &workspace,
        std::slice::from_ref(&krate),
        &GenerationAction::Generate {
            mode: FluentParseMode::default(),
            dry_run: false,
        },
        false,
        false,
    );

    assert_eq!(results.len(), 1);
    assert!(results[0].error.is_some());
}

#[test]
fn run_generation_for_crates_handles_empty_output_and_dry_run_render_paths() {
    let temp = crate::test_fixtures::create_test_crate_workspace_without_ftl();
    let workspace = create_workspace_info(&temp);
    let krate = workspace.crates[0].clone();

    crate::test_fixtures::setup_fake_runner_and_cache(&temp, FakeRunnerBehavior::silent_success());

    let results = run_generation_for_crates(
        &workspace,
        std::slice::from_ref(&krate),
        &GenerationAction::Generate {
            mode: FluentParseMode::default(),
            dry_run: true,
        },
        false,
        false,
    );
    assert_eq!(results.len(), 1);
    assert!(results[0].error.is_none());
    assert!(
        results[0].output.is_none(),
        "empty runner output should map to None"
    );

    let dry_run_has_errors =
        render_generation_results_with_dry_run(&results, true, GenerationVerb::Generate);
    assert!(!dry_run_has_errors);

    let clean_result = GenerateResult::success(
        package("crate-clean"),
        Duration::from_millis(1),
        1,
        None,
        true,
    );
    let clean_has_errors =
        render_generation_results_with_dry_run(&[clean_result], false, GenerationVerb::Clean);
    assert!(!clean_has_errors);
}

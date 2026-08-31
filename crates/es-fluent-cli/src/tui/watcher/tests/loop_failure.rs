use super::*;

#[test]
fn watch_all_errors_when_no_crates_provided() {
    let workspace = WorkspaceInfo {
        root_dir: PathBuf::from("/tmp/ws"),
        target_dir: PathBuf::from("/tmp/ws/target"),
        crates: Vec::new(),
    };

    let result = super::watch_all(&[], &workspace, &FluentParseMode::default());
    assert!(result.is_err());
}

#[test]
fn run_watch_loop_with_poll_handles_non_library_crates() {
    let temp = tempfile::tempdir().expect("tempdir");
    let crate_without_lib = test_crate("no-lib", false);
    let workspace = WorkspaceInfo {
        root_dir: temp.path().to_path_buf(),
        target_dir: temp.path().join("target"),
        crates: vec![crate_without_lib.clone()],
    };

    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).expect("create terminal");
    let result = super::run_watch_loop_with_poll(
        &mut terminal,
        &[crate_without_lib],
        &workspace,
        &FluentParseMode::default(),
        always_quit,
        Some(2),
    );

    assert!(result.is_ok());
}

#[test]
fn run_watch_loop_with_poll_processes_initial_generation_for_valid_crate() {
    let temp = tempfile::tempdir().expect("tempdir");
    let record_args_path = temp.path().join("watch-runner-args");
    let (_workspace_temp, workspace, krate) = create_valid_workspace_with_fake_runner_behavior(
        FakeRunnerBehavior::record_args(&record_args_path),
    );
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).expect("create terminal");

    let result = super::run_watch_loop_with_poll(
        &mut terminal,
        &[krate],
        &workspace,
        &FluentParseMode::default(),
        always_quit,
        Some(10),
    );

    assert!(result.is_ok());
    assert!(
        record_args_path.is_file(),
        "quitting should wait for the initial generation process"
    );
}

#[test]
fn run_watch_loop_with_file_rx_records_watcher_errors() {
    let crate_without_lib = test_crate("no-lib", false);
    let workspace = WorkspaceInfo {
        root_dir: PathBuf::from("/tmp/ws"),
        target_dir: PathBuf::from("/tmp/ws/target"),
        crates: vec![crate_without_lib.clone()],
    };
    let (tx, rx) = crossbeam_channel::unbounded();
    tx.send(Err(vec![notify::Error::generic("watch failed")]))
        .expect("send watcher error");
    drop(tx);

    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).expect("create terminal");
    let result = super::run_watch_loop_with_file_rx(
        &mut terminal,
        &[crate_without_lib],
        &workspace,
        &FluentParseMode::default(),
        rx,
        never_quit,
        Some(2),
    );

    assert!(result.is_ok());
}

#[test]
fn run_watch_loop_with_file_rx_exits_when_file_channel_disconnects() {
    let crate_without_lib = test_crate("no-lib", false);
    let workspace = WorkspaceInfo {
        root_dir: PathBuf::from("/tmp/ws"),
        target_dir: PathBuf::from("/tmp/ws/target"),
        crates: vec![crate_without_lib.clone()],
    };
    let (tx, rx) = crossbeam_channel::unbounded();
    drop(tx);

    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).expect("create terminal");
    let result = super::run_watch_loop_with_file_rx(
        &mut terminal,
        &[crate_without_lib],
        &workspace,
        &FluentParseMode::default(),
        rx,
        never_quit,
        Some(2),
    );

    assert!(result.is_ok());
}

#[test]
fn run_watch_loop_with_file_rx_accepts_no_iteration_limit_when_poll_quits() {
    let crate_without_lib = test_crate("no-lib", false);
    let workspace = WorkspaceInfo {
        root_dir: PathBuf::from("/tmp/ws"),
        target_dir: PathBuf::from("/tmp/ws/target"),
        crates: vec![crate_without_lib.clone()],
    };
    let (_tx, rx) = crossbeam_channel::unbounded();

    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).expect("create terminal");
    let result = super::run_watch_loop_with_file_rx(
        &mut terminal,
        &[crate_without_lib],
        &workspace,
        &FluentParseMode::default(),
        rx,
        always_quit,
        None,
    );

    assert!(result.is_ok());
}

#[test]
fn configure_file_watcher_reports_invalid_watch_roots() {
    let temp = tempfile::tempdir().expect("tempdir");
    let krate = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("broken-watch-root")
            .expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(
            temp.path().join("missing-manifest"),
        ),
        src_dir: crate::core::SourceDir::from_discovered(temp.path().join("missing-src")),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(
            temp.path().join("i18n.toml"),
        ),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
            temp.path().join("i18n/en"),
        ),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };

    let err = super::configure_file_watcher(&[&krate], temp.path(), &temp.path().join("target"))
        .expect_err("missing watch roots should fail watcher setup");
    assert!(err.to_string().contains("Failed to watch"));
}

#[test]
fn configure_file_watcher_reports_invalid_workspace_watch_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("missing-workspace-root");
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir).expect("create src dir");
    let krate = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("broken-workspace-watch-root")
            .expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(temp.path().to_path_buf()),
        src_dir: crate::core::SourceDir::from_discovered(src_dir),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(
            temp.path().join("i18n.toml"),
        ),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
            temp.path().join("i18n/en"),
        ),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };

    let err =
        super::configure_file_watcher(&[&krate], &workspace_root, &workspace_root.join("target"))
            .expect_err("invalid workspace root should fail watcher setup");
    assert!(err.to_string().contains("Failed to watch"));
}

#[test]
fn configure_file_watcher_reports_invalid_manifest_watch_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir).expect("create src dir");
    let krate = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("broken-manifest-watch-root")
            .expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(
            temp.path().join("missing-manifest"),
        ),
        src_dir: crate::core::SourceDir::from_discovered(src_dir),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(
            temp.path().join("i18n.toml"),
        ),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
            temp.path().join("i18n/en"),
        ),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };

    let err = super::configure_file_watcher(&[&krate], temp.path(), &temp.path().join("target"))
        .expect_err("missing manifest watch root should fail watcher setup");
    assert!(err.to_string().contains("Failed to watch"));
}

#[test]
fn update_custom_build_watches_tolerates_an_already_removed_watch() {
    let (file_tx, _file_rx) = crossbeam_channel::unbounded();
    let mut debouncer =
        notify_debouncer_full::new_debouncer(Duration::from_millis(10), None, file_tx)
            .expect("create debouncer");
    let update = super::runtime::BuildSourceWatchUpdate {
        removed: vec![PathBuf::from("/missing/es-fluent-build-watch")],
        ..Default::default()
    };

    super::update_custom_build_watches(&mut debouncer, update)
        .expect("an OS-removed watch should not stop watch mode");
}

#[test]
fn run_watch_loop_with_file_rx_handles_file_events_from_channel() {
    let crate_without_lib = test_crate("no-lib", false);
    let workspace = WorkspaceInfo {
        root_dir: PathBuf::from("/tmp/ws"),
        target_dir: PathBuf::from("/tmp/ws/target"),
        crates: vec![crate_without_lib.clone()],
    };
    let (tx, rx) = crossbeam_channel::unbounded();
    tx.send(Ok(vec![event_with_path(
        &crate_without_lib.src_dir.join("lib.rs"),
    )]))
    .expect("send file event");
    drop(tx);

    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).expect("create terminal");
    let result = super::run_watch_loop_with_file_rx(
        &mut terminal,
        &[crate_without_lib],
        &workspace,
        &FluentParseMode::default(),
        rx,
        never_quit,
        Some(2),
    );

    assert!(result.is_ok());
}

fn quit_after_event_window(_timeout: Duration) -> std::io::Result<bool> {
    static POLL_COUNT: AtomicUsize = AtomicUsize::new(0);
    let count = POLL_COUNT.fetch_add(1, Ordering::SeqCst);
    Ok(count >= 80)
}

#[test]
fn run_watch_loop_with_poll_processes_file_change_events() {
    let (_temp, workspace, krate) = create_valid_workspace_with_fake_runner();
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).expect("create terminal");

    let src_file = krate.src_dir.join("lib.rs");
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(350));
        let _ = fs::write(&src_file, "pub struct DemoChanged;\n");
    });

    let result = super::run_watch_loop_with_poll(
        &mut terminal,
        std::slice::from_ref(&krate),
        &workspace,
        &FluentParseMode::default(),
        quit_after_event_window,
        Some(120),
    );

    assert!(result.is_ok());
}

#[test]
fn run_watch_loop_with_poll_respects_zero_iteration_limit() {
    let (_temp, workspace, krate) = create_valid_workspace_with_fake_runner();
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).expect("create terminal");

    let result = super::run_watch_loop_with_poll(
        &mut terminal,
        &[krate],
        &workspace,
        &FluentParseMode::default(),
        always_quit,
        Some(0),
    );

    assert!(result.is_ok());
}

#[test]
fn watch_all_propagates_runner_preparation_errors() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace-root-file");
    fs::write(&workspace_root, "not-a-directory").expect("write workspace root sentinel");

    let krate = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("broken-watch").expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(temp.path().to_path_buf()),
        src_dir: crate::core::SourceDir::from_discovered(temp.path().join("src")),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(
            temp.path().join("i18n.toml"),
        ),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
            temp.path().join("i18n/en"),
        ),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };
    let workspace = WorkspaceInfo {
        root_dir: workspace_root,
        target_dir: temp.path().join("target"),
        crates: vec![krate.clone()],
    };

    let err = super::watch_all(&[krate], &workspace, &FluentParseMode::default())
        .expect_err("invalid workspace root should fail before entering the TUI loop");
    let error = err.to_string();
    assert!(
        error.contains("Failed to inspect .es-fluent path")
            || error.contains("Failed to create .es-fluent directory"),
        "unexpected watch setup error: {err}"
    );
}

#[test]
fn watch_all_uses_test_terminal_for_valid_workspace() {
    let (_temp, workspace, krate) = create_valid_workspace_with_fake_runner();

    let result = super::watch_all(&[krate], &workspace, &FluentParseMode::default());

    assert!(result.is_ok());
}

#[test]
fn watch_all_links_only_watched_crates() {
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
        fs::write(
            src_dir.join("lib.rs"),
            if name == "a" {
                "pub fn marker() {}\n"
            } else {
                "this is not rust\n"
            },
        )
        .expect("write lib");
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
            name: es_fluent_runner::PackageName::try_new(name).expect("valid package name"),
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
    let watched_crate = workspace.crates[0].clone();

    let temp_store = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(temp.path());
    let binary_path =
        crate::test_fixtures::fake_runner_binary_path_for_workspace(&workspace.root_dir);
    let mut crate_hashes = indexmap::IndexMap::new();
    crate_hashes.insert(
        watched_crate.name.clone(),
        crate::generation::cache::compute_crate_inputs_hash(
            &watched_crate.manifest_dir,
            &watched_crate.src_dir,
            Some(&watched_crate.i18n_config_path),
            watched_crate.custom_build_target_path.as_deref(),
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

    let result = super::watch_all(
        std::slice::from_ref(&watched_crate),
        &workspace,
        &FluentParseMode::default(),
    );

    assert!(result.is_ok());

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
        "watch runner should not link unwatched crates: {dependencies:?}"
    );
}

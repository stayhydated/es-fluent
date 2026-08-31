use super::*;

#[test]
fn spawn_generation_sends_failure_for_missing_lib_rs() {
    let krate = test_crate("missing-lib", false);
    let workspace = WorkspaceInfo {
        root_dir: PathBuf::from("/tmp/ws"),
        target_dir: PathBuf::from("/tmp/ws/target"),
        crates: vec![krate.clone()],
    };

    let (tx, rx) = crossbeam_channel::unbounded();
    let handle = super::generation::spawn_generation(
        krate,
        Arc::new(workspace),
        FluentParseMode::default(),
        tx,
    );

    let result = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("generation thread should send result");
    handle.join().expect("generation thread should finish");
    assert_eq!(result.name, "missing-lib");
    assert!(result.error.is_some());
}

#[test]
fn spawn_generation_sends_success_and_reads_changed_from_result_json() {
    let (_temp, workspace, krate) = create_valid_workspace_with_fake_runner();
    let temp_store = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(&workspace.root_dir);
    let result_json = temp_store.result_path(&krate.name);
    fs::create_dir_all(result_json.parent().unwrap()).expect("create result dir");
    fs::write(
        &result_json,
        serde_json::to_string(&serde_json::json!({ "changed": true }))
            .expect("serialize result json"),
    )
    .expect("write result json");

    let (tx, rx) = crossbeam_channel::unbounded();
    let handle = super::generation::spawn_generation(
        krate,
        Arc::new(workspace),
        FluentParseMode::default(),
        tx,
    );
    let result = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("generation result");
    handle.join().expect("generation thread should finish");

    assert!(
        result.error.is_none(),
        "unexpected error: {:?}",
        result.error
    );
    assert!(result.changed);
    assert!(
        result
            .output
            .as_deref()
            .is_some_and(|out| out.contains("watcher-run"))
    );
}

#[test]
fn spawn_generation_handles_invalid_json_and_empty_output() {
    let (_temp, workspace, krate) =
        create_valid_workspace_with_fake_runner_behavior(FakeRunnerBehavior::silent_success());
    let temp_store = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(&workspace.root_dir);
    let result_json = temp_store.result_path(&krate.name);
    fs::create_dir_all(result_json.parent().unwrap()).expect("create result dir");
    fs::write(&result_json, "{not-json").expect("write invalid json");

    let (tx, rx) = crossbeam_channel::unbounded();
    let handle = super::generation::spawn_generation(
        krate,
        Arc::new(workspace),
        FluentParseMode::default(),
        tx,
    );
    let result = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("generation result");
    handle.join().expect("generation thread should finish");

    assert!(
        result.error.is_none(),
        "unexpected error: {:?}",
        result.error
    );
    assert!(!result.changed);
    assert!(result.output.is_none(), "empty output should map to None");
}

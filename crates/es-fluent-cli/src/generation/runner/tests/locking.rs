use super::*;

#[test]
fn monolithic_runner_lock_serializes_shared_runner_access() {
    let temp = tempfile::tempdir().expect("tempdir");
    let first = acquire_monolithic_runner_lock(temp.path()).expect("acquire first lock");
    let lock_path = temp.path().join(".es-fluent/.runner-lock");
    assert!(lock_path.is_file(), "runner lock should be a regular file");
    let root = temp.path().to_path_buf();
    let (started_tx, started_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();

    let handle = thread::spawn(move || {
        started_tx.send(()).expect("send started");
        let _second = acquire_monolithic_runner_lock(&root).expect("acquire second lock");
        done_tx.send(()).expect("send done");
    });

    started_rx.recv().expect("second thread started");
    assert!(
        done_rx.recv_timeout(Duration::from_millis(150)).is_err(),
        "second lock should wait while first lock is held"
    );

    drop(first);
    done_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("second lock should acquire after first lock is dropped");
    handle.join().expect("join lock thread");
    assert!(
        lock_path.is_file(),
        "persistent lock file should not block reacquisition"
    );
    let _third =
        acquire_monolithic_runner_lock(temp.path()).expect("reacquire persistent lock file");
}

#[test]
fn monolithic_runner_lock_rejects_non_file_lock_path_without_waiting() {
    let temp = tempfile::tempdir().expect("tempdir");
    let lock_path = temp.path().join(".es-fluent/.runner-lock");
    fs::create_dir_all(&lock_path).expect("create invalid lock directory");

    let error = acquire_monolithic_runner_lock(temp.path())
        .err()
        .expect("directory lock path should fail");

    assert!(error.to_string().contains("not a regular file"));
    assert!(
        error
            .to_string()
            .contains("confirming no es-fluent command is running")
    );
}

#[test]
fn monolithic_runner_lock_is_released_when_owner_exits_without_drop() {
    let temp = tempfile::tempdir().expect("tempdir");
    let status = std::process::Command::new(std::env::current_exe().expect("current test binary"))
        .args([
            "--ignored",
            "--exact",
            "generation::runner::tests::monolithic_runner_lock_exit_without_drop_helper",
        ])
        .env("ES_FLUENT_LOCK_EXIT_TEST_ROOT", temp.path())
        .status()
        .expect("run lock owner subprocess");

    assert!(
        status.success(),
        "lock owner subprocess should exit cleanly"
    );
    assert!(
        temp.path().join(".es-fluent/.runner-lock").is_file(),
        "the persistent lock file should remain after owner exit"
    );
    let _lock = acquire_monolithic_runner_lock(temp.path())
        .expect("OS should release the lock when its owner exits");
}

use super::*;
use fs_err as fs;

#[test]
fn run_watch_returns_ok_when_package_filter_matches_nothing() {
    let temp = crate::test_fixtures::create_test_crate_workspace_without_ftl();

    let result = run_watch(WatchArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: Some("missing-crate".to_string()),
        },
        mode: FluentParseMode::default(),
    });

    assert!(result.is_ok());
}

#[test]
fn run_watch_returns_error_for_invalid_path() {
    let result = run_watch(WatchArgs {
        workspace: WorkspaceArgs {
            path: Some(std::path::PathBuf::from("/definitely/missing/path")),
            package: None,
        },
        mode: FluentParseMode::default(),
    });

    assert!(result.is_err());
}

#[test]
fn run_watch_propagates_watch_all_setup_error_for_discovered_crate() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    let metadata_path = temp.path().join(".es-fluent");
    if metadata_path.is_dir() {
        fs::remove_dir_all(&metadata_path).expect("remove metadata dir");
    }
    fs::write(&metadata_path, "not a directory").expect("write metadata sentinel");

    let result = run_watch(WatchArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        mode: FluentParseMode::default(),
    });

    assert!(result.is_err());
}

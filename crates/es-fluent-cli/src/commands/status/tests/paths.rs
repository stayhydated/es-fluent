use super::*;

#[test]
fn relative_status_message_strips_workspace_paths_from_setup_errors() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    let message = format!(
        "test-app: locale path 'fr' is not a directory: {}",
        temp.path().join("i18n/fr").display()
    );

    let normalized = relative_status_message(&message, temp.path());

    assert_eq!(
        normalized,
        "test-app: locale path 'fr' is not a directory: i18n/fr"
    );
}

#[test]
fn run_status_setup_errors_use_workspace_relative_paths() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale path file");

    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: None,
    })
    .expect("discover workspace");
    let setup_errors = normalize_status_setup_errors(
        collect_status_setup_errors(&workspace),
        &workspace.workspace_info.root_dir,
    );

    assert!(
        setup_errors
            .iter()
            .any(|error| { error == "test-app: locale path 'fr' is not a directory: i18n/fr" }),
        "status setup errors should use workspace-relative paths: {setup_errors:?}"
    );
    assert!(
        setup_errors
            .iter()
            .all(|error| !error.contains(temp.path().to_string_lossy().as_ref())),
        "status setup errors should not include absolute temp paths: {setup_errors:?}"
    );
}

#[test]
fn run_status_ftl_layout_setup_errors_use_workspace_relative_paths() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    fs::remove_file(temp.path().join("i18n/en/test-app.ftl")).expect("remove fallback ftl");
    fs::create_dir_all(temp.path().join("i18n/en/test-app.ftl"))
        .expect("create ftl path directory");

    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: None,
    })
    .expect("discover workspace");
    let setup_errors = normalize_status_setup_errors(
        collect_status_setup_errors(&workspace),
        &workspace.workspace_info.root_dir,
    );

    assert!(
        setup_errors.iter().any(|error| {
            error.contains("Expected FTL path to be a file")
                && error.contains("i18n/en/test-app.ftl")
        }),
        "status FTL layout setup errors should include relative FTL paths: {setup_errors:?}"
    );
    assert!(
        setup_errors
            .iter()
            .all(|error| !error.contains(temp.path().to_string_lossy().as_ref())),
        "status FTL layout setup errors should not include absolute temp paths: {setup_errors:?}"
    );
}

#[test]
fn collect_status_setup_errors_deduplicates_fallback_locale_path_file() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");
    fs::write(temp.path().join("i18n/en"), "not a directory\n").expect("write fallback file");
    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: None,
    })
    .expect("discover workspace");

    let errors = collect_status_setup_errors(&workspace);

    assert_eq!(
        errors
            .iter()
            .filter(|error| error
                .contains("fallback locale directory 'en' is missing or not a directory"))
            .count(),
        1
    );
    assert!(
        !errors
            .iter()
            .any(|error| error.contains("locale path 'en' is not a directory"))
    );
}

#[cfg(unix)]
#[test]
fn collect_status_setup_errors_deduplicates_fallback_locale_path_symlink() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    let outside = tempfile::tempdir().expect("outside tempdir");
    fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");
    fs::create_dir_all(outside.path().join("en")).expect("create outside locale");
    std::os::unix::fs::symlink(outside.path().join("en"), temp.path().join("i18n/en"))
        .expect("create fallback locale symlink");
    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: None,
    })
    .expect("discover workspace");

    let errors = collect_status_setup_errors(&workspace);

    assert_eq!(
        errors
            .iter()
            .filter(|error| error
                .contains("fallback locale directory 'en' is missing or not a directory"))
            .count(),
        1
    );
    assert!(
        !errors
            .iter()
            .any(|error| error.contains("locale path 'en' is not a directory"))
    );
}

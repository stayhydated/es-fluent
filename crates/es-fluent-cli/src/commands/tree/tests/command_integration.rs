use super::*;

#[test]
fn run_tree_errors_for_missing_package_filter() {
    let temp = create_workspace_with_tree_data();
    let result = run_tree(TreeArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: Some("missing-package".to_string()),
        },
        all_locales: false,
        attributes: false,
        variables: false,
        link_mode: None,
        output: OutputFormat::Text,
    });
    assert!(matches!(result, Err(CliError::Exit(1))));
}

#[test]
fn collect_rust_link_indexes_rejects_ftl_layout_before_runner_setup() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    let ftl_path = temp.path().join("i18n/en/test-app.ftl");
    fs::remove_file(&ftl_path).expect("remove ftl file");
    fs::create_dir(&ftl_path).expect("create ftl directory");
    fs::write(temp.path().join("src/lib.rs"), "this is not rust\n").expect("break Rust");
    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: None,
    })
    .expect("discover workspace");

    let error = collect_rust_link_indexes(&workspace, TreeLinkMode::Rust, true, false)
        .expect_err("FTL layout should be rejected before Rust link collection");

    assert!(error.to_string().contains("Expected FTL path"));
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "tree should reject invalid FTL paths before runner metadata"
    );
    assert!(
        !temp.path().join("target").exists(),
        "tree should reject invalid FTL paths before Cargo runs"
    );
}

#[test]
fn collect_rust_link_indexes_rejects_parse_errors_before_runner_setup() {
    let temp = create_workspace_with_tree_data();
    fs::write(
        temp.path().join("i18n/en/test-app.ftl"),
        "hello = { $name\n",
    )
    .expect("write invalid FTL");
    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: None,
    })
    .expect("discover workspace");

    let error = collect_rust_link_indexes(&workspace, TreeLinkMode::Rust, true, false)
        .expect_err("FTL parse error should be rejected before Rust link collection");

    assert!(error.to_string().contains("failed to parse FTL file"));
    assert!(error.to_string().contains("Fluent parse errors"));
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "parse errors should be reported before runner metadata"
    );
    assert!(
        !temp.path().join("target").exists(),
        "parse errors should be reported before Cargo runs"
    );
}

#[test]
fn build_file_tree_json_reports_messages_terms_variables_and_parse_errors() {
    let temp = create_workspace_with_tree_data();
    let (valid, valid_error) = build_file_tree_json(
        "test-app.ftl",
        &temp.path().join("i18n/en/test-app.ftl"),
        true,
        true,
    );

    assert!(valid_error.is_none());
    assert!(!valid.parse_error);
    assert_eq!(valid.path, "test-app.ftl");
    assert!(valid.entries.iter().any(|entry| {
        entry.id == "hello" && entry.kind == "message" && entry.variables == ["name"]
    }));
    assert!(
        valid
            .entries
            .iter()
            .any(|entry| { entry.id == "-term" && entry.kind == "term" })
    );

    let invalid = temp.path().join("i18n/en/broken.ftl");
    fs::write(&invalid, "broken = {").expect("write invalid ftl");
    let (broken, broken_error) = build_file_tree_json("broken.ftl", &invalid, true, true);
    assert!(broken.parse_error);
    assert!(broken.entries.is_empty());
    assert!(
        broken_error
            .as_deref()
            .is_some_and(|error| error.contains("Fluent parse errors"))
    );
}

#[test]
fn build_file_tree_json_honors_attribute_and_variable_filters() {
    let temp = create_workspace_with_tree_data();
    fs::write(
        temp.path().join("i18n/en/test-app.ftl"),
        "hello = Hello { $name }\n    .title = Title { $title }\n-term = Term Value\n",
    )
    .expect("write ftl with distinct value and attribute variables");

    let (hidden, hidden_error) = build_file_tree_json(
        "test-app.ftl",
        &temp.path().join("i18n/en/test-app.ftl"),
        false,
        false,
    );

    assert!(hidden_error.is_none());
    let hello = hidden
        .entries
        .iter()
        .find(|entry| entry.id == "hello")
        .expect("hello entry");
    assert!(hello.attributes.is_empty());
    assert!(hello.variables.is_empty());

    let (shown, shown_error) = build_file_tree_json(
        "test-app.ftl",
        &temp.path().join("i18n/en/test-app.ftl"),
        true,
        true,
    );
    assert!(shown_error.is_none());
    let hello = shown
        .entries
        .iter()
        .find(|entry| entry.id == "hello")
        .expect("hello entry");
    assert_eq!(hello.attributes, ["title"]);
    assert_eq!(hello.variables, ["name", "title"]);

    let (hidden_attributes, hidden_attributes_error) = build_file_tree_json(
        "test-app.ftl",
        &temp.path().join("i18n/en/test-app.ftl"),
        false,
        true,
    );
    assert!(hidden_attributes_error.is_none());
    let hello = hidden_attributes
        .entries
        .iter()
        .find(|entry| entry.id == "hello")
        .expect("hello entry");
    assert!(hello.attributes.is_empty());
    assert_eq!(hello.variables, ["name"]);
}

#[test]
fn build_crate_tree_json_collects_locale_files_and_skips_missing_locales() {
    let temp = create_workspace_with_tree_data();
    fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
    fs::create_dir_all(temp.path().join("i18n/en/unrelated")).expect("create unrelated dir");
    fs::write(temp.path().join("i18n/en/other.ftl"), "other = Other\n")
        .expect("write unrelated main ftl");
    fs::write(
        temp.path().join("i18n/en/unrelated/nested.ftl"),
        "other-nested = Other nested\n",
    )
    .expect("write unrelated nested ftl");
    let krate = crate_info_from_temp(&temp);

    let (json, parse_errors) =
        build_crate_tree_json(&krate, true, true, true).expect("tree json should build");

    assert!(parse_errors.is_empty());
    assert_eq!(json.name, "test-app");
    assert!(json.locales.iter().any(|locale| locale.locale == "en"));
    assert!(
        json.locales
            .iter()
            .any(|locale| { locale.locale == "fr" && locale.files.is_empty() })
    );
    assert!(json.locales.iter().any(|locale| {
        locale
            .files
            .iter()
            .any(|file| file.path.contains("test-app.ftl"))
    }));
    let paths = json
        .locales
        .iter()
        .flat_map(|locale| locale.files.iter().map(|file| file.path.as_str()))
        .collect::<Vec<_>>();
    assert!(
        !paths.contains(&"other.ftl"),
        "tree should ignore FTL files outside the crate layout"
    );
    assert!(
        !paths.contains(&"unrelated/nested.ftl"),
        "tree should ignore nested FTL files outside the crate layout"
    );
}

#[test]
fn build_crate_tree_json_errors_when_fallback_locale_path_is_file() {
    let temp = create_workspace_with_tree_data();
    fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");
    fs::write(temp.path().join("i18n/en"), "not a directory\n").expect("write fallback file");
    let krate = crate_info_from_temp(&temp);

    let error = build_crate_tree_json(&krate, false, true, true)
        .err()
        .expect("fallback locale path as file should fail");

    assert!(error.to_string().contains("locale directory 'en'"));
    assert!(error.to_string().contains("not a directory"));
}

#[test]
fn build_crate_tree_json_all_errors_when_fallback_locale_is_missing() {
    let temp = create_workspace_with_tree_data();
    fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");
    fs::create_dir_all(temp.path().join("i18n/fr")).expect("create non-fallback locale");
    fs::write(
        temp.path().join("i18n/fr/test-app.ftl"),
        "hello = Bonjour\n",
    )
    .expect("write non-fallback ftl");
    let krate = crate_info_from_temp(&temp);

    let error = build_crate_tree_json(&krate, true, true, true)
        .err()
        .expect("missing fallback locale should fail tree --all-locales");

    assert!(error.to_string().contains("locale directory 'en'"));
    assert!(error.to_string().contains("missing or not a directory"));
}

#[test]
fn build_crate_tree_json_errors_when_assets_dir_path_is_file() {
    let temp = create_workspace_with_tree_data();
    fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");
    let krate = crate_info_from_temp(&temp);

    let error = build_crate_tree_json(&krate, false, true, true)
        .err()
        .expect("assets_dir path as file should fail");

    assert!(error.to_string().contains("assets_dir"));
    assert!(error.to_string().contains("not a directory"));
}

#[test]
fn build_crate_tree_json_all_errors_when_locale_named_asset_path_is_file() {
    let temp = create_workspace_with_tree_data();
    fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");
    let krate = crate_info_from_temp(&temp);

    let error = build_crate_tree_json(&krate, true, true, true)
        .err()
        .expect("locale path as file should fail");

    assert!(error.to_string().contains("locale directory 'fr'"));
    assert!(error.to_string().contains("not a directory"));
}

#[test]
fn relative_tree_message_strips_workspace_paths_from_json_errors() {
    let temp = create_workspace_with_tree_data();
    let message = format!(
        "locale directory 'fr' is missing or not a directory: {}",
        temp.path().join("i18n/fr").display()
    );

    let normalized = relative_tree_message(&message, temp.path());

    assert_eq!(
        normalized,
        "locale directory 'fr' is missing or not a directory: i18n/fr"
    );
}

#[test]
fn run_tree_json_errors_use_workspace_relative_paths() {
    let temp = create_workspace_with_tree_data();
    fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");
    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: None,
    })
    .expect("discover workspace");
    let krate = &workspace.crates[0];
    let error = match build_crate_tree_json(krate, true, true, true) {
        Ok(_) => panic!("locale path file should fail tree JSON"),
        Err(error) => error,
    };
    let message = relative_tree_message(&error.to_string(), &workspace.workspace_info.root_dir);

    assert!(message.contains("locale directory 'fr'"));
    assert!(message.contains("i18n/fr"));
    assert!(
        !message.contains(temp.path().to_string_lossy().as_ref()),
        "tree JSON errors should not include absolute temp paths: {message}"
    );
}

#[test]
fn run_tree_covers_json_and_text_command_paths() {
    let temp = create_workspace_with_tree_data();

    let json = run_tree(TreeArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        all_locales: true,
        attributes: true,
        variables: true,
        link_mode: None,
        output: OutputFormat::Json,
    });
    assert!(json.is_ok());

    let text = run_tree(TreeArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        all_locales: false,
        attributes: true,
        variables: true,
        link_mode: Some("ftl".to_string()),
        output: OutputFormat::Text,
    });
    assert!(text.is_ok());
}

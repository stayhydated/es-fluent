use super::*;
use std::fs;

#[test]
fn dependency_specs_support_aliases_and_manager_features() {
    let manifest: toml::Value = toml::from_str(
        r#"
[dependencies]
i18n = { package = "es-fluent", version = "0.1" }
manager = { package = "es-fluent-manager-dioxus", version = "0.1", features = ["client"] }

[build-dependencies]
build-i18n = { package = "es-fluent-build", version = "0.1" }
"#,
    )
    .expect("manifest");
    let normal = dependency_specs(&manifest, "dependencies", None);
    let build = dependency_specs(&manifest, "build-dependencies", None);

    assert!(normal.iter().any(|dependency| {
        dependency.alias == "i18n"
            && dependency.package == "es-fluent"
            && dependency.features.is_empty()
    }));
    assert!(normal.iter().any(|dependency| {
        dependency.alias == "manager"
            && dependency.package == "es-fluent-manager-dioxus"
            && dependency.features == ["client".to_string()]
    }));
    assert!(build.iter().any(|dependency| {
        dependency.alias == "build-i18n" && dependency.package == "es-fluent-build"
    }));
}

#[test]
fn dependency_specs_merge_workspace_dependency_features() {
    let workspace: toml::Value = toml::from_str(
        r#"
[workspace.dependencies]
manager = { package = "es-fluent-manager-dioxus", version = "0.7", features = ["client"] }
"#,
    )
    .expect("workspace manifest");
    let package: toml::Value = toml::from_str(
        r#"
[dependencies]
manager = { workspace = true, features = ["ssr"] }
"#,
    )
    .expect("package manifest");
    let workspace_dependencies = workspace["workspace"]["dependencies"]
        .as_table()
        .expect("workspace dependencies");
    let dependencies = dependency_specs(&package, "dependencies", Some(workspace_dependencies));

    assert_eq!(dependencies.len(), 1);
    assert_eq!(dependencies[0].package, "es-fluent-manager-dioxus");
    assert_eq!(dependencies[0].features, ["client", "ssr"]);
}

#[test]
fn doctor_report_is_unhealthy_only_for_errors() {
    let warning = DoctorCheck {
        package: "app".to_string(),
        category: "manager",
        status: DoctorStatus::Warning,
        message: "custom manager".to_string(),
        help: None,
    };
    let report = DoctorReport::new(1, Vec::new(), vec![warning]);
    assert!(report.healthy);
    assert_eq!(report.warning_count, 1);

    let report = DoctorReport::new(0, vec!["missing config".to_string()], Vec::new());
    assert!(!report.healthy);
    assert_eq!(report.error_count, 1);
}

#[test]
fn fallback_catalog_inputs_ignore_crate_root_project_directories() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("en")).expect("create locale");
    fs::create_dir_all(temp.path().join("src")).expect("create src");
    fs::create_dir_all(temp.path().join("target")).expect("create target");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \".\"\n",
    )
    .expect("write config");
    fs::write(temp.path().join("en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback resource");
    let layout =
        ResolvedI18nLayout::from_config_path(temp.path().join("i18n.toml")).expect("layout");

    assert_eq!(
        fallback_catalog_inputs(&layout, "test-app").expect("catalog"),
        1
    );
}

#[test]
fn fallback_catalog_inputs_recognize_normalized_crate_root_assets() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir(temp.path().join("locale")).expect("create normalized path component");
    fs::create_dir(temp.path().join("en")).expect("create locale");
    fs::create_dir(temp.path().join("src")).expect("create src");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"locale/..\"\n",
    )
    .expect("write config");
    fs::write(temp.path().join("en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback resource");
    let layout =
        ResolvedI18nLayout::from_config_path(temp.path().join("i18n.toml")).expect("layout");

    assert_eq!(
        fallback_catalog_inputs(&layout, "test-app").expect("catalog"),
        1
    );
}

#[cfg(unix)]
#[test]
fn fallback_catalog_inputs_reject_symlinked_fallback_resource() {
    let temp = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside tempdir");
    fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    let outside_resource = outside.path().join("test-app.ftl");
    fs::write(&outside_resource, "hello = Outside\n").expect("write outside resource");
    std::os::unix::fs::symlink(&outside_resource, temp.path().join("i18n/en/test-app.ftl"))
        .expect("create fallback resource symlink");
    let layout =
        ResolvedI18nLayout::from_config_path(temp.path().join("i18n.toml")).expect("layout");

    let error = fallback_catalog_inputs(&layout, "test-app")
        .expect_err("symlinked fallback resources should fail doctor validation");
    assert!(error.contains("Fluent resource must be a real file, not a symlink"));
}

#[cfg(unix)]
#[test]
fn fallback_catalog_inputs_reject_symlinked_non_fallback_locale() {
    let temp = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside tempdir");
    fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    fs::create_dir_all(outside.path().join("fr")).expect("create external locale");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback resource");
    std::os::unix::fs::symlink(outside.path().join("fr"), temp.path().join("i18n/fr"))
        .expect("create non-fallback locale symlink");
    let layout =
        ResolvedI18nLayout::from_config_path(temp.path().join("i18n.toml")).expect("layout");

    let error = fallback_catalog_inputs(&layout, "test-app")
        .expect_err("symlinked non-fallback locales should fail doctor validation");
    assert!(error.contains("locale asset entries must not be symlinks"));
    assert!(error.contains("i18n/fr"));
}

#[test]
fn fallback_catalog_inputs_reject_invalid_namespace_in_non_fallback_root_locale() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("en")).expect("create fallback locale");
    fs::create_dir_all(temp.path().join("fr/test-app")).expect("create namespace dir");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \".\"\n",
    )
    .expect("write config");
    fs::write(temp.path().join("en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback resource");
    fs::write(
        temp.path().join("fr/test-app/ bad .ftl"),
        "hello = Bonjour\n",
    )
    .expect("write translated resource");
    let layout =
        ResolvedI18nLayout::from_config_path(temp.path().join("i18n.toml")).expect("layout");

    let error = fallback_catalog_inputs(&layout, "test-app")
        .expect_err("invalid namespace should fail doctor catalog validation");
    assert!(error.contains("discovered invalid namespace ' bad '"));
    assert!(error.contains("leading or trailing whitespace"));
}

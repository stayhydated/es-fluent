use super::*;
use crate::commands::common::WorkspaceArgs;
use fs_err as fs;

#[test]
fn run_doctor_succeeds_for_basic_workspace() {
    let temp = crate::test_fixtures::create_test_crate_workspace();

    let result = run_doctor(DoctorArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        output: OutputFormat::Text,
    });

    assert!(result.is_ok());
}

#[test]
fn run_doctor_fails_when_fallback_locale_directory_is_missing() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");

    let result = run_doctor(DoctorArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        output: OutputFormat::Text,
    });

    assert!(matches!(result, Err(CliError::Exit(1))));
}

#[test]
fn run_doctor_warns_for_manager_dependency_mismatch_without_failing() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    fs::write(
        temp.path().join("src/i18n.rs"),
        "es_fluent_manager_embedded::define_i18n_module!();\n",
    )
    .expect("write i18n module without matching manifest dependency");

    let result = run_doctor(DoctorArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        output: OutputFormat::Text,
    });

    assert!(result.is_ok(), "warnings should not fail doctor");
}

#[test]
fn doctor_helpers_cover_dependency_detection_and_build_script_warnings() {
    assert_eq!(
        manager_dependency_from_module("es_fluent_manager_embedded::define_i18n_module!();"),
        Some("es-fluent-manager-embedded")
    );
    assert_eq!(
        manager_dependency_from_module("es_fluent_manager_dioxus::define_i18n_module!();"),
        Some("es-fluent-manager-dioxus")
    );
    assert_eq!(
        manager_dependency_from_module("es_fluent_manager_bevy::define_i18n_module!();"),
        Some("es-fluent-manager-bevy")
    );
    assert_eq!(manager_dependency_from_module("no manager"), None);

    let temp = crate::test_fixtures::create_test_crate_workspace();
    let krate = CrateInfo {
        name: "test-app".to_string(),
        manifest_dir: temp.path().to_path_buf(),
        src_dir: temp.path().join("src"),
        i18n_config_path: temp.path().join("i18n.toml"),
        ftl_output_dir: temp.path().join("i18n/en"),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };
    let mut issues = Vec::new();

    inspect_build_script(&krate, "define_i18n_module!();", &mut issues);
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("build.rs does not track"))
    );

    fs::write(temp.path().join("build.rs"), "fn main() {}\n").expect("write build.rs");
    issues.clear();
    inspect_build_script(&krate, "define_i18n_module!();", &mut issues);
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("does not call"))
    );

    fs::write(
        temp.path().join("build.rs"),
        "fn main() { es_fluent::build::track_i18n_assets(); }\n",
    )
    .expect("write tracked build.rs");
    issues.clear();
    inspect_build_script(&krate, "define_i18n_module!();", &mut issues);
    assert!(issues.is_empty());
    assert!(file_contains(
        &temp.path().join("build.rs"),
        "track_i18n_assets"
    ));
}

#[test]
fn run_doctor_json_reports_empty_workspace_warning() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"empty\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir_all(temp.path().join("src")).expect("create src");
    fs::write(temp.path().join("src/lib.rs"), "pub struct Empty;\n").expect("write lib");

    let result = run_doctor(DoctorArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        output: OutputFormat::Json,
    });

    assert!(result.is_ok());
}

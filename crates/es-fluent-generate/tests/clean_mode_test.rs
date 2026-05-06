#![cfg(target_os = "linux")]

mod common;
mod fixtures;
use fixtures::{COUNTRY_VARIANTS, EMPTY_GROUP_A, ORPHAN_GROUPS};
use insta::assert_snapshot;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn read_ftl(path: &Path) -> String {
    fs::read_to_string(path).expect("read ftl")
}

#[test]
fn test_clean_mode_orphans() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");
    let crate_name = "test_crate";
    let ftl_file_path = i18n_path.join(format!("{}.ftl", crate_name));

    fs::create_dir_all(&i18n_path).unwrap();

    fs::write(&ftl_file_path, ORPHAN_GROUPS).unwrap();

    // 2. Define valid items (only GroupA Key1)
    let key1 = common::variant("Key1", &common::ftl_key("GroupA", "Key1"));
    let group_a = common::enum_type("GroupA", vec![key1]);

    // Run clean
    es_fluent_generate::clean::clean(
        crate_name,
        &i18n_path,
        temp_dir.path(),
        std::slice::from_ref(&group_a),
        false,
    )
    .unwrap();

    let content = read_ftl(&ftl_file_path);
    assert_snapshot!("clean_mode_orphans", content);
}

#[test]
fn test_clean_removes_empty_group_comments_for_valid_groups() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");
    let crate_name = "test_crate";
    let ftl_file_path = i18n_path.join(format!("{}.ftl", crate_name));

    fs::create_dir_all(&i18n_path).unwrap();

    fs::write(&ftl_file_path, EMPTY_GROUP_A).unwrap();

    let group_a = common::enum_type(
        "GroupA",
        vec![common::variant("Key1", &common::ftl_key("GroupA", "Key1"))],
    );
    let group_b = common::enum_type(
        "GroupB",
        vec![common::variant("Key1", &common::ftl_key("GroupB", "Key1"))],
    );
    let items = vec![group_a, group_b];

    es_fluent_generate::clean::clean(crate_name, &i18n_path, temp_dir.path(), &items, false)
        .unwrap();

    let content = read_ftl(&ftl_file_path);
    assert_snapshot!(
        "clean_removes_empty_group_comments_for_valid_groups",
        content
    );
}

#[test]
fn test_clean_preserves_variants_items() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");
    let crate_name = "test_crate";
    let ftl_file_path = i18n_path.join(format!("{}.ftl", crate_name));

    fs::create_dir_all(&i18n_path).unwrap();

    fs::write(&ftl_file_path, COUNTRY_VARIANTS).unwrap();

    let variants = vec![
        common::variant("Canada", &common::ftl_key("CountryLabelVariants", "Canada")),
        common::variant("USA", &common::ftl_key("CountryLabelVariants", "USA")),
    ];
    let group = common::enum_type("CountryLabelVariants", variants);

    es_fluent_generate::clean::clean(
        crate_name,
        &i18n_path,
        temp_dir.path(),
        std::slice::from_ref(&group),
        false,
    )
    .unwrap();

    let content = read_ftl(&ftl_file_path);
    assert_snapshot!("clean_preserves_variants_items", content);
}

#[test]
fn test_clean_writes_namespaced_files() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");
    let crate_name = "test_crate";
    let namespaced_file = i18n_path.join(crate_name).join("ui.ftl");

    fs::create_dir_all(&i18n_path).unwrap();
    fs::create_dir_all(namespaced_file.parent().unwrap()).unwrap();
    fs::write(&namespaced_file, "## Stale\n\nstale-Old = Remove me\n").unwrap();

    let variant = common::variant("Title", &common::ftl_key("Ui", "Title"));
    let item = common::enum_type_with_namespace("Ui", vec![variant], "ui");
    let changed = es_fluent_generate::clean::clean(
        crate_name,
        &i18n_path,
        temp_dir.path(),
        std::slice::from_ref(&item),
        false,
    )
    .unwrap();

    assert!(changed);
    let content = read_ftl(&namespaced_file);
    assert_snapshot!("clean_writes_namespaced_files", content);
}

#[test]
fn test_clean_removes_stale_namespaced_files() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");
    let crate_name = "test_crate";
    let stale_file = i18n_path.join(crate_name).join("ui").join("forms.ftl");
    let active_file = i18n_path.join(crate_name).join("errors.ftl");

    fs::create_dir_all(stale_file.parent().unwrap()).unwrap();
    fs::create_dir_all(active_file.parent().unwrap()).unwrap();
    fs::write(&stale_file, "## Stale\n\nstale-Old = Remove me\n").unwrap();
    fs::write(&active_file, "## Errors\n\nerrors-Missing = Missing\n").unwrap();

    let variant = common::variant("Missing", &common::ftl_key("Errors", "Missing"));
    let item = common::enum_type_with_namespace("Errors", vec![variant], "errors");
    let changed = es_fluent_generate::clean::clean(
        crate_name,
        &i18n_path,
        temp_dir.path(),
        std::slice::from_ref(&item),
        false,
    )
    .unwrap();

    assert!(changed);
    assert!(
        !stale_file.exists(),
        "stale namespace file should be removed"
    );
    assert!(
        active_file.exists(),
        "active namespace file should be retained"
    );
    let content = read_ftl(&active_file);
    assert_snapshot!("clean_removes_stale_namespaced_files_active", content);
}

#[test]
fn test_clean_deletes_stale_main_file_when_all_items_are_namespaced() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");
    let crate_name = "test_crate";
    let stale_main_file = i18n_path.join(format!("{}.ftl", crate_name));
    let active_file = i18n_path.join(crate_name).join("ui.ftl");

    fs::create_dir_all(&i18n_path).unwrap();
    fs::create_dir_all(active_file.parent().unwrap()).unwrap();
    fs::write(&stale_main_file, "## Stale\n\nstale-Old = Remove me\n").unwrap();
    fs::write(&active_file, "## Ui\n\nui-Title = Title\n").unwrap();

    let variant = common::variant("Title", &common::ftl_key("Ui", "Title"));
    let item = common::enum_type_with_namespace("Ui", vec![variant], "ui");
    let changed = es_fluent_generate::clean::clean(
        crate_name,
        &i18n_path,
        temp_dir.path(),
        std::slice::from_ref(&item),
        false,
    )
    .unwrap();

    assert!(changed);
    assert!(
        !stale_main_file.exists(),
        "stale main file should be removed"
    );
    assert!(
        active_file.exists(),
        "active namespace file should be written"
    );
}

#[test]
fn test_clean_dry_run_reports_stale_main_file_without_deleting_it() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");
    let crate_name = "test_crate";
    let stale_main_file = i18n_path.join(format!("{}.ftl", crate_name));
    let active_file = i18n_path.join(crate_name).join("ui.ftl");

    fs::create_dir_all(&i18n_path).unwrap();
    fs::create_dir_all(active_file.parent().unwrap()).unwrap();
    fs::write(&stale_main_file, "## Stale\n\nstale-Old = Remove me\n").unwrap();
    fs::write(&active_file, "## Ui\n\nui-Title = Title\n").unwrap();

    let variant = common::variant("Title", &common::ftl_key("Ui", "Title"));
    let item = common::enum_type_with_namespace("Ui", vec![variant], "ui");
    let changed = es_fluent_generate::clean::clean(
        crate_name,
        &i18n_path,
        temp_dir.path(),
        std::slice::from_ref(&item),
        true,
    )
    .unwrap();

    assert!(changed);
    assert!(
        stale_main_file.exists(),
        "dry-run should not delete stale main file"
    );
}

#[test]
fn test_clean_removes_only_unexpected_namespace_ftl_files() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");
    let crate_name = "test_crate";
    let namespace_root = i18n_path.join(crate_name);
    let stale_file = namespace_root.join("stale.ftl");
    let active_file = namespace_root.join("ui.ftl");
    let ignored_file = namespace_root.join("notes.txt");

    fs::create_dir_all(&namespace_root).unwrap();
    fs::write(&stale_file, "stale-Old = Remove me\n").unwrap();
    fs::write(&active_file, "ui-Title = Title\n").unwrap();
    fs::write(&ignored_file, "not fluent\n").unwrap();

    let variant = common::variant("Title", &common::ftl_key("Ui", "Title"));
    let item = common::enum_type_with_namespace("Ui", vec![variant], "ui");
    let changed = es_fluent_generate::clean::clean(
        crate_name,
        &i18n_path,
        temp_dir.path(),
        std::slice::from_ref(&item),
        false,
    )
    .unwrap();

    assert!(changed);
    assert!(!stale_file.exists(), "unexpected namespace FTL is removed");
    assert!(active_file.exists(), "expected namespace FTL is retained");
    assert!(ignored_file.exists(), "non-FTL files are ignored");
}

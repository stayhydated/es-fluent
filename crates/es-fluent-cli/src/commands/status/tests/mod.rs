use super::*;
use crate::commands::common::WorkspaceArgs;
use crate::core::{GenerateResult, ValidationIssue};
use crate::test_fixtures::FakeRunnerBehavior;
use fs_err as fs;

fn package(name: &str) -> es_fluent_runner::PackageName {
    es_fluent_runner::PackageName::try_new(name).expect("valid package name")
}

fn write_inventory(temp: &tempfile::TempDir, expected_keys: &[&str]) {
    let inventory_path = es_fluent_runner::RunnerMetadataStore::new(temp.path().join(".es-fluent"))
        .inventory_path(&package("test-app"));
    fs::create_dir_all(inventory_path.parent().expect("inventory parent"))
        .expect("create inventory dir");
    let keys = expected_keys
            .iter()
            .map(|key| {
                format!(
                    r#"{{"key":{{"owner":"test-app","domain":"test-app","id":"{key}"}},"variables":[],"source_file":null,"source_line":null}}"#
                )
            })
            .collect::<Vec<_>>()
            .join(",");
    fs::write(&inventory_path, format!(r#"{{"expected_keys":[{keys}]}}"#))
        .expect("write inventory");
}

use super::collectors::relative_status_message;

mod paths;
mod report_collectors;
mod workflows;

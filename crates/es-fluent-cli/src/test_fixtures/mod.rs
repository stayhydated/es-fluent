//! Shared test fixtures for es-fluent-cli tests.
//!
//! This module provides common file-based fixtures that can be reused
//! across different test modules.
#![allow(dead_code)]

#[cfg(test)]
use crate::generation::cache::{RunnerCache, compute_content_hash};
#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::path::Path;
#[cfg(test)]
use std::time::SystemTime;

pub const CARGO_TOML: &str = include_str!("../../tests/fixtures/base/Cargo.toml");
pub const I18N_TOML: &str = include_str!("../../tests/fixtures/base/i18n.toml");
pub const LIB_RS: &str = include_str!("../../tests/fixtures/base/lib.rs");
pub const HELLO_FTL: &str = include_str!("../../tests/fixtures/base/ftl/hello.ftl");
pub const HELLO_ES_FTL: &str = include_str!("../../tests/fixtures/base/ftl/hello_es.ftl");
pub const HELLO_FR_FTL: &str = include_str!("../../tests/fixtures/base/ftl/hello_fr.ftl");
pub const HELLO_WORLD_FTL: &str = include_str!("../../tests/fixtures/base/ftl/hello_world.ftl");
pub const RUNNER_SCRIPT: &str = include_str!("../../tests/fixtures/runner/runner.sh");
pub const RUNNER_OUTPUT_SCRIPT: &str = include_str!("../../tests/fixtures/runner/runner_output.sh");
pub const RUNNER_FAILING_SCRIPT: &str =
    include_str!("../../tests/fixtures/runner/runner_failing.sh");
pub const INVALID_FTL: &str = include_str!("../../tests/fixtures/runner/invalid.ftl");

// Check command specific fixtures
pub const INVENTORY_WITH_HELLO: &str =
    include_str!("../../tests/fixtures/check/inventory_with_hello.json");
pub const INVENTORY_WITH_MISSING_KEY: &str =
    include_str!("../../tests/fixtures/check/inventory_with_missing_key.json");

// Format command specific fixtures
pub const UI_UNSORTED_FTL: &str = include_str!("../../tests/fixtures/format/ui_unsorted.ftl");

// Utils specific fixtures
pub const WORKSPACE_CARGO_TOML: &str = include_str!("../../tests/fixtures/workspace/Cargo.toml");

#[cfg(test)]
pub fn create_test_crate_workspace() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("tempdir");
    write_basic_workspace(temp.path(), true);
    temp
}

#[cfg(test)]
pub fn create_test_crate_workspace_without_ftl() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("tempdir");
    write_basic_workspace(temp.path(), false);
    temp
}

#[cfg(test)]
pub fn write_basic_workspace(base: &Path, include_ftl: bool) {
    fs::create_dir_all(base.join("src")).expect("create src");
    fs::create_dir_all(base.join("i18n/en")).expect("create i18n/en");
    fs::write(base.join("Cargo.toml"), CARGO_TOML).expect("write Cargo.toml");
    fs::write(base.join("src/lib.rs"), LIB_RS).expect("write lib.rs");
    fs::write(base.join("i18n.toml"), I18N_TOML).expect("write i18n.toml");
    if include_ftl {
        fs::write(base.join("i18n/en/test-app.ftl"), HELLO_FTL).expect("write ftl");
    }
}

#[cfg(test)]
pub fn create_workspace_with_locales(locales: &[(&str, &str)]) -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("src")).expect("create src");
    fs::write(temp.path().join("Cargo.toml"), CARGO_TOML).expect("write Cargo.toml");
    fs::write(temp.path().join("src/lib.rs"), LIB_RS).expect("write lib.rs");
    fs::write(temp.path().join("i18n.toml"), I18N_TOML).expect("write i18n.toml");

    for (locale, content) in locales {
        let locale_dir = temp.path().join("i18n").join(locale);
        fs::create_dir_all(&locale_dir).expect("create locale dir");
        fs::write(locale_dir.join("test-app.ftl"), content).expect("write locale ftl");
    }

    temp
}

#[cfg(test)]
#[cfg(unix)]
pub fn set_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("set permissions");
}

#[cfg(test)]
#[cfg(not(unix))]
pub fn set_executable(_path: &Path) {}

#[cfg(test)]
pub fn setup_fake_runner_and_cache(temp: &tempfile::TempDir, script: &str) {
    let binary_path = temp.path().join("target/debug/es-fluent-runner");
    fs::create_dir_all(binary_path.parent().expect("parent")).expect("create target/debug");
    fs::write(&binary_path, script).expect("write runner");
    set_executable(&binary_path);

    let src_dir = temp.path().join("src");
    let i18n_toml = temp.path().join("i18n.toml");
    let hash = compute_content_hash(&src_dir, Some(&i18n_toml));
    let mtime = fs::metadata(&binary_path)
        .and_then(|m| m.modified())
        .expect("runner mtime")
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("mtime duration")
        .as_secs();

    let temp_dir = es_fluent_derive_core::get_es_fluent_temp_dir(temp.path());
    fs::create_dir_all(&temp_dir).expect("create temp dir");
    let mut crate_hashes = indexmap::IndexMap::new();
    crate_hashes.insert("test-app".to_string(), hash);
    RunnerCache {
        crate_hashes,
        runner_mtime: mtime,
        cli_version: env!("CARGO_PKG_VERSION").to_string(),
    }
    .save(&temp_dir)
    .expect("save runner cache");
}

//! Shared test fixtures for es-fluent-cli tests.
//!
//! This module provides common file-based fixtures that can be reused
//! across different test modules.
#![allow(dead_code)]

#[cfg(test)]
use crate::generation::cache::{RunnerCache, compute_content_hash, compute_workspace_inputs_hash};
#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::process::Command;
#[cfg(test)]
use std::sync::OnceLock;
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
#[derive(Clone, Debug, Default)]
pub struct FakeRunnerBehavior {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub echo_args: bool,
}

#[cfg(test)]
impl FakeRunnerBehavior {
    pub fn silent_success() -> Self {
        Self::default()
    }

    pub fn stdout(stdout: impl Into<String>) -> Self {
        Self {
            stdout: stdout.into(),
            ..Self::default()
        }
    }

    pub fn failing(stderr: impl Into<String>) -> Self {
        Self {
            stderr: stderr.into(),
            exit_code: 1,
            ..Self::default()
        }
    }

    pub fn echo_args() -> Self {
        Self {
            echo_args: true,
            ..Self::default()
        }
    }
}

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
pub fn fake_runner_binary_name() -> String {
    format!("es-fluent-runner{}", std::env::consts::EXE_SUFFIX)
}

#[cfg(test)]
pub fn fake_runner_binary_path(target_dir: &Path) -> PathBuf {
    target_dir.join("debug").join(fake_runner_binary_name())
}

#[cfg(test)]
fn compiled_fake_runner_binary() -> &'static PathBuf {
    static BINARY: OnceLock<PathBuf> = OnceLock::new();

    BINARY.get_or_init(|| {
        let cache_dir =
            std::env::temp_dir().join(format!("es-fluent-cli-test-runner-{}", std::process::id()));
        fs::create_dir_all(&cache_dir).expect("create fake runner cache dir");

        let source_path = cache_dir.join("fake_runner.rs");
        let binary_path = cache_dir.join(fake_runner_binary_name());

        fs::write(
            &source_path,
            r#"use std::{env, fs, process};

fn read_sidecar(exe: &std::path::Path, ext: &str) -> Option<String> {
    fs::read_to_string(exe.with_extension(ext)).ok()
}

fn main() {
    let exe = env::current_exe().expect("current_exe");
    let args: Vec<String> = env::args().skip(1).collect();
    let mode = read_sidecar(&exe, "mode").unwrap_or_default();

    if mode.trim() == "echo_args" {
        print!("{}", args.join(" "));
    }

    if let Some(stdout) = read_sidecar(&exe, "stdout") {
        print!("{stdout}");
    }

    if let Some(stderr) = read_sidecar(&exe, "stderr") {
        eprint!("{stderr}");
    }

    let exit_code = read_sidecar(&exe, "exitcode")
        .and_then(|raw| raw.trim().parse::<i32>().ok())
        .unwrap_or(0);
    process::exit(exit_code);
}
"#,
        )
        .expect("write fake runner source");

        let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
        let output = Command::new(rustc)
            .arg("--edition=2021")
            .arg(&source_path)
            .arg("-o")
            .arg(&binary_path)
            .output()
            .expect("spawn rustc for fake runner");

        assert!(
            output.status.success(),
            "failed to compile fake runner: stdout={}; stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        binary_path
    })
}

#[cfg(test)]
pub fn install_fake_runner(binary_path: &Path, behavior: &FakeRunnerBehavior) {
    fs::create_dir_all(binary_path.parent().expect("binary parent")).expect("create target/debug");
    let _ = fs::remove_file(binary_path);
    if fs::hard_link(compiled_fake_runner_binary(), binary_path).is_err() {
        let staged_binary_path = binary_path.with_extension("installing");
        let _ = fs::remove_file(&staged_binary_path);
        fs::copy(compiled_fake_runner_binary(), &staged_binary_path)
            .expect("copy fake runner binary");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut perms = fs::metadata(&staged_binary_path)
                .expect("fake runner metadata")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&staged_binary_path, perms).expect("set fake runner executable");
        }
        fs::rename(&staged_binary_path, binary_path).expect("install fake runner binary");
    }
    fs::write(
        binary_path.with_extension("mode"),
        if behavior.echo_args {
            "echo_args"
        } else {
            "static"
        },
    )
    .expect("write fake runner mode");
    fs::write(binary_path.with_extension("stdout"), &behavior.stdout)
        .expect("write fake runner stdout");
    fs::write(binary_path.with_extension("stderr"), &behavior.stderr)
        .expect("write fake runner stderr");
    fs::write(
        binary_path.with_extension("exitcode"),
        behavior.exit_code.to_string(),
    )
    .expect("write fake runner exit code");
}

#[cfg(test)]
pub fn setup_fake_runner_and_cache(temp: &tempfile::TempDir, behavior: FakeRunnerBehavior) {
    let binary_path = fake_runner_binary_path(&temp.path().join("target"));
    install_fake_runner(&binary_path, &behavior);

    let src_dir = temp.path().join("src");
    let i18n_toml = temp.path().join("i18n.toml");
    let hash = compute_content_hash(&src_dir, Some(&i18n_toml));
    let mtime = fs::metadata(&binary_path)
        .and_then(|m| m.modified())
        .expect("runner mtime")
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("mtime duration")
        .as_secs();

    let temp_store = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(temp.path());
    fs::create_dir_all(temp_store.base_dir()).expect("create temp dir");
    let mut crate_hashes = indexmap::IndexMap::new();
    crate_hashes.insert("test-app".to_string(), hash);
    RunnerCache {
        crate_hashes,
        runner_mtime: mtime,
        cli_version: env!("CARGO_PKG_VERSION").to_string(),
        workspace_inputs_hash: compute_workspace_inputs_hash(temp.path()),
    }
    .save(temp_store.base_dir())
    .expect("save runner cache");
}

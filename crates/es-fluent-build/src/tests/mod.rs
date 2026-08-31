use super::*;
use path_slash::PathExt as _;
use std::fs;
use std::path::Path;
use std::process::Command;

fn with_manifest_env<T>(value: Option<&Path>, f: impl FnOnce() -> T) -> T {
    let out_dir = value.map(|path| path.join("build-output"));
    if let Some(out_dir) = &out_dir {
        fs::create_dir_all(out_dir).expect("create build output");
    }
    temp_env::with_vars(
        [
            ("CARGO_MANIFEST_DIR", value.map(Path::as_os_str)),
            ("CARGO_PKG_NAME", Some(std::ffi::OsStr::new("test-package"))),
            ("OUT_DIR", out_dir.as_deref().map(Path::as_os_str)),
        ],
        f,
    )
}

fn toml_path(path: &Path) -> String {
    path.to_slash_lossy().into_owned()
}

fn cargo_workspace_output(
    workspace_dir: &Path,
    target_dir: &Path,
    args: &[&str],
) -> std::process::Output {
    Command::new("cargo")
        .args(args)
        .current_dir(workspace_dir)
        .env("CARGO_TARGET_DIR", target_dir)
        .output()
        .expect("run cargo workspace command")
}

fn cargo_check_output(crate_dir: &Path, target_dir: &Path, args: &[&str]) -> std::process::Output {
    let mut command = Command::new("cargo");
    command
        .arg("check")
        .arg("--quiet")
        .args(args)
        .current_dir(crate_dir)
        .env("CARGO_TARGET_DIR", target_dir);
    command.output().expect("run cargo check")
}

fn cargo_check_output_with_inventory(
    crate_dir: &Path,
    target_dir: &Path,
    inventory: bool,
) -> std::process::Output {
    let mut command = Command::new("cargo");
    command
        .arg("check")
        .arg("--quiet")
        .current_dir(crate_dir)
        .env("CARGO_TARGET_DIR", target_dir)
        .env("RUSTFLAGS", "-A warnings");
    if inventory {
        command.env(INVENTORY_RUNNER_ENV, "1");
    }
    command.output().expect("run cargo check")
}

fn run_cargo_check(crate_dir: &Path, target_dir: &Path, trace_file: &Path) {
    let status = Command::new("cargo")
        .arg("check")
        .arg("--quiet")
        .current_dir(crate_dir)
        .env("CARGO_TARGET_DIR", target_dir)
        .env("TRACE_FILE", trace_file)
        .status()
        .expect("run cargo check");

    assert!(status.success(), "cargo check should succeed");
}

fn trace_lines(trace_file: &Path) -> usize {
    fs::read_to_string(trace_file)
        .expect("read trace file")
        .lines()
        .count()
}

fn panic_message(panic: &(dyn std::any::Any + Send)) -> Option<&str> {
    if let Some(message) = panic.downcast_ref::<&str>() {
        Some(message)
    } else {
        panic.downcast_ref::<String>().map(String::as_str)
    }
}

const BUILD_TRACK_I18N_SOURCE: &str = r#"fn main() {
    es_fluent_build::track_i18n_assets();
}
"#;

const BUILD_SCRIPT_SOURCE: &str = r#"use std::fs::OpenOptions;
use std::io::Write;

fn main() {
    es_fluent_build::track_i18n_assets();

    let trace_path = std::env::var("TRACE_FILE").expect("TRACE_FILE must be set");
    let mut trace = OpenOptions::new()
        .create(true)
        .append(true)
        .open(trace_path)
        .expect("open trace file");
    writeln!(trace, "ran").expect("write trace line");
}
"#;

include!("catalog.rs");
include!("integration.rs");

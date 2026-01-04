use assert_cmd::Command;
use insta::assert_snapshot;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

const CLI_E2E_DIR: &str = "es-fluent-cli-e2e";

static TEST_DATA_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(CLI_E2E_DIR)
});

fn setup_test_env() -> assert_fs::TempDir {
    let temp = assert_fs::TempDir::new().unwrap();

    // Copy examples-tests content to temp dir
    // We use a simple recursive copy since assert_fs::copy_of might not be available or sufficient
    // for deep copying without extra dependencies like `fs_extra`.
    // Actually, let's use the `fs_extra` crate if available, or just walkdir.
    // Since we don't have fs_extra, let's use a helper.
    copy_dir_recursive(&TEST_DATA_DIR, temp.path()).expect("failed to copy test data");

    fix_cargo_manifests(temp.path());

    temp
}

fn fix_cargo_manifests(root: &Path) {
    let cargo_toml_path = root.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml_path).expect("failed to read Cargo.toml");

    let repo_root = TEST_DATA_DIR.parent().unwrap();
    let crates_path = repo_root.join("crates");
    // Ensure we use properly escaped path string for TOML if on Windows, but mainly we just need absolute path.
    // On Linux/macOS simple string replacement works.
    let crates_path_str = crates_path.to_str().unwrap();

    // Replace `path = "../crates/` with `path = "/absolute/path/to/crates/`
    let new_content = content.replace(
        "path = \"../crates/",
        &format!("path = \"{}/", crates_path_str),
    );

    std::fs::write(cargo_toml_path, new_content).expect("failed to write Cargo.toml");
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            if entry.file_name() == "target" || entry.file_name() == ".git" {
                continue;
            }
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn run_cli(temp_dir: &assert_fs::TempDir, args: &[&str]) -> String {
    let mut cmd = Command::cargo_bin("es-fluent").unwrap();

    // Set current dir to the temp dir root, or specifically the workspace root in temp?
    // TEST_DATA_DIR is the root of examples-tests.
    cmd.current_dir(temp_dir.path());
    cmd.args(args);
    // cmd.env("CLICOLOR_FORCE", "1"); // User requested NO color

    let assert = cmd.assert();
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Normalize output for snapshots
    let combined = format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr);
    normalize_output(&combined, temp_dir.path())
}

fn normalize_output(output: &str, cwd: &Path) -> String {
    use regex::Regex;

    // Strip ANSI escape codes
    let re_ansi = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    let output = re_ansi.replace_all(output, "");

    // Normalize temp dir path
    let output = output.replace(cwd.to_str().unwrap(), "[TEMP_DIR]");

    // Normalize durations (e.g. "123ms", "1.23s", "1s 500ms", "42us") -> "[DURATION]"
    // Support s, ms, us, ns
    let re_duration = Regex::new(r"(?:\d+(?:\.\d+)?(?:s|ms|us|ns)\s*)+").unwrap();
    let output = re_duration.replace_all(&output, "[DURATION]");

    // Normalize progress bar state if captured
    let output = output.replace(r"⠠", r""); // Spinner

    output.to_string()
}

#[test]
fn test_generate() {
    let temp = setup_test_env();
    let output = run_cli(&temp, &["generate", "--dry-run"]);
    assert_snapshot!(output);
}

#[test]
fn test_check() {
    let temp = setup_test_env();
    let output = run_cli(&temp, &["check"]);
    assert_snapshot!(output);
}

#[test]
fn test_fmt_dry_run() {
    let temp = setup_test_env();
    let output = run_cli(&temp, &["fmt", "--dry-run"]);
    assert_snapshot!(output);
}

#[test]
fn test_clean_dry_run() {
    let temp = setup_test_env();
    let output = run_cli(&temp, &["clean", "--dry-run"]);
    assert_snapshot!(output);
}

#[test]
fn test_check_complex_args() {
    let temp = setup_test_env();
    // Initially check should pass
    let output = run_cli(&temp, &["check"]);
    assert_snapshot!(output);
}

#[test]
fn test_check_complex_args_missing_variant() {
    let temp = setup_test_env();
    let ftl_path = temp.path().join("i18n/en/test-app-a.ftl");

    // Modify the FTL file to remove the default variant for user_gender
    let content = std::fs::read_to_string(&ftl_path).expect("failed to read ftl file");
    let new_content = content.replace("       *[other] their stream", "");
    std::fs::write(&ftl_path, new_content).expect("failed to write ftl file");

    let output = run_cli(&temp, &["check"]);
    assert_snapshot!(output);
}

#[test]
fn test_sync_locales() {
    let temp = setup_test_env();
    // Test -l and --all
    let output = run_cli(&temp, &["sync", "-l", "es"]);
    assert_snapshot!(output);
}

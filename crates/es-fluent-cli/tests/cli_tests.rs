use insta::assert_snapshot;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static TEST_DATA_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/e2e"));

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

    // Calculate repo root from CARGO_MANIFEST_DIR (crates/es-fluent-cli) -> ../.. -> root
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().unwrap().parent().unwrap();
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
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("es-fluent");

    // Set current dir to the temp dir root, or specifically the workspace root in temp?
    // TEST_DATA_DIR is the root of examples-tests.
    cmd.current_dir(temp_dir.path());

    // Inject --e2e flag for deterministic output
    let mut new_args = args.to_vec();
    new_args.push("--e2e");
    cmd.args(&new_args);

    let assert = cmd.assert();
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Normalize output for snapshots
    let combined = format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr);
    normalize_output(&combined, temp_dir.path())
}

fn normalize_output(output: &str, cwd: &Path) -> String {
    // Normalize temp dir path
    // We only need to normalize path separators if windows, but robust replacement handles strings.
    let output = output.replace(cwd.to_str().unwrap(), "[TEMP_DIR]");

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
fn test_sync_locales() {
    let temp = setup_test_env();
    // Test -l and --all
    let output = run_cli(&temp, &["sync", "-l", "es"]);
    assert_snapshot!(output);
}

#[test]
fn test_generate_real() {
    let temp = setup_test_env();
    // Run generate command (not dry-run)
    let output = run_cli(&temp, &["generate"]);
    assert_snapshot!(output);

    // Verify files are created
    // generated files are inside the crate's src/generated
    // Verify FTL file exists (it should have been updated or verified)
    // The previous assertion that it creates src/generated was wrong.
    // es-fluent generate updates FTL files from code.
    let ftl_path = temp.path().join("i18n/en/test-app-a.ftl");
    assert!(ftl_path.exists(), "FTL file should exist");
}

#[test]
fn test_fmt_real() {
    let temp = setup_test_env();
    let ftl_path = temp.path().join("i18n/en/test-app-a.ftl");

    // Read original content
    let original = std::fs::read_to_string(&ftl_path).expect("failed to read ftl file");

    // Make it ugly (remove spaces around equals)
    let ugly = original.replace(" = ", "=");
    std::fs::write(&ftl_path, &ugly).expect("failed to write ftl file");

    // Run fmt
    let output = run_cli(&temp, &["fmt"]);
    assert_snapshot!(output);

    // Read back and verify it's formatted (spaces restored)
    let formatted = std::fs::read_to_string(&ftl_path).expect("failed to read ftl file");
    assert_ne!(
        formatted, ugly,
        "File should have changed from ugly version"
    );
    assert!(
        formatted.contains(" = "),
        "Formatted file should contain spaces around equals"
    );
}

#[test]
fn test_clean_real() {
    let temp = setup_test_env();

    // Create an orphan key
    let ftl_path = temp.path().join("i18n/en/test-app-a.ftl");
    let content = std::fs::read_to_string(&ftl_path).expect("read");
    let new_content = format!("{}\norphan-key = Orphan\n", content);
    std::fs::write(&ftl_path, new_content).expect("write");

    // Now clean
    let output = run_cli(&temp, &["clean"]);
    assert_snapshot!(output);

    // Verify orphan is gone
    let cleaned_content = std::fs::read_to_string(&ftl_path).expect("read");
    assert!(
        !cleaned_content.contains("orphan-key"),
        "orphan key should be removed"
    );
}

#[test]
fn test_sync_all() {
    let temp = setup_test_env();

    // We need to set up a situation where sync does something.
    let en_path = temp.path().join("i18n/en/test-app-a.ftl");
    let mut content = std::fs::read_to_string(&en_path).expect("read en");
    content.push_str("\nnew-key = New Message\n");
    std::fs::write(&en_path, content).expect("write en");

    // Run sync all
    let output = run_cli(&temp, &["sync", "--all"]);
    assert_snapshot!(output);

    // Verify es/test-app-a.ftl has the new key
    let es_path = temp.path().join("i18n/es/test-app-a.ftl");
    let es_content = std::fs::read_to_string(&es_path).expect("read es");
    assert!(
        es_content.contains("new-key = New Message"),
        "es should contain new key"
    );
}

#[test]
fn test_sync_new_locale() {
    let temp = setup_test_env();

    // Sync to a new locale 'fr'
    let output = run_cli(&temp, &["sync", "-l", "fr"]);
    assert_snapshot!(output);

    // Verify fr exists (at i18n/fr)
    let fr_path = temp.path().join("i18n/fr/test-app-a.ftl");
    assert!(
        fr_path.exists(),
        "fr ftl should exist at path: {:?}",
        fr_path
    );

    let content = std::fs::read_to_string(&fr_path).expect("read fr");
    assert!(
        content.contains("hello_a = Hello from App A"),
        "fr should contain content"
    );
}

static FIXTURES_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"));

#[test]
fn test_check_syntax_error() {
    let temp = setup_test_env();
    let ftl_path = temp.path().join("i18n/en/test-app-a.ftl");

    // Copy fixture with syntax error
    let fixture_path = FIXTURES_DIR.join("check-syntax-error/test-app-a.ftl");
    std::fs::copy(&fixture_path, &ftl_path).expect("failed to copy fixture");

    // Run check, expect failure
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("es-fluent");
    cmd.current_dir(temp.path());
    cmd.args(&["check"]);
    cmd.assert().failure();
}

#[test]
fn test_check_missing_key() {
    let temp = setup_test_env();
    let ftl_path = temp.path().join("i18n/en/test-app-a.ftl");

    // Copy fixture with missing mandatory key
    let fixture_path = FIXTURES_DIR.join("check-missing-key/test-app-a.ftl");
    std::fs::copy(&fixture_path, &ftl_path).expect("failed to copy fixture");

    // Run check, expect failure
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("es-fluent");
    cmd.current_dir(temp.path());
    cmd.args(&["check"]);
    cmd.assert().failure();
}

#[test]
fn test_check_warning_missing_arg() {
    let temp = setup_test_env();
    let ftl_path = temp.path().join("i18n/en/test-app-a.ftl");

    // Copy fixture with missing argument (warning)
    let fixture_path = FIXTURES_DIR.join("check-missing-arg/test-app-a.ftl");
    std::fs::copy(&fixture_path, &ftl_path).expect("failed to copy fixture");

    // Run check, expect failure (exit code 1) because warnings are treated as issues by default logic in check.rs
    // "validation found 0 error(s) and 1 warning(s)" -> returns Err(CliError::Validation(...))
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("es-fluent");
    cmd.current_dir(temp.path());
    cmd.args(&["check"]);

    // Expect failure due to warning
    let assert = cmd.assert().failure();
    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check for warning message in stderr (miette usually prints to stderr)
    assert!(
        stderr.contains("user_name"),
        "Should warn about missing user_name variable in stderr"
    );
    assert!(
        stderr.contains("warning(s)"),
        "Should mention warnings count"
    );
}

#[test]
fn test_generate_mode_aggressive() {
    let temp = setup_test_env();
    let output = run_cli(&temp, &["generate", "--mode", "aggressive", "--dry-run"]);
    assert_snapshot!(output);
}

#[test]
fn test_clean_all() {
    let temp = setup_test_env();

    // Create orphan key in 'es' locale (not fallback)
    let ftl_path = temp.path().join("i18n/es/test-app-a.ftl");
    // Ensure 'es' folder exists and has content (sync test might have created it, but tests are isolated)
    // We need to make sure we have an 'es' file to clean.
    // Let's create one based on 'en' if it doesn't exist, or just append to it.
    if !ftl_path.exists() {
        std::fs::create_dir_all(ftl_path.parent().unwrap()).unwrap();
        std::fs::write(&ftl_path, "hello = Hola\norphan-key = Huerfano\n").unwrap();
    } else {
        let content = std::fs::read_to_string(&ftl_path).unwrap();
        let new_content = format!("{}\norphan-key = Huerfano\n", content);
        std::fs::write(&ftl_path, new_content).unwrap();
    }

    let output = run_cli(&temp, &["clean", "--all", "--dry-run"]);
    assert_snapshot!(output);
}

#[test]
fn test_fmt_all() {
    let temp = setup_test_env();

    // Create messy file in 'es' locale
    let ftl_path = temp.path().join("i18n/es/test-app-a.ftl");
    if !ftl_path.exists() {
        std::fs::create_dir_all(ftl_path.parent().unwrap()).unwrap();
        std::fs::write(&ftl_path, "b=2\na=1\n").unwrap();
    } else {
        std::fs::write(&ftl_path, "b=2\na=1\n").unwrap();
    }

    let output = run_cli(&temp, &["fmt", "--all", "--dry-run"]);
    assert_snapshot!(output);
}

#[test]
fn test_check_all() {
    let temp = setup_test_env();

    // Create missing key in 'es' locale
    let ftl_path_en = temp.path().join("i18n/en/test-app-a.ftl");
    // Assume EN has some keys.
    // Create ES file that is empty, so it's missing everything.
    let ftl_path_es = temp.path().join("i18n/es/test-app-a.ftl");
    std::fs::create_dir_all(ftl_path_es.parent().unwrap()).unwrap();
    std::fs::write(&ftl_path_es, "# Empty").unwrap();

    // Check with --all should fail because ES is missing keys present in EN (inventory)
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("es-fluent");
    cmd.current_dir(temp.path());
    cmd.args(&["check", "--all"]);
    cmd.assert().failure();
}

#[test]
fn test_sync_dry_run() {
    let temp = setup_test_env();

    // Add new key to EN
    let en_path = temp.path().join("i18n/en/test-app-a.ftl");
    let mut content = std::fs::read_to_string(&en_path).unwrap();
    content.push_str("\nnew-sync-key = Sync Me\n");
    std::fs::write(&en_path, content).unwrap();

    // Run sync --dry-run
    let output = run_cli(&temp, &["sync", "-l", "es", "--dry-run"]);
    assert_snapshot!(output);

    // Verify ES file NOT changed
    let es_path = temp.path().join("i18n/es/test-app-a.ftl");
    if es_path.exists() {
        let es_content = std::fs::read_to_string(&es_path).unwrap();
        assert!(
            !es_content.contains("new-sync-key"),
            "ES should NOT contain new key in dry-run"
        );
    }
}

#[test]
fn test_workspace_package() {
    let temp = setup_test_env();
    // Only generate for test-app-a, ignoring test-lib-b
    let output = run_cli(&temp, &["generate", "--package", "test-app-a", "--dry-run"]);
    assert_snapshot!(output);
}

#[test]
fn test_workspace_path() {
    let temp = setup_test_env();
    // Point explicitly to the crate dir
    let crate_path = temp.path().join("crates/test-app-a");
    let output = run_cli(
        &temp,
        &[
            "generate",
            "--path",
            crate_path.to_str().unwrap(),
            "--dry-run",
        ],
    );
    assert_snapshot!(output);
}

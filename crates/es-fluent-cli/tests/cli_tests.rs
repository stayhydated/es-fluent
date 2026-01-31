use insta::assert_snapshot;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static TEST_DATA_WORKSPACE_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/e2e_workspace"));

static TEST_DATA_PACKAGE_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/e2e_package"));

static TEST_DATA_CHECK_ISSUES_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/e2e_check_issues"));

fn setup_workspace_env() -> assert_fs::TempDir {
    let temp = assert_fs::TempDir::new().unwrap();
    copy_dir_recursive(&TEST_DATA_WORKSPACE_DIR, temp.path()).expect("failed to copy test data");
    fix_cargo_manifests(temp.path());
    temp
}

fn setup_package_env() -> assert_fs::TempDir {
    let temp = assert_fs::TempDir::new().unwrap();
    copy_dir_recursive(&TEST_DATA_PACKAGE_DIR, temp.path()).expect("failed to copy test data");
    fix_cargo_manifests(temp.path());
    temp
}

fn setup_check_issues_env() -> assert_fs::TempDir {
    let temp = assert_fs::TempDir::new().unwrap();
    copy_dir_recursive(&TEST_DATA_CHECK_ISSUES_DIR, temp.path()).expect("failed to copy test data");
    fix_cargo_manifests(temp.path());
    temp
}

fn fix_cargo_manifests(root: &Path) {
    let cargo_toml_path = root.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml_path).expect("failed to read Cargo.toml");

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().unwrap().parent().unwrap();
    let crates_path = repo_root.join("crates");
    let crates_path_str = crates_path.to_str().unwrap();

    let new_content = content
        .replace(
            "path = \"../crates/",
            &format!("path = \"{}/", crates_path_str),
        )
        .replace(
            "path = \"../../../crates/",
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
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cargo-es-fluent");

    // Set current dir to the temp dir root, or specifically the workspace root in temp?
    // TEST_DATA_DIR is the root of examples-tests.
    cmd.current_dir(temp_dir.path());

    // Inject "es-fluent" subcommand (required for cargo subcommand pattern) and --e2e flag
    let mut new_args = vec!["es-fluent"];
    new_args.extend(args);
    new_args.push("--e2e");
    cmd.args(&new_args);

    // Disable all colors via standard env var
    cmd.env("NO_COLOR", "1");
    // Disable hyperlinks
    cmd.env("FORCE_HYPERLINK", "0");

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

/// Configuration for E2E test suite - holds setup info for workspace or package tests.
struct E2eConfig {
    /// Setup function that creates the temp directory
    setup: fn() -> assert_fs::TempDir,
    /// Relative path to the FTL file within the temp directory
    ftl_path: &'static str,
    /// Source lib.rs path (relative) for incremental tests
    src_lib_path: &'static str,
}

const WORKSPACE_CONFIG: E2eConfig = E2eConfig {
    setup: setup_workspace_env,
    ftl_path: "i18n/en/test-app-a.ftl",
    src_lib_path: "crates/test-app-a/src/lib.rs",
};

const PACKAGE_CONFIG: E2eConfig = E2eConfig {
    setup: setup_package_env,
    ftl_path: "i18n/en/test-app-package.ftl",
    src_lib_path: "src/lib.rs",
};

const CHECK_ISSUES_CONFIG: E2eConfig = E2eConfig {
    setup: setup_check_issues_env,
    ftl_path: "i18n/en/test-check-issues.ftl",
    src_lib_path: "src/lib.rs",
};

// =============================================================================
// Core Test Functions - Reusable logic for both workspace and package suites
// =============================================================================

fn run_generate_dry_run_test(config: &E2eConfig) -> String {
    let temp = (config.setup)();
    run_cli(&temp, &["generate", "--dry-run"])
}

fn run_check_test(config: &E2eConfig) -> String {
    let temp = (config.setup)();
    run_cli(&temp, &["check"])
}

fn run_fmt_dry_run_test(config: &E2eConfig) -> String {
    let temp = (config.setup)();
    run_cli(&temp, &["fmt", "--dry-run"])
}

fn run_clean_dry_run_test(config: &E2eConfig) -> String {
    let temp = (config.setup)();
    run_cli(&temp, &["clean", "--dry-run"])
}

fn run_generate_real_test(config: &E2eConfig) -> String {
    let temp = (config.setup)();
    let output = run_cli(&temp, &["generate"]);

    let ftl_path = temp.path().join(config.ftl_path);
    assert!(ftl_path.exists(), "FTL file should exist");

    output
}

fn run_generate_namespaced_output_test() -> String {
    let temp = setup_workspace_env();

    let src_path = temp.path().join("crates/test-app-a/src/lib.rs");
    let mut content = std::fs::read_to_string(&src_path).expect("read lib.rs");
    content
        .push_str("\n\n#[derive(EsFluent)]\n#[fluent(namespace = \"ui\")]\npub struct UiBanner;\n");
    std::fs::write(&src_path, content).expect("write lib.rs");

    let output = run_cli(&temp, &["generate"]);

    let namespaced_path = temp.path().join("i18n/en/test-app-a/ui.ftl");
    assert!(namespaced_path.exists(), "Namespaced FTL file should exist");
    let namespaced_content = std::fs::read_to_string(&namespaced_path).expect("read ui.ftl");
    assert!(
        namespaced_content.contains("ui_banner"),
        "Namespaced file should contain the new key"
    );

    let default_path = temp.path().join("i18n/en/test-app-a.ftl");
    let default_content = std::fs::read_to_string(&default_path).expect("read default ftl");
    assert!(
        !default_content.contains("UiBanner"),
        "Default file should not contain namespaced group"
    );

    output
}

fn run_generate_file_relative_namespace_test() -> String {
    let temp = setup_workspace_env();

    let src_path = temp.path().join("crates/test-app-a/src/lib.rs");
    let mut content = std::fs::read_to_string(&src_path).expect("read lib.rs");
    content.push_str(
        r#"
#[derive(EsFluent)]
#[fluent(namespace(file(relative)))]
pub struct FileRelativeNs;
"#,
    );
    std::fs::write(&src_path, content).expect("write lib.rs");

    let output = run_cli(&temp, &["generate"]);

    let namespaced_path = temp.path().join("i18n/en/test-app-a/lib.ftl");
    assert!(
        namespaced_path.exists(),
        "File-relative namespace should generate lib.ftl under i18n"
    );
    let namespaced_content = std::fs::read_to_string(&namespaced_path).expect("read lib.ftl");
    assert!(
        namespaced_content.contains("file_relative_ns"),
        "Namespaced file should contain the new key"
    );

    let stray_path = temp.path().join("crates/test-app-a/src/lib.ftl");
    assert!(
        !stray_path.exists(),
        "Should not create lib.ftl inside the source directory"
    );

    output
}

fn run_generate_invalid_namespace_file_relative_test() -> String {
    let temp = setup_workspace_env();

    let i18n_path = temp.path().join("crates/test-app-a/i18n.toml");
    let mut toml = std::fs::read_to_string(&i18n_path).expect("read i18n.toml");
    toml.push_str("\nnamespaces = [\"ui\"]\n");
    std::fs::write(&i18n_path, toml).expect("write i18n.toml");

    let src_path = temp.path().join("crates/test-app-a/src/lib.rs");
    let mut content = std::fs::read_to_string(&src_path).expect("read lib.rs");
    content.push_str(
        r#"
#[derive(EsFluent)]
#[fluent(namespace(file(relative)))]
pub struct InvalidNs;
"#,
    );
    std::fs::write(&src_path, content).expect("write lib.rs");

    run_cli(&temp, &["generate"])
}

fn run_fmt_real_test(config: &E2eConfig) -> String {
    let temp = (config.setup)();
    let ftl_path = temp.path().join(config.ftl_path);

    let original = std::fs::read_to_string(&ftl_path).expect("failed to read ftl file");
    let ugly = original.replace(" = ", "=");
    std::fs::write(&ftl_path, &ugly).expect("failed to write ftl file");

    let output = run_cli(&temp, &["fmt"]);

    let formatted = std::fs::read_to_string(&ftl_path).expect("failed to read ftl file");
    assert_ne!(
        formatted, ugly,
        "File should have changed from ugly version"
    );
    assert!(
        formatted.contains(" = "),
        "Formatted file should contain spaces around equals"
    );

    output
}

fn run_clean_real_test(config: &E2eConfig) -> String {
    let temp = (config.setup)();
    let ftl_path = temp.path().join(config.ftl_path);
    let content = std::fs::read_to_string(&ftl_path).expect("read");
    let new_content = format!("{}\norphan-key = Orphan\n", content);
    std::fs::write(&ftl_path, new_content).expect("write");

    let output = run_cli(&temp, &["clean"]);

    let cleaned_content = std::fs::read_to_string(&ftl_path).expect("read");
    assert!(
        !cleaned_content.contains("orphan-key"),
        "orphan key should be removed"
    );

    output
}

fn run_sync_dry_run_test(config: &E2eConfig) -> String {
    let temp = (config.setup)();
    let ftl_path = temp.path().join(config.ftl_path);
    let mut content = std::fs::read_to_string(&ftl_path).unwrap();
    content.push_str("\nnew-sync-key = Sync Me\n");
    std::fs::write(&ftl_path, content).unwrap();

    let output = run_cli(&temp, &["sync", "-l", "es", "--dry-run"]);

    let ftl_filename = config.ftl_path.split('/').last().unwrap();
    let es_path = temp.path().join("i18n/es").join(ftl_filename);
    if es_path.exists() {
        let es_content = std::fs::read_to_string(&es_path).unwrap();
        assert!(
            !es_content.contains("new-sync-key"),
            "ES should NOT contain new key in dry-run"
        );
    }

    output
}

fn run_generate_mode_aggressive_test(config: &E2eConfig) -> String {
    let temp = (config.setup)();
    run_cli(&temp, &["generate", "--mode", "aggressive", "--dry-run"])
}

fn run_generate_incremental_test(config: &E2eConfig) -> (String, String) {
    let temp = (config.setup)();
    let initial_output = run_cli(&temp, &["generate"]);

    // Sleep to ensure file mtime passes (cargo mtime resolution)
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Add a new struct to the crate
    let src_path = temp.path().join(config.src_lib_path);
    let mut content = std::fs::read_to_string(&src_path).expect("read lib.rs");
    content.push_str("\n\n#[derive(EsFluent)]\npub struct IncrementalTest;\n");
    std::fs::write(&src_path, content).expect("write lib.rs");

    // Run generate again
    let incremental_output = run_cli(&temp, &["generate"]);

    let ftl_path = temp.path().join(config.ftl_path);
    let ftl_content = std::fs::read_to_string(&ftl_path).expect("read ftl");
    assert!(
        ftl_content.contains("incremental_test"),
        "FTL should contain the new struct's key after incremental generation"
    );

    (initial_output, incremental_output)
}

fn run_version_mismatch_test(temp: assert_fs::TempDir) {
    use es_fluent_cli::generation::cache::RunnerCache;

    // 1. Run generate normally to establish baseline (fresh runner)
    let _ = run_cli(&temp, &["generate"]);

    let cache_path = temp.path().join(".es-fluent/runner_cache.json");
    let binary_path = temp.path().join("target/debug/es-fluent-runner");

    // Record initial binary mtime (if exists)
    let initial_mtime = std::fs::metadata(&binary_path)
        .and_then(|m| m.modified())
        .ok();

    // 2. Simulate older CLI version by setting cache version to "0.0.0"
    let cache_content = std::fs::read_to_string(&cache_path).expect("read cache");
    let mut cache: RunnerCache = serde_json::from_str(&cache_content).expect("parse cache");
    let original_version = cache.cli_version.clone();
    cache.cli_version = "0.0.0".to_string();
    let new_cache_content = serde_json::to_string_pretty(&cache).expect("serialize cache");
    std::fs::write(&cache_path, new_cache_content).expect("write stale cache");

    // Small delay to ensure mtime granularity
    std::thread::sleep(std::time::Duration::from_millis(1000));

    // 3. Run generate again - should detect version mismatch and rebuild
    let _ = run_cli(&temp, &["generate"]);

    // 4. Verify: Cache version should be updated (not "0.0.0")
    let final_cache = std::fs::read_to_string(&cache_path).expect("read final cache");
    let final_cache: RunnerCache = serde_json::from_str(&final_cache).expect("parse final cache");
    assert_ne!(
        final_cache.cli_version, "0.0.0",
        "Cache should have updated version to current CLI version"
    );
    assert_eq!(
        final_cache.cli_version, original_version,
        "Cache version should match the CLI's actual version"
    );

    // 5. Verify: Binary should have been rebuilt (mtime changed)
    if let Some(initial) = initial_mtime {
        let final_mtime = std::fs::metadata(&binary_path)
            .and_then(|m| m.modified())
            .expect("binary should exist after rebuild");
        assert!(
            final_mtime > initial,
            "Binary mtime should have increased after version mismatch rebuild"
        );
    }
}

mod workspace {
    use super::*;

    #[test]
    fn test_generate_dry_run() {
        assert_snapshot!(run_generate_dry_run_test(&WORKSPACE_CONFIG));
    }

    #[test]
    fn test_check() {
        assert_snapshot!(run_check_test(&WORKSPACE_CONFIG));
    }

    #[test]
    fn test_fmt_dry_run() {
        assert_snapshot!(run_fmt_dry_run_test(&WORKSPACE_CONFIG));
    }

    #[test]
    fn test_clean_dry_run() {
        assert_snapshot!(run_clean_dry_run_test(&WORKSPACE_CONFIG));
    }

    #[test]
    fn test_generate_real() {
        assert_snapshot!(run_generate_real_test(&WORKSPACE_CONFIG));
    }

    #[test]
    fn test_fmt_real() {
        assert_snapshot!(run_fmt_real_test(&WORKSPACE_CONFIG));
    }

    #[test]
    fn test_clean_real() {
        assert_snapshot!(run_clean_real_test(&WORKSPACE_CONFIG));
    }

    #[test]
    fn test_sync_dry_run() {
        assert_snapshot!(run_sync_dry_run_test(&WORKSPACE_CONFIG));
    }

    #[test]
    fn test_generate_mode_aggressive() {
        assert_snapshot!(run_generate_mode_aggressive_test(&WORKSPACE_CONFIG));
    }

    #[test]
    fn test_generate_incremental() {
        let (initial, incremental) = run_generate_incremental_test(&WORKSPACE_CONFIG);
        assert_snapshot!(initial);
        assert_snapshot!(incremental);
    }

    #[test]
    fn test_version_mismatch_rebuild_workspace() {
        let temp = setup_workspace_env();
        run_version_mismatch_test(temp);
    }
}

mod package {
    use super::*;

    #[test]
    fn test_generate_dry_run() {
        assert_snapshot!(run_generate_dry_run_test(&PACKAGE_CONFIG));
    }

    #[test]
    fn test_check() {
        assert_snapshot!(run_check_test(&PACKAGE_CONFIG));
    }

    #[test]
    fn test_fmt_dry_run() {
        assert_snapshot!(run_fmt_dry_run_test(&PACKAGE_CONFIG));
    }

    #[test]
    fn test_clean_dry_run() {
        assert_snapshot!(run_clean_dry_run_test(&PACKAGE_CONFIG));
    }

    #[test]
    fn test_generate_real() {
        assert_snapshot!(run_generate_real_test(&PACKAGE_CONFIG));
    }

    #[test]
    fn test_fmt_real() {
        assert_snapshot!(run_fmt_real_test(&PACKAGE_CONFIG));
    }

    #[test]
    fn test_clean_real() {
        assert_snapshot!(run_clean_real_test(&PACKAGE_CONFIG));
    }

    #[test]
    fn test_sync_dry_run() {
        assert_snapshot!(run_sync_dry_run_test(&PACKAGE_CONFIG));
    }

    #[test]
    fn test_generate_mode_aggressive() {
        assert_snapshot!(run_generate_mode_aggressive_test(&PACKAGE_CONFIG));
    }

    #[test]
    fn test_generate_incremental() {
        let (initial, incremental) = run_generate_incremental_test(&PACKAGE_CONFIG);
        assert_snapshot!(initial);
        assert_snapshot!(incremental);
    }

    #[test]
    fn test_version_mismatch_rebuild_package() {
        let temp = setup_package_env();
        run_version_mismatch_test(temp);
    }
}

#[test]
fn test_workspace_package() {
    let temp = setup_workspace_env();
    // Only generate for test-app-a, ignoring test-lib-b
    let output = run_cli(&temp, &["generate", "--package", "test-app-a", "--dry-run"]);
    assert_snapshot!(output);
}

#[test]
fn test_workspace_path() {
    let temp = setup_workspace_env();
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

#[test]
fn test_sync_all() {
    let temp = setup_workspace_env();

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
    let temp = setup_workspace_env();

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
    let temp = setup_workspace_env();
    let ftl_path = temp.path().join("i18n/en/test-app-a.ftl");

    // Copy fixture with syntax error
    let fixture_path = FIXTURES_DIR.join("check-syntax-error/test-app-a.ftl");
    std::fs::copy(&fixture_path, &ftl_path).expect("failed to copy fixture");

    // Run check and snapshot output
    let output = run_cli(&temp, &["check"]);
    assert_snapshot!(output);
}

#[test]
fn test_check_missing_key() {
    let temp = setup_workspace_env();
    let ftl_path = temp.path().join("i18n/en/test-app-a.ftl");

    // Copy fixture with missing mandatory key
    let fixture_path = FIXTURES_DIR.join("check-missing-key/test-app-a.ftl");
    std::fs::copy(&fixture_path, &ftl_path).expect("failed to copy fixture");

    // Run check, expect failure
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cargo-es-fluent");
    cmd.current_dir(temp.path());
    cmd.args(&["es-fluent", "check"]);
    cmd.assert().failure();
}

#[test]
fn test_check_warning_missing_arg() {
    let temp = setup_workspace_env();
    let ftl_path = temp.path().join("i18n/en/test-app-a.ftl");

    // Copy fixture with missing argument (warning)
    let fixture_path = FIXTURES_DIR.join("check-missing-arg/test-app-a.ftl");
    std::fs::copy(&fixture_path, &ftl_path).expect("failed to copy fixture");

    // Run check, expect failure (exit code 1) because warnings are treated as issues by default logic in check.rs
    // "validation found 0 error(s) and 1 warning(s)" -> returns Err(CliError::Validation(...))
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cargo-es-fluent");
    cmd.current_dir(temp.path());
    cmd.args(&["es-fluent", "check"]);

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
fn test_clean_all() {
    let temp = setup_workspace_env();

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
    let temp = setup_workspace_env();

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
    let temp = setup_workspace_env();

    // Assume EN has some keys.
    // Create ES file that is empty, so it's missing everything.
    let ftl_path_es = temp.path().join("i18n/es/test-app-a.ftl");
    std::fs::create_dir_all(ftl_path_es.parent().unwrap()).unwrap();
    std::fs::write(&ftl_path_es, "# Empty").unwrap();

    // Check with --all should fail because ES is missing keys present in EN (inventory)
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cargo-es-fluent");
    cmd.current_dir(temp.path());
    cmd.args(&["es-fluent", "check", "--all"]);
    cmd.assert().failure();
}

#[test]
fn test_check_ignore_unknown_crate() {
    let temp = setup_workspace_env();

    // Try to ignore a crate that doesn't exist in the workspace
    let output = run_cli(&temp, &["check", "--ignore", "nonexistent-crate"]);
    assert_snapshot!(output);
}

#[test]
fn test_check_ignore_multiple_unknown_crates() {
    let temp = setup_workspace_env();

    // Try to ignore multiple crates that don't exist
    let output = run_cli(
        &temp,
        &[
            "check",
            "--ignore",
            "fake-crate-1,fake-crate-2",
            "--ignore",
            "another-fake",
        ],
    );
    assert_snapshot!(output);
}

#[test]
fn test_check_ignore_valid_crate() {
    let temp = setup_workspace_env();

    // Ignore test-lib-b, should only check test-app-a
    let output = run_cli(&temp, &["check", "--ignore", "test-lib-b"]);
    assert_snapshot!(output);
}

#[test]
fn test_check_ignore_all_crates() {
    let temp = setup_workspace_env();

    // Ignore all crates - should report no crates found
    let output = run_cli(&temp, &["check", "--ignore", "test-app-a,test-lib-b"]);
    assert_snapshot!(output);
}

#[test]
fn test_generate_force_run() {
    let temp = setup_workspace_env();

    // First run to populate cache
    run_cli(&temp, &["generate"]);

    // Second run with --force-run should still rebuild
    let output = run_cli(&temp, &["generate", "--force-run", "--dry-run"]);
    assert_snapshot!(output);
}

#[test]
fn test_generate_namespaced_output() {
    assert_snapshot!(run_generate_namespaced_output_test());
}

#[test]
fn test_generate_file_relative_namespace() {
    assert_snapshot!(run_generate_file_relative_namespace_test());
}

#[test]
fn test_generate_invalid_namespace_file_relative() {
    let output = run_generate_invalid_namespace_file_relative_test();
    assert!(
        output.contains("InvalidNamespace"),
        "Expected invalid namespace error, got:\n{}",
        output
    );
    assert!(
        output.contains("namespace: \"lib\""),
        "Expected resolved namespace in error output, got:\n{}",
        output
    );
}

#[test]
fn test_sync_namespaced() {
    let temp = setup_workspace_env();

    // First, generate namespaced FTL files
    let src_path = temp.path().join("crates/test-app-a/src/lib.rs");
    let mut content = std::fs::read_to_string(&src_path).expect("read lib.rs");
    content
        .push_str("\n\n#[derive(EsFluent)]\n#[fluent(namespace = \"ui\")]\npub struct UiBanner;\n");
    std::fs::write(&src_path, content).expect("write lib.rs");

    // Run generate to create the namespaced file in en
    run_cli(&temp, &["generate"]);

    // Verify the namespaced file exists in en
    let en_namespaced_path = temp.path().join("i18n/en/test-app-a/ui.ftl");
    assert!(
        en_namespaced_path.exists(),
        "Namespaced FTL file should exist in en"
    );

    // Now run sync --all to sync to es
    let output = run_cli(&temp, &["sync", "--all"]);

    // Verify the namespaced file was created in es with the proper subdirectory
    let es_namespaced_path = temp.path().join("i18n/es/test-app-a/ui.ftl");
    assert!(
        es_namespaced_path.exists(),
        "Namespaced FTL file should exist in es at {:?}",
        es_namespaced_path
    );

    // Verify the content was synced
    let es_content = std::fs::read_to_string(&es_namespaced_path).expect("read es ui.ftl");
    assert!(
        es_content.contains("ui_banner"),
        "es namespaced file should contain ui_banner key"
    );

    // Also verify the main file still syncs correctly
    let es_main_path = temp.path().join("i18n/es/test-app-a.ftl");
    assert!(es_main_path.exists(), "Main FTL file should exist in es");

    assert_snapshot!(output);
}

#[test]
fn test_check_force_run() {
    let temp = setup_workspace_env();

    // First run to populate cache
    run_cli(&temp, &["check"]);

    // Second run with --force-run should still rebuild
    let output = run_cli(&temp, &["check", "--force-run"]);
    assert_snapshot!(output);
}

#[test]
fn test_clean_force_run() {
    let temp = setup_workspace_env();

    // First run to populate cache
    run_cli(&temp, &["clean"]);

    // Second run with --force-run should still rebuild
    let output = run_cli(&temp, &["clean", "--force-run", "--dry-run"]);
    assert_snapshot!(output);
}

#[test]
fn test_clean_orphaned_dry_run() {
    let temp = setup_workspace_env();

    // Create an orphaned FTL file in a non-fallback locale (es)
    // This simulates a file that was synced but the crate no longer exists
    let orphaned_path = temp.path().join("i18n/es/old-crate.ftl");
    std::fs::write(&orphaned_path, "old-key = Old Value\n").expect("write orphaned file");

    // Run clean --orphaned --all --dry-run
    let output = run_cli(&temp, &["clean", "--orphaned", "--all", "--dry-run"]);

    // Verify the file still exists (dry run)
    assert!(
        orphaned_path.exists(),
        "Orphaned file should still exist after dry run"
    );

    assert_snapshot!(output);
}

#[test]
fn test_clean_orphaned_real() {
    let temp = setup_workspace_env();

    // Create an orphaned FTL file in a non-fallback locale (es)
    let orphaned_path = temp.path().join("i18n/es/old-crate.ftl");
    std::fs::write(&orphaned_path, "old-key = Old Value\n").expect("write orphaned file");

    // Run clean --orphaned --all
    let output = run_cli(&temp, &["clean", "--orphaned", "--all"]);

    // Verify the orphaned file is removed
    assert!(!orphaned_path.exists(), "Orphaned file should be removed");

    // Verify the legitimate files still exist
    let legit_path = temp.path().join("i18n/es/test-app-a.ftl");
    assert!(legit_path.exists(), "Legitimate file should still exist");

    assert_snapshot!(output);
}

/// Test that orphaned main FTL files are detected when a crate only uses namespaces.
/// This tests the scenario where a crate like bevy-example only has namespaced types
/// (e.g., bevy-example/ui.ftl) but no main file (bevy-example.ftl).
/// The main file in non-fallback locales should be considered orphaned.
#[test]
fn test_clean_orphaned_namespaced_only_crate() {
    let temp = setup_workspace_env();

    // Simulate a crate that only uses namespaces (like bevy-example)
    // The main FTL file exists in non-fallback locale but NOT in fallback
    let orphaned_main = temp.path().join("i18n/es/namespaced-only.ftl");
    std::fs::write(&orphaned_main, "key = value\n").expect("write orphaned main file");

    // Create the namespaced file in fallback (this is the "real" file)
    let ns_dir = temp.path().join("i18n/en/namespaced-only");
    std::fs::create_dir_all(&ns_dir).expect("create ns dir");
    std::fs::write(ns_dir.join("ui.ftl"), "ui_key = UI Value\n").expect("write ns file");

    // Also create the namespaced file in es (this should NOT be orphaned)
    let ns_dir_es = temp.path().join("i18n/es/namespaced-only");
    std::fs::create_dir_all(&ns_dir_es).expect("create ns dir es");
    std::fs::write(ns_dir_es.join("ui.ftl"), "ui_key = UI Value ES\n").expect("write ns file es");

    // Run clean --orphaned --all --dry-run
    let output = run_cli(&temp, &["clean", "--orphaned", "--all", "--dry-run"]);

    // The orphaned main file should be detected
    assert!(
        output.contains("namespaced-only.ftl"),
        "Should detect orphaned main FTL file: {}",
        output
    );

    // Verify files still exist (dry run)
    assert!(
        orphaned_main.exists(),
        "Orphaned main file should still exist after dry run"
    );
    assert!(
        ns_dir_es.join("ui.ftl").exists(),
        "Namespaced file should still exist after dry run"
    );
}

/// Test that clean --orphaned preserves legitimate files in the fallback locale.
/// We should never delete files from the fallback locale.
#[test]
fn test_clean_orphaned_preserves_fallback() {
    let temp = setup_workspace_env();

    // Create a file in the fallback locale (en) that doesn't correspond to any crate
    // Even though this looks orphaned, we should NOT touch fallback locale files
    let fallback_file = temp.path().join("i18n/en/orphan-in-fallback.ftl");
    std::fs::write(&fallback_file, "key = value\n").expect("write fallback file");

    // Run clean --orphaned --all
    let output = run_cli(&temp, &["clean", "--orphaned", "--all"]);

    // The file in fallback should NOT be removed
    assert!(
        fallback_file.exists(),
        "Fallback locale files should never be removed, even if orphaned"
    );

    // Should report no orphaned files found (since we skip fallback)
    assert!(
        output.contains("No orphaned FTL files found"),
        "Should report no orphaned files: {}",
        output
    );
}

mod check_issues {
    use super::*;

    #[test]
    fn test_check_issues() {
        // This should fail because of missing keys and variables
        let temp = (CHECK_ISSUES_CONFIG.setup)();
        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cargo-es-fluent");
        cmd.current_dir(temp.path());
        cmd.args(&["es-fluent", "check"]);

        // It should exit with failure code
        cmd.assert().failure();

        // Run again with run_cli to capture output for snapshot
        // Note: run_cli doesn't check exit code, just returns output
        let output = run_cli(&temp, &["check"]);
        assert_snapshot!(output);
    }
}

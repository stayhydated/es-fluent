use std::fs;
use std::path::Path;
use std::process::Command;

fn assert_compile_fails(package_name: &str, source: &str, expected_stderr: &[&str]) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp_dir = tempfile::Builder::new()
        .prefix(package_name)
        .tempdir()
        .expect("create temporary compile-fail crate");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("create compile-fail src directory");

    fs::write(
        temp_dir.path().join("Cargo.toml"),
        format!(
            r#"[package]
name = "{package_name}"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
es-fluent-manager-embedded = {{ path = "{}" }}
"#,
            manifest_dir.display()
        ),
    )
    .expect("write compile-fail Cargo.toml");

    fs::write(src_dir.join("main.rs"), source).expect("write compile-fail main.rs");

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .arg("check")
        .arg("--quiet")
        .arg("--target-dir")
        .arg(temp_dir.path().join("target"))
        .current_dir(temp_dir.path())
        .output()
        .expect("run cargo check for compile-fail crate");

    assert!(
        !output.status.success(),
        "expected {package_name} to fail compilation\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    for expected in expected_stderr {
        assert!(
            stderr.contains(expected),
            "expected {package_name} stderr to contain {expected:?}\nstderr:\n{stderr}"
        );
    }
}

#[test]
fn old_embedded_singleton_startup_functions_are_unavailable() {
    assert_compile_fails(
        "removed_embedded_startup_functions",
        include_str!("compile_fail/removed_startup_functions.rs"),
        &[
            "cannot find function `init`",
            "cannot find function `try_init`",
            "cannot find function `init_with_language`",
            "cannot find function `try_init_with_language`",
        ],
    );
}

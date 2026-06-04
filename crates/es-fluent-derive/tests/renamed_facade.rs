use std::process::Command;

use fs_err as fs;
use tempfile::TempDir;

#[test]
fn derives_compile_when_es_fluent_dependency_is_renamed() {
    let temp = TempDir::new().expect("create temp crate");
    let crate_dir = temp.path();
    let src_dir = crate_dir.join("src");
    fs::create_dir_all(&src_dir).expect("create src dir");

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root");
    let facade_path = workspace_root.join("crates/es-fluent");

    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            r#"
[package]
name = "renamed-facade-fixture"
version = "0.0.0"
edition = "2024"

[dependencies]
localized = {{ package = "es-fluent", path = "{}" }}
"#,
            facade_path.display()
        ),
    )
    .expect("write Cargo.toml");

    fs::write(
        src_dir.join("lib.rs"),
        r#"
use localized::{EsFluent, EsFluentChoice, EsFluentLabel, EsFluentVariants};

#[derive(EsFluentChoice)]
pub enum Tone {
    Friendly,
}

#[derive(EsFluent)]
pub struct Greeting {
    pub name: String,
    #[fluent(selector)]
    pub tone: Tone,
}

#[derive(EsFluentLabel, EsFluentVariants)]
#[fluent_label(origin, variants)]
pub struct LoginForm {
    pub username: String,
}
"#,
    )
    .expect("write lib.rs");

    let output = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(crate_dir.join("Cargo.toml"))
        .output()
        .expect("run cargo check");

    assert!(
        output.status.success(),
        "cargo check failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

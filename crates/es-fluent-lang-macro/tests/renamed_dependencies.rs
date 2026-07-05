use std::process::Command;

use path_slash::PathExt as _;
use tempfile::TempDir;

fn toml_path(path: &std::path::Path) -> String {
    path.to_slash_lossy().into_owned()
}

#[test]
fn language_macro_compiles_when_runtime_dependencies_are_renamed() {
    let temp = TempDir::new().expect("create temp crate");
    let crate_dir = temp.path();
    let src_dir = crate_dir.join("src");
    std::fs::create_dir_all(src_dir).expect("create src dir");
    std::fs::create_dir_all(crate_dir.join("i18n/en")).expect("create locale dir");

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root");
    let facade_path = workspace_root.join("crates/es-fluent");
    let lang_path = workspace_root.join("crates/es-fluent-lang");

    std::fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            r#"
[package]
name = "renamed-language-fixture"
version = "0.0.0"
edition = "2024"

[dependencies]
localized = {{ package = "es-fluent", path = "{}" }}
language_pack = {{ package = "es-fluent-lang", path = "{}" }}
"#,
            toml_path(&facade_path),
            toml_path(&lang_path)
        ),
    )
    .expect("write Cargo.toml");

    std::fs::write(
        crate_dir.join("i18n.toml"),
        r#"
fallback_language = "en"
assets_dir = "i18n"
"#,
    )
    .expect("write i18n.toml");

    std::fs::write(
        crate_dir.join("src/lib.rs"),
        r#"
use language_pack::es_fluent_language;

#[es_fluent_language(custom)]
pub enum Languages {}

pub fn fallback_language() -> localized::unic_langid::LanguageIdentifier {
    Languages::default().into()
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

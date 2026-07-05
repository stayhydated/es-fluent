use std::process::Command;

use path_slash::PathExt as _;
use tempfile::TempDir;

fn toml_path(path: &std::path::Path) -> String {
    path.to_slash_lossy().into_owned()
}

#[test]
fn manager_macros_compile_when_runtime_dependencies_are_renamed() {
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
    let embedded_path = workspace_root.join("crates/es-fluent-manager-embedded");
    let bevy_path = workspace_root.join("crates/es-fluent-manager-bevy");
    let dioxus_path = workspace_root.join("crates/es-fluent-manager-dioxus");
    let lang_path = workspace_root.join("crates/es-fluent-lang");

    std::fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            r#"
[package]
name = "renamed-manager-fixture"
version = "0.0.0"
edition = "2024"

[dependencies]
localized = {{ package = "es-fluent", path = "{}" }}
embedded_runtime = {{ package = "es-fluent-manager-embedded", path = "{}" }}
bevy_runtime = {{ package = "es-fluent-manager-bevy", path = "{}", default-features = false, features = ["macros"] }}
dioxus_runtime = {{ package = "es-fluent-manager-dioxus", path = "{}" }}
language_pack = {{ package = "es-fluent-lang", path = "{}" }}
"#,
            toml_path(&facade_path),
            toml_path(&embedded_path),
            toml_path(&bevy_path),
            toml_path(&dioxus_path),
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
    std::fs::write(crate_dir.join("i18n/en/renamed-manager-fixture.ftl"), "").expect("write ftl");

    std::fs::write(
        crate_dir.join("src/lib.rs"),
        r#"
embedded_runtime::define_i18n_module!();
dioxus_runtime::define_i18n_module!();

use language_pack::es_fluent_language;

#[es_fluent_language]
#[derive(Clone, Copy)]
pub enum Languages {}

#[derive(bevy_runtime::BevyFluentText, Clone)]
pub struct Message;

#[derive(bevy_runtime::BevyFluentText, Clone)]
pub struct LocalizedMessage {
    #[locale]
    language: Languages,
}

impl localized::FluentMessage for Message {
    fn to_fluent_string_with(
        &self,
        _localize: &mut localized::FluentMessageLookup<'_>,
    ) -> String {
        String::new()
    }
}

impl localized::FluentMessage for LocalizedMessage {
    fn to_fluent_string_with(
        &self,
        _localize: &mut localized::FluentMessageLookup<'_>,
    ) -> String {
        String::new()
    }
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

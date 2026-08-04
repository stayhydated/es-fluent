use std::fs;
use std::path::PathBuf;

#[test]
fn macro_failures_match_user_diagnostics() {
    let trybuild_manifest = workspace_target_dir().join("tests/trybuild/es-fluent-derive");
    fs::create_dir_all(&trybuild_manifest).expect("create trybuild manifest dir");
    fs::write(
        trybuild_manifest.join("i18n.toml"),
        r#"
fallback_language = "en"
assets_dir = "i18n"
domains = ["auth"]
namespaces = ["allowed"]
"#,
    )
    .expect("write trybuild i18n.toml");

    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
    tests.pass("tests/ui-pass/*.rs");
}

fn workspace_target_dir() -> PathBuf {
    if let Some(target_dir) = std::env::var_os("CARGO_TARGET_DIR") {
        return PathBuf::from(target_dir);
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .join("target")
}

use std::fs;
use std::path::PathBuf;

#[test]
fn namespace_allowlist_failures_match_user_diagnostics() {
    let workspace_target = workspace_target_dir();
    let trybuild_manifest = workspace_target.join("tests/trybuild/es-fluent-derive");

    fs::create_dir_all(&trybuild_manifest).expect("create trybuild manifest dir");
    fs::write(
        trybuild_manifest.join("i18n.toml"),
        r#"
fallback_language = "en"
assets_dir = "i18n"
namespaces = ["allowed"]
"#,
    )
    .expect("write trybuild i18n.toml");

    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui_namespace/*.rs");
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

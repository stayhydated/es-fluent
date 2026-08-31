use crate::*;

#[test]
fn binary_fmt_json_invalid_path_includes_requested_path() {
    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            "/definitely/missing/path",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"][0].as_str().is_some_and(|message| {
        message.contains("Failed to canonicalize root directory")
            && message.contains("/definitely/missing/path")
    }));
}

#[test]
fn binary_fmt_path_inside_workspace_member_scopes_to_that_member() {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    for name in ["a", "b"] {
        let crate_dir = temp.path().join(name);
        std::fs::create_dir_all(crate_dir.join("src")).expect("create src");
        std::fs::create_dir_all(crate_dir.join("i18n/en")).expect("create fallback locale");
        std::fs::write(
            crate_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write crate manifest");
        std::fs::write(crate_dir.join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
        std::fs::write(
            crate_dir.join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n config");
        std::fs::write(
            crate_dir.join(format!("i18n/en/{name}.ftl")),
            "hello = Hello\n",
        )
        .expect("write ftl");
    }

    let nested_member_path = temp
        .path()
        .join("a/src")
        .to_str()
        .expect("nested path")
        .to_string();
    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            &nested_member_path,
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    let files = json["files"].as_array().expect("files array");
    assert_eq!(files.len(), 1);
    let path = files[0]["path"].as_str().expect("file path");
    assert_eq!(path, "a/i18n/en/a.ftl");
    assert!(
        !path.contains(temp.path().to_string_lossy().as_ref()),
        "fmt JSON file paths should be relative: {path}"
    );

    #[cfg(unix)]
    {
        let outside = fixtures::tempdir();
        let symlinked_member_path = temp.path().join("a/src/external");
        std::os::unix::fs::symlink(outside.path(), &symlinked_member_path)
            .expect("create symlink inside member");
        let symlinked_member_path = symlinked_member_path
            .to_str()
            .expect("symlinked member path")
            .to_string();
        let output = Command::cargo_bin("cargo-es-fluent")
            .expect("binary exists")
            .args([
                "es-fluent",
                "fmt",
                "--path",
                &symlinked_member_path,
                "--output",
                "json",
            ])
            .assert()
            .success()
            .stderr(predicate::str::is_empty())
            .get_output()
            .stdout
            .clone();

        let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
        let files = json["files"].as_array().expect("files array");
        assert_eq!(files.len(), 1);
        let path = files[0]["path"].as_str().expect("file path");
        assert_eq!(path, "a/i18n/en/a.ftl");
    }

    let nested_member_file = temp
        .path()
        .join("a/src/lib.rs")
        .to_str()
        .expect("nested file path")
        .to_string();
    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            &nested_member_file,
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    let files = json["files"].as_array().expect("files array");
    assert_eq!(files.len(), 1);
    let path = files[0]["path"].as_str().expect("file path");
    assert_eq!(path, "a/i18n/en/a.ftl");
    assert!(
        !path.contains(temp.path().to_string_lossy().as_ref()),
        "fmt JSON file paths should be relative: {path}"
    );

    let workspace_manifest = temp
        .path()
        .join("Cargo.toml")
        .to_str()
        .expect("workspace manifest path")
        .to_string();
    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            &workspace_manifest,
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    let files = json["files"].as_array().expect("files array");
    assert_eq!(files.len(), 2);
    let paths = files
        .iter()
        .map(|file| file["path"].as_str().expect("file path"))
        .collect::<Vec<_>>();
    assert!(paths.contains(&"a/i18n/en/a.ftl"));
    assert!(paths.contains(&"b/i18n/en/b.ftl"));
    assert!(
        paths
            .iter()
            .all(|path| !path.contains(temp.path().to_string_lossy().as_ref())),
        "fmt JSON file paths should be relative: {paths:?}"
    );

    std::fs::create_dir_all(temp.path().join("tools")).expect("create workspace subdir");
    let workspace_subdir = temp
        .path()
        .join("tools")
        .to_str()
        .expect("workspace subdir")
        .to_string();
    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            &workspace_subdir,
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(json["files"].as_array().is_some_and(Vec::is_empty));
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("no crates with i18n.toml were found"))
    );
}

#[test]
fn binary_fmt_dry_run_json_reports_preview_mode_without_writing() {
    let temp = fixtures::create_workspace();
    let ftl_path = temp.path().join("i18n/en/test-app.ftl");
    std::fs::write(&ftl_path, "z-last = Z\na-first = A\n").expect("write unsorted ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--dry-run",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["formatted_count"], 1);
    assert_eq!(json["files"][0]["changed"], true);
    assert_eq!(
        std::fs::read_to_string(&ftl_path).expect("read ftl"),
        "z-last = Z\na-first = A\n"
    );
}

#[test]
fn binary_fmt_reports_binary_only_crate_as_notice_without_skipping() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create i18n");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"bin-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write Cargo.toml");
    std::fs::write(temp.path().join("src/main.rs"), "fn main() {}\n").expect("write main");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n.toml");
    let ftl_path = temp.path().join("i18n/en/bin-app.ftl");
    std::fs::write(&ftl_path, "z-last = Z\na-first = A\n").expect("write unsorted ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Notice bin-app (missing library target)",
        ))
        .stdout(predicate::str::contains("Skipping bin-app").not())
        .stdout(predicate::str::contains("Formatted:"));

    assert_eq!(
        std::fs::read_to_string(&ftl_path).expect("read formatted ftl"),
        "a-first = A\nz-last = Z\n"
    );
}

#[test]
fn binary_fmt_all_json_reports_noncanonical_locale_directory() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::create_dir_all(temp.path().join("i18n/en-us")).expect("create bad locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"fmt-locale\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write Cargo.toml");
    std::fs::write(temp.path().join("src/main.rs"), "fn main() {}\n").expect("write main");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n.toml");
    std::fs::write(
        temp.path().join("i18n/en/fmt-locale.ftl"),
        "hello = Hello\n",
    )
    .expect("write fallback ftl");
    std::fs::write(
        temp.path().join("i18n/en-us/fmt-locale.ftl"),
        "hello = Hello\n",
    )
    .expect("write bad locale ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all-locales",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("en-us") && message.contains("en-US"))
    );
}

#[test]
fn binary_fmt_rejects_configured_assets_dir_outside_crate() {
    let temp = fixtures::tempdir();
    let outside_name = format!(
        "{}-configured-outside-assets",
        temp.path()
            .file_name()
            .expect("temp name")
            .to_string_lossy()
    );
    let outside = temp
        .path()
        .parent()
        .expect("temp parent")
        .join(&outside_name);
    let outside_assets = outside.join("i18n/en");
    let outside_ftl = outside_assets.join("asset-config-escape.ftl");
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(&outside_assets).expect("create outside assets");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"asset-config-escape\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        format!("fallback_language = \"en\"\nassets_dir = \"../{outside_name}/i18n\"\n"),
    )
    .expect("write escaping i18n config");
    std::fs::write(&outside_ftl, "b = B\na = A\n").expect("write outside ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid assets_dir"))
        .stderr(predicate::str::contains("crate root"));

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"][0].as_str().is_some_and(
        |message| message.contains("Invalid assets_dir") && message.contains("crate root")
    ));

    let outside_content = std::fs::read_to_string(&outside_ftl).expect("read outside ftl");
    assert_eq!(outside_content, "b = B\na = A\n");
    let _ = std::fs::remove_dir_all(outside);
}

#[cfg(unix)]
#[test]
fn binary_fmt_rejects_symlinked_assets_dir_outside_crate() {
    let temp = fixtures::tempdir();
    let outside = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(outside.path().join("en")).expect("create outside fallback");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"symlink-assets\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        outside.path().join("en/symlink-assets.ftl"),
        "z = Z\na = A\n",
    )
    .expect("write outside ftl");
    std::os::unix::fs::symlink(outside.path(), temp.path().join("i18n"))
        .expect("create assets symlink");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"][0].as_str().is_some_and(
        |message| message.contains("Invalid assets_dir") && message.contains("crate root")
    ));

    let outside_content = std::fs::read_to_string(outside.path().join("en/symlink-assets.ftl"))
        .expect("read outside ftl");
    assert_eq!(outside_content, "z = Z\na = A\n");
}

#[cfg(unix)]
#[test]
fn binary_fmt_rejects_symlinked_assets_dir_inside_crate_without_formatting_target() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("real-i18n/en")).expect("create real fallback");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"symlink-assets-inside\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    let ftl_path = temp.path().join("real-i18n/en/symlink-assets-inside.ftl");
    std::fs::write(&ftl_path, "z = Z\na = A\n").expect("write unsorted ftl");
    std::os::unix::fs::symlink(temp.path().join("real-i18n"), temp.path().join("i18n"))
        .expect("create in-crate assets symlink");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"][0].as_str().is_some_and(
        |message| message.contains("Invalid assets_dir") && message.contains("not symlinks")
    ));

    let content = std::fs::read_to_string(&ftl_path).expect("read ftl");
    assert_eq!(content, "z = Z\na = A\n");
}

#[cfg(unix)]
#[test]
fn binary_fmt_rejects_symlinked_fallback_locale_without_formatting_external_file() {
    let temp = fixtures::tempdir();
    let outside = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n")).expect("create assets dir");
    std::fs::create_dir_all(outside.path().join("en")).expect("create outside fallback");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    let outside_ftl = outside.path().join("en/test-app.ftl");
    std::fs::write(&outside_ftl, "z = Z\na = A\n").expect("write outside ftl");
    std::os::unix::fs::symlink(outside.path().join("en"), temp.path().join("i18n/en"))
        .expect("create fallback locale symlink");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["errors"][0].as_str().is_some_and(
            |message| message.contains("FTL directories") && message.contains("symlinks")
        )
    );

    let outside_content = std::fs::read_to_string(&outside_ftl).expect("read outside ftl");
    assert_eq!(outside_content, "z = Z\na = A\n");
}

#[test]
fn binary_fmt_json_rejects_fallback_locale_path_as_file() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n")).expect("create assets dir");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/en"), "not a directory\n").expect("write fallback file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["files"][0]["error"]
            .as_str()
            .is_some_and(|error| error.contains("not a directory"))
    );
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|error| error.contains("not a directory") && error.contains("i18n/en"))
    );
    assert!(
        !json["files"][0]["path"]
            .as_str()
            .is_some_and(|path| path.contains(temp.path().to_string_lossy().as_ref())),
        "fmt JSON file paths should be workspace-relative"
    );
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|error| error.contains(temp.path().to_string_lossy().as_ref())),
        "fmt JSON errors should not include absolute workspace paths"
    );
}

#[test]
fn binary_fmt_json_reports_file_parse_errors_in_top_level_errors() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        temp.path().join("i18n/en/test-app.ftl"),
        "hello = { $name\n",
    )
    .expect("write invalid ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["files"][0]["error"]
            .as_str()
            .is_some_and(|error| error.contains("parse errors"))
    );
    assert!(json["errors"][0].as_str().is_some_and(
        |error| error.contains("parse errors") && error.contains("i18n/en/test-app.ftl")
    ));
    assert_eq!(json["files"][0]["path"], "i18n/en/test-app.ftl");
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|error| error.contains(temp.path().to_string_lossy().as_ref())),
        "fmt JSON parse errors should not include absolute workspace paths"
    );
}

#[test]
fn binary_fmt_json_rejects_ftl_path_as_directory() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en/test-app.ftl"))
        .expect("create ftl directory");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("Expected FTL path to be a file")
                && message.contains("test-app.ftl"))
    );
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains(temp.path().to_string_lossy().as_ref())),
        "fmt JSON setup errors should not include absolute workspace paths"
    );
}

#[test]
fn binary_fmt_json_rejects_assets_dir_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["files"][0]["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("i18n"))
    );
    assert!(
        json["files"][0]["error"]
            .as_str()
            .is_some_and(|error| error.contains("assets_dir for test-app"))
    );
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|error| error.contains("assets_dir for test-app"))
    );
}

#[test]
fn binary_fmt_all_json_rejects_locale_named_asset_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all-locales",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["files"][0]["error"]
            .as_str()
            .is_some_and(|error| error.contains("locale directory 'fr'"))
    );
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|error| error.contains("locale directory 'fr'"))
    );
}

#[test]
fn binary_fmt_json_leaves_all_files_unchanged_with_mixed_workspace_errors() {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    for name in ["a", "b"] {
        std::fs::create_dir_all(temp.path().join(format!("{name}/src"))).expect("create src");
        std::fs::create_dir_all(temp.path().join(format!("{name}/i18n/en"))).expect("create en");
        std::fs::write(
            temp.path().join(format!("{name}/Cargo.toml")),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write manifest");
        std::fs::write(
            temp.path().join(format!("{name}/src/lib.rs")),
            "pub fn marker() {}\n",
        )
        .expect("write lib");
        std::fs::write(
            temp.path().join(format!("{name}/i18n.toml")),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write config");
    }
    std::fs::write(temp.path().join("a/i18n/en/a.ftl"), "z = Z\na = A\n")
        .expect("write unsorted ftl");
    let unsorted_path = temp.path().join("a/i18n/en/a.ftl");
    let before = std::fs::read_to_string(&unsorted_path).expect("read unsorted FTL");
    std::fs::write(temp.path().join("b/i18n/en/b.ftl"), "hello = Hello\n")
        .expect("write fallback ftl");
    std::fs::write(temp.path().join("b/i18n/fr"), "not a directory\n").expect("write locale file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all-locales",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["formatted_count"], 0);
    assert_eq!(json["error_count"], 1);
    assert!(
        json["files"]
            .as_array()
            .is_some_and(|files| files.iter().all(|file| file["changed"] == false))
    );
    assert!(
        json["files"]
            .as_array()
            .is_some_and(|files| files.iter().any(|file| file["error"]
                .as_str()
                .is_some_and(|error| error.contains("locale directory 'fr'"))))
    );
    assert_eq!(
        std::fs::read_to_string(&unsorted_path).expect("read FTL after failed format"),
        before,
        "a planning error must abort the complete formatting transaction"
    );
    assert!(json["errors"][0].as_str().is_some_and(
        |error| error.contains("locale directory 'fr'") && error.contains("b/i18n/fr")
    ));
}

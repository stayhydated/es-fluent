use crate::*;

#[test]
fn binary_tree_treats_a_closed_stdout_pipe_as_clean_shutdown() {
    let temp = fixtures::create_workspace();
    let mut ftl = String::new();
    for index in 0..20_000 {
        ftl.push_str(&format!("message-{index} = Message {index}\n"));
    }
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), ftl).expect("write large FTL fixture");

    for (output_args, expected_first_line) in [
        (&[][..], "Fluent FTL Tree"),
        (&["--output", "json"][..], "{"),
    ] {
        let mut args = vec![
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all-locales",
            "--e2e",
        ];
        args.extend_from_slice(output_args);
        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_cargo-es-fluent"))
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn cargo-es-fluent");

        let stdout = child.stdout.take().expect("piped stdout");
        let mut reader = std::io::BufReader::new(stdout);
        let mut first_line = String::new();
        reader
            .read_line(&mut first_line)
            .expect("read first output line");
        assert_eq!(first_line.trim_end(), expected_first_line);
        drop(reader);

        let mut stderr = child.stderr.take().expect("piped stderr");
        let status = child.wait().expect("wait for cargo-es-fluent");
        let mut stderr_text = String::new();
        stderr
            .read_to_string(&mut stderr_text)
            .expect("read stderr");

        assert!(status.success(), "unexpected exit status: {status}");
        assert!(
            !stderr_text.contains("Broken pipe") && !stderr_text.contains("panicked"),
            "unexpected stderr: {stderr_text}"
        );
    }
}

#[test]
fn binary_tree_json_rejects_fallback_locale_path_as_file() {
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
            "tree",
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
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["errors"][0]["crate_name"], "test-app");
    assert!(
        json["errors"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("locale directory 'en'")
                && message.contains("not a directory"))
    );
}

#[test]
fn binary_tree_all_json_rejects_missing_fallback_locale_directory() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create non-fallback locale");
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
        temp.path().join("i18n/fr/test-app.ftl"),
        "hello = Bonjour\n",
    )
    .expect("write non-fallback ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
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
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["crates"], Value::Array(Vec::new()));
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["errors"][0]["crate_name"], "test-app");
    assert!(
        json["errors"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("locale directory 'en'")
                && message.contains("missing or not a directory"))
    );
}

#[test]
fn binary_tree_json_rejects_ftl_path_as_directory() {
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
            "tree",
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
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["errors"][0]["crate_name"], "test-app");
    assert!(
        json["errors"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("Expected FTL path to be a file")
                && message.contains("test-app.ftl"))
    );
}

#[test]
fn binary_tree_text_rust_links_rejects_ftl_path_directory_before_runner() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en/test-app.ftl"))
        .expect("create ftl directory");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "this is not rust\n").expect("write bad lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .env("FORCE_HYPERLINK", "1")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--link-mode",
            "rust",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Expected FTL path to be a file"))
        .stderr(predicate::str::contains("test-app.ftl"))
        .stderr(predicate::str::contains("could not compile").not());

    assert!(
        !temp.path().join(".es-fluent").exists(),
        "tree should reject invalid FTL paths before runner metadata"
    );
    assert!(
        !temp.path().join("target").exists(),
        "tree should reject invalid FTL paths before Cargo runs"
    );
}

#[test]
fn binary_tree_json_rejects_assets_dir_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
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
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["errors"][0]["crate_name"], "test-app");
    assert!(
        json["errors"][0]["message"].as_str().is_some_and(
            |message| message.contains("assets_dir") && message.contains("not a directory")
        )
    );
}

#[cfg(unix)]
#[test]
fn binary_tree_json_rejects_symlinked_assets_dir() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::create_dir_all(temp.path().join("real-i18n/en")).expect("create real locale");
    std::fs::write(
        temp.path().join("real-i18n/en/test-app.ftl"),
        "hello = Hello\n",
    )
    .expect("write real ftl");
    std::os::unix::fs::symlink(temp.path().join("real-i18n"), temp.path().join("i18n"))
        .expect("create assets symlink");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
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
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["errors"][0]["crate_name"], "workspace");
    assert!(
        json["errors"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("assets_dir") && message.contains("symlink"))
    );
}

#[cfg(unix)]
#[test]
fn binary_tree_json_rejects_symlinked_fallback_locale_dir() {
    let temp = fixtures::create_workspace();
    let outside = fixtures::tempdir();
    std::fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");
    std::fs::create_dir_all(outside.path().join("en")).expect("create outside locale");
    std::fs::write(outside.path().join("en/test-app.ftl"), "hello = Hello\n")
        .expect("write outside ftl");
    std::os::unix::fs::symlink(outside.path().join("en"), temp.path().join("i18n/en"))
        .expect("create fallback locale symlink");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
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
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["errors"][0]["crate_name"], "test-app");
    assert!(
        json["errors"][0]["message"]
            .as_str()
            .is_some_and(
                |message| message.contains("locale directory 'en'") && message.contains("symlink")
            )
    );
}

#[test]
fn binary_tree_all_json_rejects_locale_named_asset_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
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
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["errors"][0]["crate_name"], "test-app");
    assert!(
        json["errors"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("locale directory 'fr'")
                && message.contains("not a directory"))
    );
}

#[test]
fn binary_tree_json_honors_attribute_and_variable_filters() {
    let temp = fixtures::create_workspace();
    std::fs::write(
        temp.path().join("i18n/en/test-app.ftl"),
        "hello = Hello { $name }\n    .title = Title { $name }\n",
    )
    .expect("write ftl with attributes and variables");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
            "--no-attributes",
            "--no-variables",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    let entry = &json["crates"][0]["locales"][0]["files"][0]["entries"][0];
    assert_eq!(entry["id"], "hello");
    assert!(
        entry["attributes"]
            .as_array()
            .expect("attributes")
            .is_empty()
    );
    assert!(entry["variables"].as_array().expect("variables").is_empty());
}

#[test]
fn binary_tree_json_no_attributes_hides_attribute_only_variables() {
    let temp = fixtures::create_workspace();
    std::fs::write(
        temp.path().join("i18n/en/test-app.ftl"),
        "hello = Hello { $name }\n    .title = Title { $title }\n",
    )
    .expect("write ftl with distinct value and attribute variables");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
            "--no-attributes",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    let entry = &json["crates"][0]["locales"][0]["files"][0]["entries"][0];
    assert_eq!(entry["id"], "hello");
    assert!(
        entry["attributes"]
            .as_array()
            .expect("attributes")
            .is_empty()
    );
    assert_eq!(entry["variables"], serde_json::json!(["name"]));
}

#[test]
fn binary_tree_json_reports_ftl_parse_errors_and_fails() {
    let temp = fixtures::create_workspace();
    std::fs::write(
        temp.path().join("i18n/en/test-app.ftl"),
        "hello = { $name\n",
    )
    .expect("write invalid ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
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
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    let file = &json["crates"][0]["locales"][0]["files"][0];
    assert_eq!(file["parse_error"], true);
    assert!(file["entries"].as_array().expect("entries").is_empty());
    assert_eq!(json["errors"][0]["crate_name"], "test-app");
    assert!(
        json["errors"][0]["message"]
            .as_str()
            .is_some_and(
                |message| message.contains("failed to parse FTL file 'test-app.ftl'")
                    && message.contains("Fluent parse errors")
            )
    );

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("failed to parse FTL file 'test-app.ftl'")
                .and(predicate::str::contains("Fluent parse errors")),
        );
}

#[test]
fn binary_tree_json_is_file_only_without_link_mode() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create i18n");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "this is not rust\n").expect("write bad lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n").expect("write ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 0);
    assert_eq!(
        json["crates"][0]["locales"][0]["files"][0]["entries"][0]["id"],
        "hello"
    );
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "tree JSON should not prepare runner metadata"
    );
    assert!(
        !temp.path().join("target").exists(),
        "tree JSON should not run Cargo for Rust links"
    );
}

#[test]
fn binary_tree_json_rejects_link_mode_before_workspace_discovery() {
    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            "/definitely/missing/path",
            "--link-mode",
            "ftl",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["errors"][0]["crate_name"], "workspace");
    let message = json["errors"][0]["message"]
        .as_str()
        .expect("tree error message");
    assert!(message.contains("--link-mode cannot be used with --output json"));
    assert!(
        !message.contains("/definitely/missing/path"),
        "tree should reject the output-mode conflict before workspace discovery"
    );
}

#[test]
fn binary_tree_text_shows_empty_locale_directories() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"empty-tree\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("empty-tree"))
        .stdout(predicate::str::contains("en"));
}

#[test]
fn binary_tree_text_rust_mode_inspects_binary_only_crate_without_runner() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"binary-only-tree\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"binary-only-tree\"\npath = \"src/main.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/main.rs"), "fn main() {}\n").expect("write main");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        temp.path().join("i18n/en/binary-only-tree.ftl"),
        "hello = Hello\n",
    )
    .expect("write ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .env("FORCE_HYPERLINK", "1")
        .args([
            "es-fluent",
            "tree",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--link-mode",
            "rust",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("binary-only-tree"))
        .stdout(predicate::str::contains("hello"));

    assert!(
        !temp.path().join(".es-fluent").exists(),
        "tree should not prepare runner metadata when no selected crate has a library target"
    );
    assert!(
        !temp.path().join("target").exists(),
        "tree should not run Cargo when no selected crate has a library target"
    );
}

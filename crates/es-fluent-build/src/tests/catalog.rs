#[serial_test::serial(manifest)]
#[test]
fn track_i18n_assets_reads_config_and_assets_path() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("i18n")).expect("create assets dir");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");

    with_manifest_env(Some(temp.path()), || {
        track_i18n_assets();
    });
}

#[serial_test::serial(manifest)]
#[test]
fn inventory_runner_initializes_catalog_without_parsing_fallback_ftl() {
    let temp = tempfile::tempdir().expect("tempdir");
    let locale_dir = temp.path().join("i18n/en");
    fs::create_dir_all(&locale_dir).expect("create locale dir");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    fs::write(locale_dir.join("test-package.ftl"), "broken = {\n").expect("write malformed FTL");

    temp_env::with_var(INVENTORY_RUNNER_ENV, Some("1"), || {
        with_manifest_env(Some(temp.path()), track_i18n_assets);
    });

    assert_eq!(
        fs::read(
            temp.path()
                .join("build-output")
                .join(FALLBACK_CATALOG_FILE_NAME)
        )
        .expect("read catalog"),
        b""
    );
}

#[serial_test::serial(manifest)]
#[test]
fn crate_root_assets_ignore_project_directories_when_building_catalog() {
    let temp = tempfile::tempdir().expect("tempdir");
    let locale_dir = temp.path().join("en");
    fs::create_dir_all(locale_dir.join("test-package")).expect("create namespace dir");
    fs::create_dir(temp.path().join("src")).expect("create src dir");
    fs::create_dir(temp.path().join("target")).expect("create target dir");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \".\"\n",
    )
    .expect("write config");
    fs::write(
        locale_dir.join("test-package.ftl"),
        "hello = Hello from the crate root\n",
    )
    .expect("write fallback resource");
    fs::write(locale_dir.join("test-package/ui.ftl"), "title = Root UI\n")
        .expect("write fallback namespace resource");

    with_manifest_env(Some(temp.path()), track_i18n_assets);

    let catalog = fs::read(
        temp.path()
            .join("build-output")
            .join(FALLBACK_CATALOG_FILE_NAME),
    )
    .expect("read catalog");
    assert!(
        catalog
            .windows(b"test-package\thello\n".len())
            .any(|window| { window == b"test-package\thello\n" })
    );
    assert!(
        catalog
            .windows(b"test-package\ttitle\n".len())
            .any(|window| { window == b"test-package\ttitle\n" })
    );
}

#[serial_test::serial(manifest)]
#[test]
fn normalized_crate_root_assets_ignore_project_directories_when_building_catalog() {
    let temp = tempfile::tempdir().expect("tempdir");
    let locale_dir = temp.path().join("en");
    fs::create_dir(temp.path().join("locale")).expect("create normalized path component");
    fs::create_dir(temp.path().join("src")).expect("create src dir");
    fs::create_dir(temp.path().join("target")).expect("create target dir");
    fs::create_dir_all(&locale_dir).expect("create locale dir");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"locale/..\"\n",
    )
    .expect("write config");
    fs::write(locale_dir.join("test-package.ftl"), "hello = Hello\n")
        .expect("write fallback resource");

    with_manifest_env(Some(temp.path()), track_i18n_assets);

    let catalog = fs::read(
        temp.path()
            .join("build-output")
            .join(FALLBACK_CATALOG_FILE_NAME),
    )
    .expect("read catalog");
    assert!(
        catalog
            .windows(b"test-package\thello\n".len())
            .any(|window| { window == b"test-package\thello\n" })
    );
}

#[cfg(unix)]
#[serial_test::serial(manifest)]
#[test]
fn track_i18n_assets_rejects_symlinked_fallback_locale_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside tempdir");
    fs::create_dir_all(temp.path().join("i18n")).expect("create assets dir");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    fs::write(
        outside.path().join("test-package.ftl"),
        "hello = Outside fallback\n",
    )
    .expect("write outside fallback resource");
    std::os::unix::fs::symlink(outside.path(), temp.path().join("i18n/en"))
        .expect("create fallback locale symlink");

    let panic = with_manifest_env(Some(temp.path()), || {
        std::panic::catch_unwind(track_i18n_assets)
    })
    .expect_err("symlinked fallback locale should be rejected");
    let message = panic_message(panic.as_ref()).unwrap_or_default();
    assert!(
        message.contains("symlink"),
        "unexpected panic message: {message}"
    );
}

#[cfg(unix)]
#[serial_test::serial(manifest)]
#[test]
fn track_i18n_assets_rejects_symlinked_fallback_resource() {
    let temp = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside tempdir");
    fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    let outside_resource = outside.path().join("test-package.ftl");
    fs::write(&outside_resource, "hello = Outside fallback\n")
        .expect("write outside fallback resource");
    std::os::unix::fs::symlink(
        &outside_resource,
        temp.path().join("i18n/en/test-package.ftl"),
    )
    .expect("create fallback resource symlink");

    let panic = with_manifest_env(Some(temp.path()), || {
        std::panic::catch_unwind(track_i18n_assets)
    })
    .expect_err("symlinked fallback resource should be rejected");
    let message = panic_message(panic.as_ref()).unwrap_or_default();
    assert!(
        message.contains("symlink"),
        "unexpected panic message: {message}"
    );
}

#[cfg(unix)]
#[serial_test::serial(manifest)]
#[test]
fn crate_root_assets_reject_symlinked_namespace_resource() {
    let temp = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside tempdir");
    fs::create_dir_all(temp.path().join("en/test-package")).expect("create fallback namespace");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \".\"\n",
    )
    .expect("write config");
    let outside_resource = outside.path().join("test-package.ftl");
    fs::write(&outside_resource, "hello = Outside fallback\n")
        .expect("write outside fallback resource");
    std::os::unix::fs::symlink(
        &outside_resource,
        temp.path().join("en/test-package/ui.ftl"),
    )
    .expect("create crate-root namespace resource symlink");

    let panic = with_manifest_env(Some(temp.path()), || {
        std::panic::catch_unwind(track_i18n_assets)
    })
    .expect_err("crate-root namespace resource symlink should be rejected");
    let message = panic_message(panic.as_ref()).unwrap_or_default();
    assert!(
        message.contains("symlink"),
        "unexpected panic message: {message}"
    );
}

#[serial_test::serial(manifest)]
#[test]
fn inventory_mode_change_rebuilds_strict_catalog() {
    let temp = tempfile::tempdir().expect("tempdir");
    let crate_dir = temp.path().join("inventory-cache");
    let locale_dir = crate_dir.join("i18n/en");
    let target_dir = temp.path().join("target");
    let workspace_crates = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates directory");

    fs::create_dir_all(crate_dir.join("src")).expect("create src dir");
    fs::create_dir_all(&locale_dir).expect("create locale dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "inventory-cache"
version = "0.1.0"
edition = "2024"

[dependencies]
es-fluent = {{ path = "{}" }}

[build-dependencies]
es-fluent-build = {{ path = "{}" }}
"#,
            toml_path(&workspace_crates.join("es-fluent")),
            toml_path(Path::new(env!("CARGO_MANIFEST_DIR"))),
        ),
    )
    .expect("write Cargo.toml");
    fs::write(crate_dir.join("build.rs"), BUILD_TRACK_I18N_SOURCE).expect("write build script");
    fs::write(
        crate_dir.join("src/lib.rs"),
        "#[derive(es_fluent::EsFluent)]\npub struct MissingValue;\n",
    )
    .expect("write lib.rs");
    fs::write(
        crate_dir.join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    fs::write(
        locale_dir.join("inventory-cache.ftl"),
        "present = Present\n",
    )
    .expect("write fallback resource");

    let inventory = cargo_check_output_with_inventory(&crate_dir, &target_dir, true);
    assert!(
        inventory.status.success(),
        "inventory build should succeed: {}",
        String::from_utf8_lossy(&inventory.stderr)
    );

    let application = cargo_check_output_with_inventory(&crate_dir, &target_dir, false);
    assert!(
        !application.status.success(),
        "application build should re-run strict catalog validation"
    );
    assert!(
        String::from_utf8_lossy(&application.stderr)
            .contains("missing fallback Fluent message `missing_value`"),
        "strict validation should report the missing key: {}",
        String::from_utf8_lossy(&application.stderr)
    );
}

#[serial_test::serial(manifest)]
#[test]
fn fallback_str_policy_still_rejects_malformed_catalog() {
    let temp = tempfile::tempdir().expect("tempdir");
    let locale_dir = temp.path().join("i18n/en");
    fs::create_dir_all(&locale_dir).expect("create locale dir");
    fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\nmissing_message_policy = \"fallback-str\"\n",
        )
        .expect("write config");
    fs::write(locale_dir.join("test-package.ftl"), "broken = {\n").expect("write malformed FTL");

    let result = with_manifest_env(Some(temp.path()), || {
        std::panic::catch_unwind(track_i18n_assets)
    });
    assert!(result.is_err());
}

#[serial_test::serial(manifest)]
#[test]
fn track_i18n_assets_does_not_create_stamp_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let i18n_dir = temp.path().join("i18n");
    fs::create_dir_all(i18n_dir.join("en")).expect("create en dir");
    fs::create_dir_all(i18n_dir.join("fr")).expect("create fr dir");
    fs::write(i18n_dir.join("en").join("main.ftl"), "hello = Hello").expect("write ftl");
    fs::write(i18n_dir.join("fr").join("main.ftl"), "hello = Bonjour").expect("write ftl");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");

    with_manifest_env(Some(temp.path()), || {
        track_i18n_assets();
    });

    let stamp = temp.path().join(".es-fluent").join("locales.stamp");
    assert!(!stamp.exists(), "stamp file should not be created");
}

#[serial_test::serial(manifest)]
#[test]
fn track_i18n_assets_rejects_external_assets_dir_without_stamp_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let crate_dir = temp.path().join("my-crate");
    let assets_dir = temp.path().join("assets").join("i18n");
    fs::create_dir_all(&crate_dir).expect("create crate dir");
    fs::create_dir_all(assets_dir.join("en")).expect("create en dir");
    fs::write(assets_dir.join("en").join("main.ftl"), "hello = Hello").expect("write ftl");
    fs::write(
        crate_dir.join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"../assets/i18n\"\n",
    )
    .expect("write config");

    let panic = with_manifest_env(Some(&crate_dir), || {
        std::panic::catch_unwind(track_i18n_assets)
    })
    .expect_err("external assets_dir should be rejected");
    let message = panic_message(panic.as_ref()).unwrap_or_default();
    assert!(
        message.contains("Failed to read i18n.toml configuration")
            && message.contains("InvalidAssetsDir"),
        "unexpected panic message: {message}"
    );

    let stamp = temp
        .path()
        .join("assets")
        .join(".es-fluent")
        .join("locales.stamp");
    assert!(
        !stamp.exists(),
        "stamp file should not be written next to external assets dir"
    );
}

#[serial_test::serial(manifest)]
#[test]
fn track_i18n_assets_rebuilds_when_locale_folder_deleted() {
    let temp = tempfile::tempdir().expect("tempdir");
    let crate_dir = temp.path().join("sample-crate");
    let i18n_dir = crate_dir.join("i18n");
    let src_dir = crate_dir.join("src");
    let trace_file = temp.path().join("trace.log");
    let target_dir = temp.path().join("target");

    fs::create_dir_all(&src_dir).expect("create src dir");
    fs::create_dir_all(i18n_dir.join("en")).expect("create en dir");
    fs::create_dir_all(i18n_dir.join("fr")).expect("create fr dir");
    fs::write(i18n_dir.join("en").join("main.ftl"), "hello = Hello").expect("write en ftl");
    fs::write(i18n_dir.join("fr").join("main.ftl"), "hello = Bonjour").expect("write fr ftl");

    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "sample-crate"
version = "0.1.0"
edition = "2024"

[build-dependencies]
es-fluent-build = {{ path = "{}" }}
"#,
            toml_path(Path::new(env!("CARGO_MANIFEST_DIR")))
        ),
    )
    .expect("write Cargo.toml");

    fs::write(crate_dir.join("build.rs"), BUILD_SCRIPT_SOURCE).expect("write build.rs");
    fs::write(src_dir.join("lib.rs"), "pub fn value() -> u8 { 1 }\n").expect("write lib.rs");
    fs::write(
        crate_dir.join("i18n.toml"),
        "fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");

    run_cargo_check(&crate_dir, &target_dir, &trace_file);
    assert_eq!(trace_lines(&trace_file), 1, "initial build should run once");

    run_cargo_check(&crate_dir, &target_dir, &trace_file);
    assert_eq!(trace_lines(&trace_file), 1, "no changes should not rebuild");

    fs::remove_dir_all(i18n_dir.join("fr")).expect("delete locale folder");
    run_cargo_check(&crate_dir, &target_dir, &trace_file);

    assert_eq!(
        trace_lines(&trace_file),
        2,
        "deleting a locale folder should trigger rebuild"
    );
}

#[serial_test::serial(manifest)]
#[test]
fn track_i18n_assets_panics_without_manifest_dir() {
    let panic = with_manifest_env(None, || std::panic::catch_unwind(track_i18n_assets));
    assert!(panic.is_err());
}

#[serial_test::serial(manifest)]
#[test]
fn configured_derive_without_build_helper_reports_actionable_catalog_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    let crate_dir = temp.path().join("missing-build-helper");
    let src_dir = crate_dir.join("src");
    let locale_dir = crate_dir.join("i18n/en");
    let target_dir = temp.path().join("target");
    let es_fluent_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates directory")
        .join("es-fluent");

    fs::create_dir_all(&src_dir).expect("create src dir");
    fs::create_dir_all(&locale_dir).expect("create locale dir");
    fs::write(
            crate_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"missing-build-helper\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nes-fluent = {{ path = \"{}\" }}\n",
                toml_path(&es_fluent_dir)
            ),
        )
        .expect("write Cargo.toml");
    fs::write(
        src_dir.join("lib.rs"),
        "#[derive(es_fluent::EsFluent)]\npub enum Greeting { Hello, Goodbye }\n",
    )
    .expect("write lib.rs");
    fs::write(
        crate_dir.join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n.toml");
    fs::write(
        locale_dir.join("missing-build-helper.ftl"),
        "greeting-Hello = Hello\ngreeting-Goodbye = Goodbye\n",
    )
    .expect("write fallback FTL");

    let output = cargo_check_output(&crate_dir, &target_dir, &[]);
    assert!(
        !output.status.success(),
        "build should require catalog setup"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    for expected in [
        "es-fluent fallback catalog is unavailable",
        "package `missing-build-helper`",
        "add `es-fluent-build` under `[build-dependencies]`",
        "es_fluent_build::track_i18n_assets()",
    ] {
        assert!(
            stderr.contains(expected),
            "expected {expected:?} in stderr: {stderr}"
        );
    }
    assert_eq!(
        stderr
            .matches("es-fluent fallback catalog is unavailable")
            .count(),
        1,
        "setup diagnostics should be emitted once per derive: {stderr}"
    );
    assert!(!stderr.contains("OUT_DIR"));
}

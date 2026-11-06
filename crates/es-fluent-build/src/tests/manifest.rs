fn write_package(root: &Path, assets_dir: &str, locales_root: &str) {
    fs::create_dir_all(root.join(locales_root).join("en/test-package")).expect("create en dir");
    fs::create_dir_all(root.join(locales_root).join("fr")).expect("create fr dir");
    fs::write(
        root.join("i18n.toml"),
        format!(
            "fallback_language = \"en\"\nassets_dir = \"{assets_dir}\"\ndomains = [\"extra\"]\n"
        ),
    )
    .expect("write config");
    fs::write(
        root.join(locales_root).join("en/test-package.ftl"),
        "hello = Hello\n-brand = Brand\n",
    )
    .expect("write base resource");
    fs::write(
        root.join(locales_root).join("en/test-package/ui.ftl"),
        "title = Title\n",
    )
    .expect("write namespaced resource");
    fs::write(root.join(locales_root).join("en/extra.ftl"), "extra = Extra\n")
        .expect("write domain resource");
    fs::write(
        root.join(locales_root).join("fr/test-package.ftl"),
        "hello = Bonjour\n",
    )
    .expect("write fr resource");
}

fn collect_manifest(root: &Path) -> BuildManifest {
    let layout = ResolvedI18nLayout::from_manifest_dir(root).expect("layout");
    BuildManifest::collect(&layout, "test-package", true).expect("collect manifest")
}

#[test]
fn manifest_round_trips_shared_discovery() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_package(temp.path(), "i18n", "i18n");

    let manifest = collect_manifest(temp.path());
    let decoded = BuildManifest::from_bytes(&manifest.to_bytes()).expect("decode manifest");
    assert_eq!(decoded, manifest);

    let config = decoded.config().expect("config");
    assert_eq!(
        decoded.languages().expect("languages"),
        manifest::discover_languages(&config, temp.path()).expect("discover languages")
    );
    assert_eq!(
        decoded.resources().expect("resources"),
        manifest::discover_resources(&config, temp.path(), "test-package")
            .expect("discover resources")
    );

    let messages = decoded.fallback_messages().expect("fallback catalog");
    assert!(messages.contains("test-package", "hello"));
    assert!(messages.contains("test-package", "title"));
    assert!(messages.contains("extra", "extra"));
    assert!(!messages.contains("test-package", "brand"));
}

#[test]
fn manifest_bytes_do_not_depend_on_package_location() {
    let first = tempfile::tempdir().expect("first tempdir");
    let second = tempfile::tempdir().expect("second tempdir");
    write_package(first.path(), "i18n", "i18n");
    write_package(second.path(), "i18n", "i18n");

    assert_eq!(
        collect_manifest(first.path()).to_bytes(),
        collect_manifest(second.path()).to_bytes()
    );
}

#[test]
fn manifest_records_resource_discovery_errors_for_macros() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_package(temp.path(), "i18n", "i18n");
    fs::create_dir_all(temp.path().join("i18n/not a locale")).expect("create invalid locale");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    let layout = ResolvedI18nLayout::from_manifest_dir(temp.path()).expect("layout");

    let manifest = BuildManifest::collect(&layout, "test-package", false).expect("collect");
    let error = manifest.languages().expect_err("invalid locale directory");
    let expected = manifest::discover_languages(&layout.config, temp.path())
        .expect_err("discovery should report the invalid locale");
    assert_eq!(error, expected);
}

#[test]
fn manifest_rejects_another_format() {
    let error = BuildManifest::from_bytes(br#"{"format":0}"#).expect_err("format mismatch");

    assert!(
        error.contains("use matching es-fluent and es-fluent-build versions"),
        "unexpected error: {error}"
    );
}

#[test]
fn crate_root_assets_watch_locale_directories_instead_of_the_crate_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_package(temp.path(), ".", ".");
    fs::create_dir_all(temp.path().join("target/debug")).expect("create target dir");
    fs::create_dir_all(temp.path().join("src")).expect("create src dir");
    let layout = ResolvedI18nLayout::from_manifest_dir(temp.path()).expect("layout");

    assert_eq!(
        rerun_paths(&layout),
        vec![
            layout.config_path.clone(),
            layout.locale_dir("en"),
            layout.locale_dir("fr"),
        ]
    );
}

#[test]
fn nested_assets_watch_the_assets_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_package(temp.path(), "i18n", "i18n");
    let layout = ResolvedI18nLayout::from_manifest_dir(temp.path()).expect("layout");

    assert_eq!(
        rerun_paths(&layout),
        vec![layout.config_path.clone(), layout.assets_dir.clone()]
    );
}

#[serial_test::serial(manifest)]
#[test]
fn unchanged_inputs_keep_the_manifest_modification_time() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_package(temp.path(), "i18n", "i18n");
    let manifest_path = temp
        .path()
        .join("build-output")
        .join(BUILD_MANIFEST_FILE_NAME);

    with_manifest_env(Some(temp.path()), track_i18n_assets);
    let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1);
    fs::File::options()
        .write(true)
        .open(&manifest_path)
        .expect("open manifest")
        .set_modified(old)
        .expect("backdate manifest");

    with_manifest_env(Some(temp.path()), track_i18n_assets);
    assert_eq!(
        fs::metadata(&manifest_path)
            .expect("manifest metadata")
            .modified()
            .expect("modified time"),
        old
    );

    fs::write(
        temp.path().join("i18n/en/test-package.ftl"),
        "hello = Hello\ngoodbye = Goodbye\n",
    )
    .expect("add fallback message");
    with_manifest_env(Some(temp.path()), track_i18n_assets);
    assert_ne!(
        fs::metadata(&manifest_path)
            .expect("manifest metadata")
            .modified()
            .expect("modified time"),
        old
    );
}

#[serial_test::serial(manifest)]
#[test]
fn crate_root_assets_do_not_rerun_the_build_script_for_target_changes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let crate_dir = temp.path().join("sample-crate");
    let trace_file = temp.path().join("trace.log");
    // The target directory sits inside the crate root, as in a single-package
    // repository, so every build changes a directory under `assets_dir`.
    let target_dir = crate_dir.join("target");

    fs::create_dir_all(crate_dir.join("src")).expect("create src dir");
    fs::create_dir_all(crate_dir.join("en")).expect("create en dir");
    fs::write(crate_dir.join("en/sample-crate.ftl"), "hello = Hello\n").expect("write en ftl");
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
    fs::write(
        crate_dir.join("src/lib.rs"),
        "pub fn value() -> u8 { 1 }\n",
    )
    .expect("write lib.rs");
    fs::write(
        crate_dir.join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \".\"\n",
    )
    .expect("write config");

    run_cargo_check(&crate_dir, &target_dir, &trace_file);
    assert_eq!(trace_lines(&trace_file), 1, "initial build should run once");

    run_cargo_check(&crate_dir, &target_dir, &trace_file);
    assert_eq!(
        trace_lines(&trace_file),
        1,
        "target directory changes should not rerun the build script"
    );

    fs::write(
        crate_dir.join("en/sample-crate.ftl"),
        "hello = Hello\ngoodbye = Goodbye\n",
    )
    .expect("edit en ftl");
    run_cargo_check(&crate_dir, &target_dir, &trace_file);
    assert_eq!(
        trace_lines(&trace_file),
        2,
        "locale changes should rerun the build script"
    );
}

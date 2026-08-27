#![doc = include_str!("../README.md")]
#![allow(clippy::needless_doctest_main)]

use es_fluent_shared::fluent::FluentDomain;
use es_fluent_shared::resource::{
    FALLBACK_CATALOG_ENV, FALLBACK_CATALOG_FILE_NAME, FallbackCatalog, INVENTORY_RUNNER_ENV,
    ResourcePlan,
};
use es_fluent_toml::ResolvedI18nLayout;
use std::path::{Path, PathBuf};

#[allow(clippy::needless_doctest_main)]
/// Tracks configured locale assets and writes the strict fallback-message catalog.
///
/// Call this from your crate's `build.rs` so locale changes trigger a rebuild and
/// derived messages can be checked against resolvable fallback-locale values.
///
/// # Example
///
/// ```no_run
/// // build.rs
/// fn main() {
///     es_fluent_build::track_i18n_assets();
/// }
/// ```
pub fn track_i18n_assets() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
    let package_name = std::env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME must be set");
    let out_dir = std::env::var_os("OUT_DIR")
        .map(PathBuf::from)
        .expect("OUT_DIR must be set");
    let layout = ResolvedI18nLayout::from_manifest_dir(Path::new(&manifest_dir))
        .expect("Failed to read i18n.toml configuration");

    let catalog_path = out_dir.join(FALLBACK_CATALOG_FILE_NAME);
    println!("cargo:rerun-if-changed={}", layout.config_path.display());
    println!("cargo:rerun-if-changed={}", layout.assets_dir.display());
    println!("cargo:rerun-if-env-changed={INVENTORY_RUNNER_ENV}");
    println!(
        "cargo:rustc-env={FALLBACK_CATALOG_ENV}={}",
        catalog_path.display()
    );

    if std::env::var_os(INVENTORY_RUNNER_ENV).is_some() {
        std::fs::write(&catalog_path, b"")
            .expect("Failed to initialize fallback Fluent message catalog");
        return;
    }

    write_fallback_catalog(&layout, &package_name, &out_dir)
        .expect("Failed to build fallback Fluent message catalog");
}

fn write_fallback_catalog(
    layout: &ResolvedI18nLayout,
    package_name: &str,
    out_dir: &Path,
) -> Result<(), String> {
    layout
        .config
        .validate_for_package(package_name)
        .map_err(|error| error.to_string())?;
    let mut domains =
        vec![FluentDomain::try_new(package_name.to_string()).map_err(|error| error.to_string())?];
    domains.extend(layout.config.domains.iter().cloned());

    let mut catalog = FallbackCatalog::default();
    let crate_root_assets = assets_dir_is_manifest_root(layout);
    for domain in domains {
        let paths = if crate_root_assets {
            fallback_root_resource_paths(layout, &domain)?
        } else {
            let plans = ResourcePlan::sparse_from_assets(domain.as_str(), &layout.assets_dir)
                .map_err(|error| error.to_string())?;
            let Some((_, resources)) = plans
                .resource_specs_by_language()
                .iter()
                .find(|(language, _)| language == &layout.config.fallback_language)
            else {
                continue;
            };

            resources
                .iter()
                .map(|resource| {
                    layout
                        .output_dir
                        .join(resource.locale_relative_path.as_str())
                })
                .collect()
        };

        for path in paths {
            let source = std::fs::read_to_string(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            catalog.insert_source(&domain, source).map_err(|error| {
                format!(
                    "failed to catalog fallback resource {}: {error}",
                    path.display()
                )
            })?;
        }
    }

    let path = out_dir.join(FALLBACK_CATALOG_FILE_NAME);
    std::fs::write(&path, catalog.encode())
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn assets_dir_is_manifest_root(layout: &ResolvedI18nLayout) -> bool {
    match (
        layout.manifest_dir.canonicalize(),
        layout.assets_dir.canonicalize(),
    ) {
        (Ok(manifest_dir), Ok(assets_dir)) => manifest_dir == assets_dir,
        _ => false,
    }
}

fn fallback_root_resource_paths(
    layout: &ResolvedI18nLayout,
    domain: &FluentDomain,
) -> Result<Vec<PathBuf>, String> {
    let locales = layout
        .available_locale_names()
        .map_err(|error| error.to_string())?;
    let mut fallback_paths = Vec::new();

    for locale in locales {
        let locale_dir = layout.assets_dir.join(&locale);
        let base_path = locale_dir.join(format!("{}.ftl", domain.as_str()));
        if base_path.exists() && locale == layout.fallback_language {
            fallback_paths.push(base_path);
        }

        let namespace_root = locale_dir.join(domain.as_str());
        if !namespace_root.is_dir() {
            continue;
        }

        let namespace_paths = discover_namespace_paths(domain, &namespace_root)?;
        if locale == layout.fallback_language {
            fallback_paths.extend(namespace_paths);
        }
    }

    Ok(fallback_paths)
}

fn discover_namespace_paths(
    domain: &FluentDomain,
    namespace_root: &Path,
) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    let mut pending = vec![namespace_root.to_path_buf()];

    while let Some(current_dir) = pending.pop() {
        let entries = std::fs::read_dir(&current_dir)
            .map_err(|error| format!("failed to read {}: {error}", current_dir.display()))?;

        for entry in entries {
            let path = entry
                .map_err(|error| {
                    format!(
                        "failed to read directory entry in {}: {error}",
                        current_dir.display()
                    )
                })?
                .path();

            if path.is_dir() {
                pending.push(path);
                continue;
            }

            if !path.is_file() {
                continue;
            }

            if path.extension().and_then(|extension| extension.to_str()) != Some("ftl") {
                continue;
            }

            let relative_path = path.strip_prefix(namespace_root).map_err(|error| {
                format!(
                    "failed to derive namespace for asset {} relative to {}: {error}",
                    path.display(),
                    namespace_root.display()
                )
            })?;
            let relative_without_extension = relative_path.with_extension("");
            let mut components = Vec::new();
            for component in relative_without_extension.components() {
                let component = component.as_os_str().to_str().ok_or_else(|| {
                    format!(
                        "namespace path {} contains non-UTF-8 components",
                        relative_without_extension.display()
                    )
                })?;
                components.push(component);
            }

            if components.is_empty() {
                continue;
            }

            let namespace = components.join("/");
            es_fluent_shared::namespace::ResolvedNamespace::new(namespace.clone()).map_err(
                |error| {
                    format!(
                        "discovered invalid namespace '{namespace}' in assets for crate '{}': {error}",
                        domain.as_str()
                    )
                },
            )?;
            paths.push(path);
        }
    }

    Ok(paths)
}

#[cfg(test)]
#[serial_test::serial(manifest)]
mod tests {
    use super::*;
    use path_slash::PathExt as _;
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    fn with_manifest_env<T>(value: Option<&Path>, f: impl FnOnce() -> T) -> T {
        let out_dir = value.map(|path| path.join("build-output"));
        if let Some(out_dir) = &out_dir {
            fs::create_dir_all(out_dir).expect("create build output");
        }
        temp_env::with_vars(
            [
                ("CARGO_MANIFEST_DIR", value.map(Path::as_os_str)),
                ("CARGO_PKG_NAME", Some(std::ffi::OsStr::new("test-package"))),
                ("OUT_DIR", out_dir.as_deref().map(Path::as_os_str)),
            ],
            f,
        )
    }

    fn toml_path(path: &Path) -> String {
        path.to_slash_lossy().into_owned()
    }

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
        fs::write(locale_dir.join("test-package.ftl"), "broken = {\n")
            .expect("write malformed FTL");

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
        fs::write(locale_dir.join("test-package.ftl"), "broken = {\n")
            .expect("write malformed FTL");

        let result = with_manifest_env(Some(temp.path()), || {
            std::panic::catch_unwind(track_i18n_assets)
        });
        assert!(result.is_err());
    }

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

    #[test]
    fn track_i18n_assets_panics_without_manifest_dir() {
        let panic = with_manifest_env(None, || std::panic::catch_unwind(track_i18n_assets));
        assert!(panic.is_err());
    }

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

    #[test]
    fn configured_missing_fallback_message_obeys_package_local_policy() {
        let temp = tempfile::tempdir().expect("tempdir");
        let crate_dir = temp.path().join("fallback-app");
        let src_dir = crate_dir.join("src");
        let locale_dir = crate_dir.join("i18n/en");
        let target_dir = temp.path().join("target");
        let workspace_crates = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace crates directory");

        fs::create_dir_all(&src_dir).expect("create src dir");
        fs::create_dir_all(&locale_dir).expect("create locale dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            format!(
                r#"[package]
name = "fallback-app"
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
        fs::write(crate_dir.join("build.rs"), BUILD_TRACK_I18N_SOURCE).expect("write build.rs");
        fs::write(
            src_dir.join("lib.rs"),
            "#[derive(es_fluent::EsFluent)]\npub struct MissingValue;\n",
        )
        .expect("write lib.rs");
        fs::write(
            crate_dir.join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n.toml");
        fs::write(locale_dir.join("fallback-app.ftl"), "present = Present\n")
            .expect("write fallback FTL");

        let strict = cargo_check_output(&crate_dir, &target_dir, &[]);
        assert!(
            !strict.status.success(),
            "strict build should reject the missing key"
        );
        let strict_stderr = String::from_utf8_lossy(&strict.stderr);
        for expected in [
            "missing fallback Fluent message `missing_value`",
            "domain `fallback-app`",
            "Rust item `MissingValue`",
            "pub struct MissingValue",
            "expected a message value under `i18n/en`",
            "cargo es-fluent generate --package fallback-app",
        ] {
            assert!(
                strict_stderr.contains(expected),
                "expected {expected:?} in strict stderr: {strict_stderr}"
            );
        }
        assert!(!strict_stderr.contains("E0080"));
        assert!(!strict_stderr.contains("OUT_DIR"));

        fs::write(
            locale_dir.join("fallback-app.ftl"),
            "missing_value = Missing value\n",
        )
        .expect("write complete fallback FTL");
        let complete = cargo_check_output(&crate_dir, &target_dir, &[]);
        assert!(
            complete.status.success(),
            "strict build should accept the fallback key: {}",
            String::from_utf8_lossy(&complete.stderr)
        );

        fs::write(locale_dir.join("fallback-app.ftl"), "present = Present\n")
            .expect("restore missing fallback FTL");
        fs::write(
            crate_dir.join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\nmissing_message_policy = \"fallback-str\"\n",
        )
        .expect("write fallback policy");
        let fallback = cargo_check_output(&crate_dir, &target_dir, &[]);
        assert!(
            fallback.status.success(),
            "fallback-str build should succeed: {}",
            String::from_utf8_lossy(&fallback.stderr)
        );
    }

    #[test]
    fn mixed_workspace_keeps_missing_message_policy_package_local() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace_dir = temp.path().join("mixed-policy");
        let strict_dir = workspace_dir.join("strict-app");
        let fallback_dir = workspace_dir.join("fallback-app");
        let target_dir = temp.path().join("target");
        let workspace_crates = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace crates directory");
        fs::create_dir_all(strict_dir.join("src")).expect("create strict src");
        fs::create_dir_all(strict_dir.join("i18n/en")).expect("create strict locale");
        fs::create_dir_all(fallback_dir.join("src")).expect("create fallback src");
        fs::create_dir_all(fallback_dir.join("i18n/en")).expect("create fallback locale");
        fs::write(
            workspace_dir.join("Cargo.toml"),
            "[workspace]\nmembers = [\"strict-app\", \"fallback-app\"]\nresolver = \"3\"\n",
        )
        .expect("write workspace manifest");
        for (package, directory) in [("strict-app", &strict_dir), ("fallback-app", &fallback_dir)] {
            fs::write(
                directory.join("Cargo.toml"),
                format!(
                    "[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nes-fluent = {{ path = \"{}\" }}\n\n[build-dependencies]\nes-fluent-build = {{ path = \"{}\" }}\n",
                    toml_path(&workspace_crates.join("es-fluent")),
                    toml_path(Path::new(env!("CARGO_MANIFEST_DIR"))),
                ),
            )
            .expect("write package manifest");
            fs::write(directory.join("build.rs"), BUILD_TRACK_I18N_SOURCE)
                .expect("write build script");
        }
        fs::write(
            strict_dir.join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write strict config");
        fs::write(
            fallback_dir.join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\nmissing_message_policy = \"fallback-str\"\n",
        )
        .expect("write fallback config");
        fs::write(
            strict_dir.join("src/lib.rs"),
            "#[derive(es_fluent::EsFluent)]\npub struct MissingStrict;\n",
        )
        .expect("write strict source");
        fs::write(
            fallback_dir.join("src/lib.rs"),
            r#"#[derive(es_fluent::EsFluent)]
pub struct MissingFallback;

#[cfg(test)]
mod tests {
    use super::MissingFallback;
    use es_fluent::{FluentArgs, FluentLocalizer, FluentLocalizerExt as _};
    use es_fluent::registry::StaticFluentMessageKey;

    struct Missing;

    impl FluentLocalizer for Missing {
        fn localize<'a>(&self, _key: StaticFluentMessageKey, _args: Option<&FluentArgs<'a>>) -> Option<String> {
            None
        }
    }

    #[test]
    fn normal_and_fallible_lookup_keep_distinct_semantics() {
        assert_eq!(Missing.localize_message(&MissingFallback), "missing_fallback");
        assert_eq!(Missing.try_localize_message(&MissingFallback), None);
    }
}
"#,
        )
        .expect("write fallback source");
        fs::write(
            strict_dir.join("i18n/en/strict-app.ftl"),
            "present = Present\n",
        )
        .expect("write incomplete strict resource");
        fs::write(
            fallback_dir.join("i18n/en/fallback-app.ftl"),
            "present = Present\n",
        )
        .expect("write fallback resource");

        let strict = cargo_workspace_output(
            &workspace_dir,
            &target_dir,
            &["check", "--quiet", "-p", "strict-app"],
        );
        assert!(!strict.status.success());
        assert!(
            String::from_utf8_lossy(&strict.stderr)
                .contains("missing fallback Fluent message `missing_strict`")
        );

        let workspace = cargo_workspace_output(
            &workspace_dir,
            &target_dir,
            &["check", "--quiet", "--workspace"],
        );
        assert!(!workspace.status.success());
        let stderr = String::from_utf8_lossy(&workspace.stderr);
        assert!(stderr.contains("domain `strict-app`"), "{stderr}");
        assert!(stderr.contains("Rust item `MissingStrict`"), "{stderr}");

        fs::write(
            strict_dir.join("i18n/en/strict-app.ftl"),
            "missing_strict = Missing strict\n",
        )
        .expect("complete strict resource");
        let complete = cargo_workspace_output(
            &workspace_dir,
            &target_dir,
            &["test", "--quiet", "--workspace"],
        );
        assert!(
            complete.status.success(),
            "mixed workspace should pass after completing strict resources: {}",
            String::from_utf8_lossy(&complete.stderr)
        );
    }

    fn cargo_workspace_output(
        workspace_dir: &Path,
        target_dir: &Path,
        args: &[&str],
    ) -> std::process::Output {
        Command::new("cargo")
            .args(args)
            .current_dir(workspace_dir)
            .env("CARGO_TARGET_DIR", target_dir)
            .output()
            .expect("run cargo workspace command")
    }

    fn cargo_check_output(
        crate_dir: &Path,
        target_dir: &Path,
        args: &[&str],
    ) -> std::process::Output {
        let mut command = Command::new("cargo");
        command
            .arg("check")
            .arg("--quiet")
            .args(args)
            .current_dir(crate_dir)
            .env("CARGO_TARGET_DIR", target_dir);
        command.output().expect("run cargo check")
    }

    fn cargo_check_output_with_inventory(
        crate_dir: &Path,
        target_dir: &Path,
        inventory: bool,
    ) -> std::process::Output {
        let mut command = Command::new("cargo");
        command
            .arg("check")
            .arg("--quiet")
            .current_dir(crate_dir)
            .env("CARGO_TARGET_DIR", target_dir)
            .env("RUSTFLAGS", "-A warnings");
        if inventory {
            command.env(INVENTORY_RUNNER_ENV, "1");
        }
        command.output().expect("run cargo check")
    }

    fn run_cargo_check(crate_dir: &Path, target_dir: &Path, trace_file: &Path) {
        let status = Command::new("cargo")
            .arg("check")
            .arg("--quiet")
            .current_dir(crate_dir)
            .env("CARGO_TARGET_DIR", target_dir)
            .env("TRACE_FILE", trace_file)
            .status()
            .expect("run cargo check");

        assert!(status.success(), "cargo check should succeed");
    }

    fn trace_lines(trace_file: &Path) -> usize {
        fs::read_to_string(trace_file)
            .expect("read trace file")
            .lines()
            .count()
    }

    fn panic_message(panic: &(dyn std::any::Any + Send)) -> Option<&str> {
        if let Some(message) = panic.downcast_ref::<&str>() {
            Some(message)
        } else {
            panic.downcast_ref::<String>().map(String::as_str)
        }
    }

    const BUILD_TRACK_I18N_SOURCE: &str = r#"fn main() {
    es_fluent_build::track_i18n_assets();
}
"#;

    const BUILD_SCRIPT_SOURCE: &str = r#"use std::fs::OpenOptions;
use std::io::Write;

fn main() {
    es_fluent_build::track_i18n_assets();

    let trace_path = std::env::var("TRACE_FILE").expect("TRACE_FILE must be set");
    let mut trace = OpenOptions::new()
        .create(true)
        .append(true)
        .open(trace_path)
        .expect("open trace file");
    writeln!(trace, "ran").expect("write trace line");
}
"#;
}

use crate::I18nConfig;
use std::path::Path;

#[allow(clippy::needless_doctest_main)]
/// Emits Cargo rebuild hints for `i18n.toml` and the configured assets directory.
///
/// Call this from your crate's `build.rs` to ensure changes to locale files
/// (including renames and deletions) trigger a rebuild, keeping embedded/localized data fresh.
///
/// # Example
///
/// ```no_run
/// // build.rs
/// fn main() {
///     es_fluent_toml::build::track_i18n_assets();
/// }
/// ```
pub fn track_i18n_assets() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
    let config =
        I18nConfig::read_from_manifest_dir().expect("Failed to read i18n.toml configuration");
    let assets_dir = config
        .assets_dir_from_manifest()
        .expect("Failed to resolve assets directory from i18n.toml");

    let config_path = Path::new(&manifest_dir).join("i18n.toml");
    println!("cargo:rerun-if-changed={}", config_path.display());
    println!("cargo:rerun-if-changed={}", assets_dir.display());
}

#[cfg(test)]
#[serial_test::serial(manifest)]
mod tests {
    use super::*;
    use crate::test_utils::with_manifest_env;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::tempdir;

    fn toml_path(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    #[test]
    fn track_i18n_assets_reads_config_and_assets_path() {
        let temp = tempdir().expect("tempdir");
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
    fn track_i18n_assets_does_not_create_stamp_file() {
        let temp = tempdir().expect("tempdir");
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
    fn track_i18n_assets_does_not_create_stamp_file_for_external_assets_dir() {
        let temp = tempdir().expect("tempdir");
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

        with_manifest_env(Some(&crate_dir), || {
            track_i18n_assets();
        });

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
        let temp = tempdir().expect("tempdir");
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
es-fluent-toml = {{ path = "{}" }}
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

    const BUILD_SCRIPT_SOURCE: &str = r#"use std::fs::OpenOptions;
use std::io::Write;

fn main() {
    es_fluent_toml::build::track_i18n_assets();

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

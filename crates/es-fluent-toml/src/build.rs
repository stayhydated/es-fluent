use crate::I18nConfig;
use std::fs;
use std::path::Path;

/// Emits Cargo rebuild hints for `i18n.toml` and the configured assets directory.
///
/// Call this from your crate's `build.rs` to ensure changes to locale files
/// (including renames and deletions) trigger a rebuild, keeping embedded/localized data fresh.
///
/// # Example
///
/// ```
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

    if assets_dir.is_dir() {
        let assets_parent = assets_dir.parent().unwrap_or(&assets_dir);
        let es_fluent_dir = assets_parent.join(".es-fluent");
        let _ = fs::create_dir_all(&es_fluent_dir);
        let stamp_file = es_fluent_dir.join("locales.stamp");
        let mut locales = Vec::new();

        if let Ok(entries) = fs::read_dir(&assets_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    println!("cargo:rerun-if-changed={}", path.display());
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        let mut has_ftl = false;
                        if let Ok(ftl_entries) = fs::read_dir(&path) {
                            for ftl_entry in ftl_entries.flatten() {
                                let ftl_path = ftl_entry.path();
                                if ftl_path.extension().is_some_and(|e| e == "ftl") {
                                    println!("cargo:rerun-if-changed={}", ftl_path.display());
                                    has_ftl = true;
                                }
                            }
                        }
                        if has_ftl {
                            locales.push(name.to_string());
                        }
                    }
                }
            }
        }

        locales.sort();
        let stamp_content = locales.join("\n");

        let existing_stamp = fs::read_to_string(&stamp_file).unwrap_or_default();
        if existing_stamp != stamp_content {
            let _ = fs::write(&stamp_file, &stamp_content);
        }

        println!("cargo:rerun-if-changed={}", stamp_file.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::with_manifest_env;
    use tempfile::tempdir;

    #[test]
    fn track_i18n_assets_reads_config_and_assets_path() {
        let temp = tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("i18n")).expect("create assets dir");
        std::fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write config");

        with_manifest_env(Some(temp.path()), || {
            track_i18n_assets();
        });
    }

    #[test]
    fn track_i18n_assets_creates_stamp_file() {
        let temp = tempdir().expect("tempdir");
        let i18n_dir = temp.path().join("i18n");
        std::fs::create_dir_all(i18n_dir.join("en")).expect("create en dir");
        std::fs::create_dir_all(i18n_dir.join("fr")).expect("create fr dir");
        std::fs::write(i18n_dir.join("en").join("main.ftl"), "hello = Hello").expect("write ftl");
        std::fs::write(i18n_dir.join("fr").join("main.ftl"), "hello = Bonjour").expect("write ftl");
        std::fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write config");

        with_manifest_env(Some(temp.path()), || {
            track_i18n_assets();
        });

        let stamp = temp.path().join(".es-fluent").join("locales.stamp");
        assert!(
            stamp.exists(),
            "stamp file should be created at .es-fluent/locales.stamp"
        );
        let content = std::fs::read_to_string(stamp).expect("read stamp");
        assert_eq!(content, "en\nfr", "stamp should contain sorted locale list");
    }

    #[test]
    fn track_i18n_assets_updates_stamp_on_locale_change() {
        let temp = tempdir().expect("tempdir");
        let i18n_dir = temp.path().join("i18n");
        std::fs::create_dir_all(i18n_dir.join("en")).expect("create en dir");
        std::fs::write(i18n_dir.join("en").join("main.ftl"), "hello = Hello").expect("write ftl");
        std::fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write config");

        with_manifest_env(Some(temp.path()), || {
            track_i18n_assets();
        });

        let stamp = temp.path().join(".es-fluent").join("locales.stamp");
        assert_eq!(
            std::fs::read_to_string(&stamp).unwrap(),
            "en",
            "initial stamp"
        );

        std::fs::create_dir_all(i18n_dir.join("de")).expect("create de dir");
        std::fs::write(i18n_dir.join("de").join("main.ftl"), "hello = Hallo").expect("write ftl");
        with_manifest_env(Some(temp.path()), || {
            track_i18n_assets();
        });

        assert_eq!(
            std::fs::read_to_string(&stamp).unwrap(),
            "de\nen",
            "stamp updated with new locale"
        );
    }

    #[test]
    fn track_i18n_assets_stamp_relative_to_assets_dir() {
        let temp = tempdir().expect("tempdir");
        let crate_dir = temp.path().join("my-crate");
        let assets_dir = temp.path().join("assets").join("i18n");
        std::fs::create_dir_all(&crate_dir).expect("create crate dir");
        std::fs::create_dir_all(assets_dir.join("en")).expect("create en dir");
        std::fs::write(assets_dir.join("en").join("main.ftl"), "hello = Hello").expect("write ftl");
        std::fs::write(
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
            stamp.exists(),
            "stamp should be at assets parent, not crate dir"
        );
        assert_eq!(
            std::fs::read_to_string(&stamp).unwrap(),
            "en",
            "stamp content correct"
        );
    }

    #[test]
    fn track_i18n_assets_updates_stamp_on_locale_deletion() {
        let temp = tempdir().expect("tempdir");
        let i18n_dir = temp.path().join("i18n");
        std::fs::create_dir_all(i18n_dir.join("en")).expect("create en dir");
        std::fs::create_dir_all(i18n_dir.join("fr")).expect("create fr dir");
        std::fs::create_dir_all(i18n_dir.join("de")).expect("create de dir");
        std::fs::write(i18n_dir.join("en").join("main.ftl"), "hello = Hello").expect("write ftl");
        std::fs::write(i18n_dir.join("fr").join("main.ftl"), "hello = Bonjour").expect("write ftl");
        std::fs::write(i18n_dir.join("de").join("main.ftl"), "hello = Hallo").expect("write ftl");
        std::fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write config");

        with_manifest_env(Some(temp.path()), || {
            track_i18n_assets();
        });

        let stamp = temp.path().join(".es-fluent").join("locales.stamp");
        assert_eq!(
            std::fs::read_to_string(&stamp).unwrap(),
            "de\nen\nfr",
            "initial stamp should have all three locales"
        );

        std::fs::remove_dir_all(i18n_dir.join("de")).expect("delete de dir");
        std::fs::remove_dir_all(i18n_dir.join("fr")).expect("delete fr dir");

        with_manifest_env(Some(temp.path()), || {
            track_i18n_assets();
        });

        assert_eq!(
            std::fs::read_to_string(&stamp).unwrap(),
            "en",
            "stamp should only contain remaining locale after deletion"
        );
    }

    #[test]
    fn track_i18n_assets_stamp_no_change_when_locales_unchanged() {
        let temp = tempdir().expect("tempdir");
        let i18n_dir = temp.path().join("i18n");
        std::fs::create_dir_all(i18n_dir.join("en")).expect("create en dir");
        std::fs::create_dir_all(i18n_dir.join("fr")).expect("create fr dir");
        std::fs::write(i18n_dir.join("en").join("main.ftl"), "hello = Hello").expect("write ftl");
        std::fs::write(i18n_dir.join("fr").join("main.ftl"), "hello = Bonjour").expect("write ftl");
        std::fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write config");

        with_manifest_env(Some(temp.path()), || {
            track_i18n_assets();
        });

        let stamp = temp.path().join(".es-fluent").join("locales.stamp");
        let original_metadata = std::fs::metadata(&stamp).expect("stamp metadata");
        let original_modified = original_metadata.modified().expect("modified time");

        std::thread::sleep(std::time::Duration::from_millis(10));

        with_manifest_env(Some(temp.path()), || {
            track_i18n_assets();
        });

        let new_metadata = std::fs::metadata(&stamp).expect("new stamp metadata");
        let new_modified = new_metadata.modified().expect("new modified time");

        assert_eq!(
            original_modified, new_modified,
            "stamp file should not be rewritten when content unchanged"
        );
    }

    #[test]
    fn track_i18n_assets_ignores_dirs_without_ftl_files() {
        let temp = tempdir().expect("tempdir");
        let i18n_dir = temp.path().join("i18n");
        std::fs::create_dir_all(i18n_dir.join("en")).expect("create en dir");
        std::fs::create_dir_all(i18n_dir.join("empty-locale")).expect("create empty locale dir");
        std::fs::write(i18n_dir.join("en").join("main.ftl"), "hello = Hello").expect("write ftl");
        std::fs::write(
            i18n_dir.join("empty-locale").join("readme.txt"),
            "not an ftl file",
        )
        .expect("write readme");
        std::fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write config");

        with_manifest_env(Some(temp.path()), || {
            track_i18n_assets();
        });

        let stamp = temp.path().join(".es-fluent").join("locales.stamp");
        assert_eq!(
            std::fs::read_to_string(&stamp).unwrap(),
            "en",
            "should only include locales with ftl files"
        );
    }

    #[test]
    fn track_i18n_assets_panics_without_manifest_dir() {
        let panic = with_manifest_env(None, || std::panic::catch_unwind(track_i18n_assets));
        assert!(panic.is_err());
    }
}

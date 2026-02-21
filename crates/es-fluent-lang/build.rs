use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    if env::var("CARGO_FEATURE_LOCALIZED_LANGS").is_err() {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let i18n_dir = manifest_dir.join("i18n");
    let es_fluent_dir = manifest_dir.join(".es-fluent");
    let _ = fs::create_dir_all(&es_fluent_dir);
    let locales_stamp = es_fluent_dir.join("locales.stamp");

    println!("cargo:rerun-if-changed={}", i18n_dir.display());

    let locales = scan_locales(&i18n_dir);
    update_locales_stamp(&locales_stamp, &locales);

    println!("cargo:rerun-if-changed={}", locales_stamp.display());
}

fn scan_locales(i18n_dir: &Path) -> Vec<String> {
    let mut locales = Vec::new();

    if i18n_dir.is_dir() {
        let entries = fs::read_dir(i18n_dir).unwrap();
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(locale_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            println!("cargo:rerun-if-changed={}", path.display());
            let resource_path = path.join("es-fluent-lang.ftl");
            if resource_path.is_file() {
                println!("cargo:rerun-if-changed={}", resource_path.display());
                locales.push(locale_name.to_string());
            }
        }
    }

    locales.sort();
    locales
}

fn update_locales_stamp(stamp_path: &Path, locales: &[String]) {
    let new_stamp_content = locales.join("\n");
    let existing_stamp = fs::read_to_string(stamp_path).unwrap_or_default();
    if existing_stamp != new_stamp_content {
        fs::write(stamp_path, new_stamp_content).expect("write locales stamp");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn scan_locales_finds_directories_with_ftl_files() {
        let temp = TempDir::new().expect("tempdir");
        let i18n_dir = temp.path().join("i18n");
        fs::create_dir_all(i18n_dir.join("en")).expect("create en");
        fs::create_dir_all(i18n_dir.join("fr")).expect("create fr");
        fs::write(
            i18n_dir.join("en").join("es-fluent-lang.ftl"),
            "test = test",
        )
        .expect("write ftl");
        fs::write(
            i18n_dir.join("fr").join("es-fluent-lang.ftl"),
            "test = test",
        )
        .expect("write ftl");

        let locales = scan_locales(&i18n_dir);

        assert_eq!(locales, vec!["en", "fr"]);
    }

    #[test]
    fn scan_locales_ignores_directories_without_ftl() {
        let temp = TempDir::new().expect("tempdir");
        let i18n_dir = temp.path().join("i18n");
        fs::create_dir_all(i18n_dir.join("en")).expect("create en");
        fs::create_dir_all(i18n_dir.join("empty")).expect("create empty");
        fs::write(
            i18n_dir.join("en").join("es-fluent-lang.ftl"),
            "test = test",
        )
        .expect("write ftl");

        let locales = scan_locales(&i18n_dir);

        assert_eq!(locales, vec!["en"]);
    }

    #[test]
    fn scan_locales_returns_sorted_locales() {
        let temp = TempDir::new().expect("tempdir");
        let i18n_dir = temp.path().join("i18n");
        fs::create_dir_all(i18n_dir.join("de")).expect("create de");
        fs::create_dir_all(i18n_dir.join("en")).expect("create en");
        fs::create_dir_all(i18n_dir.join("fr")).expect("create fr");
        fs::write(i18n_dir.join("de").join("es-fluent-lang.ftl"), "test").expect("write");
        fs::write(i18n_dir.join("en").join("es-fluent-lang.ftl"), "test").expect("write");
        fs::write(i18n_dir.join("fr").join("es-fluent-lang.ftl"), "test").expect("write");

        let locales = scan_locales(&i18n_dir);

        assert_eq!(locales, vec!["de", "en", "fr"]);
    }

    #[test]
    fn scan_locales_handles_missing_directory() {
        let temp = TempDir::new().expect("tempdir");
        let nonexistent = temp.path().join("nonexistent");

        let locales = scan_locales(&nonexistent);

        assert!(locales.is_empty());
    }

    #[test]
    fn scan_locales_ignores_files_in_i18n_root() {
        let temp = TempDir::new().expect("tempdir");
        let i18n_dir = temp.path().join("i18n");
        fs::create_dir_all(i18n_dir.join("en")).expect("create en");
        fs::write(i18n_dir.join("readme.txt"), "readme").expect("write file");
        fs::write(i18n_dir.join("en").join("es-fluent-lang.ftl"), "test").expect("write ftl");

        let locales = scan_locales(&i18n_dir);

        assert_eq!(locales, vec!["en"]);
    }

    #[test]
    fn update_locales_stamp_writes_when_changed() {
        let temp = TempDir::new().expect("tempdir");
        let stamp_path = temp.path().join("locales.stamp");

        update_locales_stamp(&stamp_path, &["en".to_string(), "fr".to_string()]);
        assert_eq!(
            fs::read_to_string(&stamp_path).expect("read stamp"),
            "en\nfr"
        );

        update_locales_stamp(&stamp_path, &["en".to_string()]);
        assert_eq!(fs::read_to_string(&stamp_path).expect("read stamp"), "en");
    }

    #[test]
    fn stamp_updated_on_locale_deletion() {
        let temp = TempDir::new().expect("tempdir");
        let i18n_dir = temp.path().join("i18n");
        let es_fluent_dir = temp.path().join(".es-fluent");
        fs::create_dir_all(&es_fluent_dir).expect("create .es-fluent");

        fs::create_dir_all(i18n_dir.join("en")).expect("create en");
        fs::create_dir_all(i18n_dir.join("fr")).expect("create fr");
        fs::create_dir_all(i18n_dir.join("de")).expect("create de");
        fs::write(i18n_dir.join("en").join("es-fluent-lang.ftl"), "test").expect("write");
        fs::write(i18n_dir.join("fr").join("es-fluent-lang.ftl"), "test").expect("write");
        fs::write(i18n_dir.join("de").join("es-fluent-lang.ftl"), "test").expect("write");

        let locales = scan_locales(&i18n_dir);
        let stamp_path = es_fluent_dir.join("locales.stamp");
        update_locales_stamp(&stamp_path, &locales);
        let stamp_content = fs::read_to_string(&stamp_path).expect("read initial stamp");

        assert_eq!(stamp_content, "de\nen\nfr", "initial stamp has all locales");

        fs::remove_dir_all(i18n_dir.join("fr")).expect("delete fr");
        fs::remove_dir_all(i18n_dir.join("de")).expect("delete de");

        let new_locales = scan_locales(&i18n_dir);
        update_locales_stamp(&stamp_path, &new_locales);
        let new_stamp_content = fs::read_to_string(&stamp_path).expect("read updated stamp");

        assert_ne!(
            stamp_content, new_stamp_content,
            "stamp content should change"
        );
        assert_eq!(new_stamp_content, "en", "only en remains");
    }
}

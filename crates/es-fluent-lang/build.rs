use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let is_localized_langs = env::var("CARGO_FEATURE_LOCALIZED_LANGS").is_ok();

    if !is_localized_langs {
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        let out_path = out_dir.join("es_fluent_lang_static_resources.rs");
        fs::write(out_path, "").unwrap();
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let i18n_dir = manifest_dir.join("i18n");
    let es_fluent_dir = manifest_dir.join(".es-fluent");
    let _ = fs::create_dir_all(&es_fluent_dir);
    let locales_stamp = es_fluent_dir.join("locales.stamp");

    println!("cargo:rerun-if-changed={}", i18n_dir.display());

    let locales = scan_locales(&i18n_dir);

    let new_stamp_content = locales.join("\n");
    let existing_stamp = fs::read_to_string(&locales_stamp).unwrap_or_default();
    if existing_stamp != new_stamp_content {
        fs::write(&locales_stamp, &new_stamp_content).unwrap();
    }

    println!("cargo:rerun-if-changed={}", locales_stamp.display());

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_path = out_dir.join("es_fluent_lang_static_resources.rs");

    fs::write(&out_path, generate_static_resources(&locales)).unwrap();
}

fn scan_locales(i18n_dir: &std::path::Path) -> Vec<String> {
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

fn generate_static_resources(locales: &[String]) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "static ES_FLUENT_LANG_STATIC_RESOURCES: [EsFluentLangStaticResource; {}] = [\n",
        locales.len()
    ));
    for locale in locales {
        output.push_str(&format!(
            "    EsFluentLangStaticResource::new(\"{}\"),\n",
            locale
        ));
    }
    output.push_str("];\n\n");
    for idx in 0..locales.len() {
        output.push_str(&format!(
            "inventory::submit! {{ &ES_FLUENT_LANG_STATIC_RESOURCES[{}] as &dyn es_fluent_manager_core::StaticI18nResource }}\n",
            idx
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
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
    fn generate_static_resources_empty() {
        let output = generate_static_resources(&[]);
        assert!(output.contains("[EsFluentLangStaticResource; 0]"));
        assert!(!output.contains("inventory::submit!"));
    }

    #[test]
    fn generate_static_resources_single_locale() {
        let output = generate_static_resources(&["en".to_string()]);

        assert!(output.contains("[EsFluentLangStaticResource; 1]"));
        assert!(output.contains("EsFluentLangStaticResource::new(\"en\")"));
        assert!(output.contains("inventory::submit!"));
    }

    #[test]
    fn generate_static_resources_multiple_locales() {
        let output =
            generate_static_resources(&["en".to_string(), "fr".to_string(), "ja".to_string()]);

        assert!(output.contains("[EsFluentLangStaticResource; 3]"));
        assert!(output.contains("EsFluentLangStaticResource::new(\"en\")"));
        assert!(output.contains("EsFluentLangStaticResource::new(\"fr\")"));
        assert!(output.contains("EsFluentLangStaticResource::new(\"ja\")"));
        assert!(output.contains("ES_FLUENT_LANG_STATIC_RESOURCES[0]"));
        assert!(output.contains("ES_FLUENT_LANG_STATIC_RESOURCES[1]"));
        assert!(output.contains("ES_FLUENT_LANG_STATIC_RESOURCES[2]"));
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
        let stamp_content = locales.join("\n");
        fs::write(&stamp_path, &stamp_content).expect("write stamp");

        assert_eq!(
            fs::read_to_string(&stamp_path).unwrap(),
            "de\nen\nfr",
            "initial stamp has all locales"
        );

        fs::remove_dir_all(i18n_dir.join("fr")).expect("delete fr");
        fs::remove_dir_all(i18n_dir.join("de")).expect("delete de");

        let new_locales = scan_locales(&i18n_dir);
        let new_stamp_content = new_locales.join("\n");

        assert_ne!(
            stamp_content, new_stamp_content,
            "stamp content should change"
        );
        assert_eq!(new_stamp_content, "en", "only en remains");
    }
}

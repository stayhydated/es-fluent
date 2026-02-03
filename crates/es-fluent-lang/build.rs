use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let i18n_dir = manifest_dir.join("i18n");

    println!("cargo:rerun-if-changed={}", i18n_dir.display());

    let mut locales = Vec::new();

    if i18n_dir.is_dir() {
        let entries = fs::read_dir(&i18n_dir).unwrap();
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(locale_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let resource_path = path.join("es-fluent-lang.ftl");
            if resource_path.is_file() {
                println!("cargo:rerun-if-changed={}", resource_path.display());
                locales.push(locale_name.to_string());
            }
        }
    }

    locales.sort();

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_path = out_dir.join("es_fluent_lang_static_resources.rs");

    let mut output = String::new();
    output.push_str(&format!(
        "static ES_FLUENT_LANG_STATIC_RESOURCES: [EsFluentLangStaticResource; {}] = [\n",
        locales.len()
    ));
    for locale in &locales {
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

    fs::write(out_path, output).unwrap();
}

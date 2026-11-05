//! Content fingerprint of the i18n inputs that es-fluent macros read from disk.

use es_fluent_toml::{ResolvedI18nLayout, crate_root_asset_ignored_dir_names};
use std::hash::{DefaultHasher, Hash as _, Hasher as _};
use std::path::Path;

/// Hashes `i18n.toml`, the locale directory listing, and every FTL file under
/// a locale directory.
///
/// Macros read these inputs with `std::fs`, which rustc does not record.
/// The fingerprint changes whenever any of them do, including when a locale
/// directory or FTL file is added or removed. `inventory_runner` is included
/// because CLI inventory builds expand derives differently.
pub(crate) fn build_inputs_fingerprint(
    layout: &ResolvedI18nLayout,
    inventory_runner: bool,
) -> String {
    let mut hasher = DefaultHasher::new();
    inventory_runner.hash(&mut hasher);
    hash_file(&mut hasher, &layout.config_path);

    let crate_root_assets = super::catalog::assets_dir_is_manifest_root(layout);
    for entry in sorted_entries(&layout.assets_dir) {
        let Some(name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
            continue;
        };
        if !entry.path().is_dir()
            || (crate_root_assets && crate_root_asset_ignored_dir_names().contains(&name.as_str()))
        {
            continue;
        }
        "dir".hash(&mut hasher);
        name.hash(&mut hasher);
        if es_fluent_shared::parse_canonical_language_identifier(&name).is_ok() {
            hash_ftl_tree(&mut hasher, &entry.path(), &name);
        }
    }

    format!("{:016x}", hasher.finish())
}

fn hash_ftl_tree(hasher: &mut DefaultHasher, dir: &Path, relative: &str) {
    for entry in sorted_entries(dir) {
        let path = entry.path();
        let Some(name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
            continue;
        };
        let relative = format!("{relative}/{name}");
        if path.is_dir() {
            "dir".hash(hasher);
            relative.hash(hasher);
            hash_ftl_tree(hasher, &path, &relative);
        } else if path.extension().is_some_and(|extension| extension == "ftl") {
            "file".hash(hasher);
            relative.hash(hasher);
            hash_file(hasher, &path);
        }
    }
}

fn hash_file(hasher: &mut DefaultHasher, path: &Path) {
    match std::fs::read(path) {
        Ok(contents) => contents.hash(hasher),
        Err(error) => error.kind().to_string().hash(hasher),
    }
}

fn sorted_entries(dir: &Path) -> Vec<std::fs::DirEntry> {
    let mut entries = std::fs::read_dir(dir)
        .map(|entries| entries.filter_map(Result::ok).collect::<Vec<_>>())
        .unwrap_or_default();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct Package(tempfile::TempDir);

    impl Package {
        fn new(assets_dir: &str) -> Self {
            let temp = tempfile::tempdir().expect("tempdir");
            fs::write(
                temp.path().join("i18n.toml"),
                format!("fallback_language = \"en\"\nassets_dir = \"{assets_dir}\"\n"),
            )
            .expect("write i18n.toml");
            Self(temp)
        }

        fn write(&self, relative: &str, contents: &str) {
            let path = self.0.path().join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("create dirs");
            fs::write(path, contents).expect("write file");
        }

        fn fingerprint(&self) -> String {
            let layout = ResolvedI18nLayout::from_manifest_dir(self.0.path()).expect("layout");
            build_inputs_fingerprint(&layout, false)
        }
    }

    #[test]
    fn fingerprint_is_stable_for_unchanged_inputs() {
        let package = Package::new("i18n");
        package.write("i18n/en/app.ftl", "hello = Hello\n");

        assert_eq!(package.fingerprint(), package.fingerprint());
    }

    #[test]
    fn fingerprint_tracks_config_locales_and_ftl_contents() {
        let package = Package::new("i18n");
        package.write("i18n/en/app.ftl", "hello = Hello\n");
        let mut seen = vec![package.fingerprint()];

        package.write("i18n/en/app.ftl", "hello = Hi\n");
        seen.push(package.fingerprint());

        fs::create_dir(package.0.path().join("i18n/fr")).expect("create empty locale");
        seen.push(package.fingerprint());

        package.write("i18n/fr/app/ui.ftl", "title = Titre\n");
        seen.push(package.fingerprint());

        package.write(
            "i18n.toml",
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\nfallback = \"fallback-str\"\n",
        );
        seen.push(package.fingerprint());

        let layout = ResolvedI18nLayout::from_manifest_dir(package.0.path()).expect("layout");
        seen.push(build_inputs_fingerprint(&layout, true));

        let unique = seen.iter().collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            unique.len(),
            seen.len(),
            "every input change must change the fingerprint: {seen:?}"
        );
    }

    #[test]
    fn fingerprint_ignores_files_macros_do_not_read() {
        let package = Package::new(".");
        package.write("en/app.ftl", "hello = Hello\n");
        let before = package.fingerprint();

        package.write("en/notes.txt", "draft\n");
        package.write("src/lib.rs", "pub fn changed() {}\n");
        package.write("target/debug/out.ftl", "ignored = Ignored\n");

        assert_eq!(package.fingerprint(), before);
    }
}

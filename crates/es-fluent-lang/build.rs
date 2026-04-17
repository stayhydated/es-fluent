use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_LOCALIZED_LANGS");

    if env::var("CARGO_FEATURE_LOCALIZED_LANGS").is_err() {
        return;
    }

    let i18n_dir = i18n_dir_from_manifest();
    println!("cargo:rerun-if-changed={}", i18n_dir.display());
}

fn i18n_dir_from_manifest() -> PathBuf {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set"));
    manifest_dir.join("i18n")
}

#[cfg(test)]
#[serial_test::serial(manifest)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn with_manifest_env<T>(manifest_dir: Option<&std::path::Path>, f: impl FnOnce() -> T) -> T {
        temp_env::with_var("CARGO_MANIFEST_DIR", manifest_dir, f)
    }

    #[test]
    fn i18n_dir_from_manifest_resolves_expected_path() {
        let temp = tempdir().expect("tempdir");
        let expected = temp.path().join("i18n");

        with_manifest_env(Some(temp.path()), || {
            assert_eq!(i18n_dir_from_manifest(), expected);
        });
    }

    #[test]
    fn i18n_dir_from_manifest_panics_without_manifest_env() {
        with_manifest_env(None, || {
            let panic = std::panic::catch_unwind(i18n_dir_from_manifest);
            assert!(panic.is_err());
        });
    }
}

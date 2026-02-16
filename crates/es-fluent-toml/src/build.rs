use crate::I18nConfig;
use std::path::Path;

/// Emits Cargo rebuild hints for `i18n.toml` and the configured assets directory.
///
/// Call this from your crate's `build.rs` to ensure changes to locale files
/// (including renames) trigger a rebuild, keeping embedded/localized data fresh.
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
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};
    use tempfile::tempdir;

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn with_manifest_env<T>(value: Option<&str>, f: impl FnOnce() -> T) -> T {
        let _guard = ENV_LOCK.lock().expect("lock poisoned");
        let previous = std::env::var("CARGO_MANIFEST_DIR").ok();

        match value {
            Some(path) => {
                // SAFETY: tests hold a global lock and restore the previous value afterwards.
                unsafe { std::env::set_var("CARGO_MANIFEST_DIR", path) };
            },
            None => {
                // SAFETY: tests hold a global lock and restore the previous value afterwards.
                unsafe { std::env::remove_var("CARGO_MANIFEST_DIR") };
            },
        }

        let result = f();

        match previous {
            Some(prev) => {
                // SAFETY: tests hold a global lock and restore the previous value afterwards.
                unsafe { std::env::set_var("CARGO_MANIFEST_DIR", prev) };
            },
            None => {
                // SAFETY: tests hold a global lock and restore the previous value afterwards.
                unsafe { std::env::remove_var("CARGO_MANIFEST_DIR") };
            },
        }

        result
    }

    #[test]
    fn track_i18n_assets_reads_config_and_assets_path() {
        let temp = tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("i18n")).expect("create assets dir");
        std::fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write config");

        with_manifest_env(temp.path().to_str(), || {
            track_i18n_assets();
        });
    }

    #[test]
    fn track_i18n_assets_panics_without_manifest_dir() {
        let panic = with_manifest_env(None, || std::panic::catch_unwind(track_i18n_assets));
        assert!(panic.is_err());
    }
}

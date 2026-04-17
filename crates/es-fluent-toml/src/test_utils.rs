use std::path::Path;

pub fn with_manifest_env<T>(value: Option<&Path>, f: impl FnOnce() -> T) -> T {
    temp_env::with_var("CARGO_MANIFEST_DIR", value, f)
}

use std::path::Path;

pub fn with_manifest_env<T>(value: Option<&Path>, f: impl FnOnce() -> T) -> T {
    let previous = std::env::var("CARGO_MANIFEST_DIR").ok();

    match value {
        Some(path) => {
            unsafe { std::env::set_var("CARGO_MANIFEST_DIR", path) };
        },
        None => {
            unsafe { std::env::remove_var("CARGO_MANIFEST_DIR") };
        },
    }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    match previous {
        Some(prev) => {
            unsafe { std::env::set_var("CARGO_MANIFEST_DIR", prev) };
        },
        None => {
            unsafe { std::env::remove_var("CARGO_MANIFEST_DIR") };
        },
    }

    match result {
        Ok(value) => value,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

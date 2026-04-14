use std::path::Path;
use std::sync::{LazyLock, Mutex};

pub static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub fn with_manifest_env<T>(value: Option<&Path>, f: impl FnOnce() -> T) -> T {
    let _guard = ENV_LOCK.lock().expect("lock poisoned");
    let previous = std::env::var("CARGO_MANIFEST_DIR").ok();

    match value {
        Some(path) => {
            unsafe { std::env::set_var("CARGO_MANIFEST_DIR", path) };
        },
        None => {
            unsafe { std::env::remove_var("CARGO_MANIFEST_DIR") };
        },
    }

    let result = f();

    match previous {
        Some(prev) => {
            unsafe { std::env::set_var("CARGO_MANIFEST_DIR", prev) };
        },
        None => {
            unsafe { std::env::remove_var("CARGO_MANIFEST_DIR") };
        },
    }

    result
}

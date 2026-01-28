#![doc = include_str!("../README.md")]

#[doc(hidden)]
pub mod meta;

#[doc(hidden)]
pub mod registry;

#[cfg(feature = "derive")]
pub use es_fluent_derive::{EsFluent, EsFluentChoice, EsFluentThis, EsFluentVariants};

#[doc(hidden)]
pub use fluent_bundle::FluentValue;

#[doc(hidden)]
pub use inventory as __inventory;

/// Extracts the file stem from a file path at compile time.
/// Used by derive macros when `namespace = file` is specified.
#[doc(hidden)]
pub const fn __namespace_from_file_path(path: &str) -> &str {
    let bytes = path.as_bytes();
    let len = bytes.len();

    // Find last '/' or '\\'
    let mut last_slash = 0;
    let mut i = 0;
    while i < len {
        if bytes[i] == b'/' || bytes[i] == b'\\' {
            last_slash = i + 1;
        }
        i += 1;
    }

    // Find last '.'
    let mut last_dot = len;
    i = last_slash;
    while i < len {
        if bytes[i] == b'.' {
            last_dot = i;
        }
        i += 1;
    }

    // If last_dot is at the start of filename (hidden file), use full name
    if last_dot == last_slash {
        last_dot = len;
    }

    // Extract substring [last_slash..last_dot]
    // SAFETY: We're working with valid UTF-8 and only splitting at ASCII boundaries
    let result_len = last_dot - last_slash;
    if result_len == 0 {
        return "unknown";
    }

    // Use unsafe to create the substring at compile time
    unsafe {
        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
            bytes.as_ptr().add(last_slash),
            result_len,
        ))
    }
}

/// Extracts the relative path (without extension, stripping src/ prefix) from a file path at compile time.
/// Used by derive macros when `namespace = file(relative)` is specified.
#[doc(hidden)]
pub const fn __namespace_from_file_path_relative(path: &str) -> &str {
    let bytes = path.as_bytes();
    let len = bytes.len();

    // Find where the extension starts (last '.')
    let mut last_dot = len;
    let mut i = 0;
    while i < len {
        if bytes[i] == b'.' {
            last_dot = i;
        }
        i += 1;
    }

    // Check for "src/" or "src\" prefix to skip
    let start = if len >= 4
        && bytes[0] == b's'
        && bytes[1] == b'r'
        && bytes[2] == b'c'
        && (bytes[3] == b'/' || bytes[3] == b'\\')
    {
        4
    } else {
        0
    };

    let result_len = last_dot - start;
    if result_len == 0 {
        return "unknown";
    }

    // SAFETY: We're working with valid UTF-8 and only splitting at ASCII boundaries
    unsafe {
        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
            bytes.as_ptr().add(start),
            result_len,
        ))
    }
}

#[doc(hidden)]
pub use rust_embed as __rust_embed;

#[doc(hidden)]
pub use es_fluent_manager_core as __manager_core;

#[doc(hidden)]
pub use unic_langid;

use arc_swap::ArcSwap;
use es_fluent_manager_core::FluentManager;
use std::sync::{Arc, OnceLock};

mod traits;
pub use traits::{EsFluentChoice, FluentDisplay, ThisFtl, ToFluentString};

#[doc(hidden)]
static CONTEXT: OnceLock<ArcSwap<FluentManager>> = OnceLock::new();

#[doc(hidden)]
static CUSTOM_LOCALIZER: OnceLock<
    Box<
        dyn Fn(&str, Option<&std::collections::HashMap<&str, FluentValue>>) -> Option<String>
            + Send
            + Sync,
    >,
> = OnceLock::new();

/// Sets the global `FluentManager` context.
///
/// This function should be called once at the beginning of your application's
/// lifecycle.
///
/// # Panics
///
/// This function will panic if the context has already been set.
#[doc(hidden)]
pub fn set_context(manager: FluentManager) {
    CONTEXT
        .set(ArcSwap::from_pointee(manager))
        .map_err(|_| "Context already set")
        .expect("Failed to set context");
}

/// Sets the global `FluentManager` context with a shared `ArcSwap<FluentManager>`.
///
/// This function is useful when you want to share the `FluentManager` between
/// multiple threads.
///
/// # Panics
///
/// This function will panic if the context has already been set.
#[doc(hidden)]
pub fn set_shared_context(manager: Arc<FluentManager>) {
    CONTEXT
        .set(ArcSwap::new(manager))
        .map_err(|_| "Context already set")
        .expect("Failed to set shared context");
}

/// Sets a custom localizer function.
///
/// The custom localizer will be called before the global context's `localize`
/// method. If the custom localizer returns `Some(message)`, the message will be
/// returned. Otherwise, the global context will be used.
///
/// # Panics
///
/// This function will panic if the custom localizer has already been set.
#[doc(hidden)]
pub fn set_custom_localizer<F>(localizer: F)
where
    F: Fn(&str, Option<&std::collections::HashMap<&str, FluentValue>>) -> Option<String>
        + Send
        + Sync
        + 'static,
{
    CUSTOM_LOCALIZER
        .set(Box::new(localizer))
        .map_err(|_| "Custom localizer already set")
        .expect("Failed to set custom localizer");
}

/// Selects a language for all localizers in the global context.
#[doc(hidden)]
pub fn select_language(lang: &unic_langid::LanguageIdentifier) {
    if let Some(context) = CONTEXT.get() {
        context.load().select_language(lang);
    }
}

/// Localizes a message by its ID.
///
/// This function will first try to use the custom localizer if it has been set.
/// If the custom localizer returns `None`, it will then try to use the global
/// context.
///
/// If the message is not found, a warning will be logged and the ID will be
/// returned as the message.
#[doc(hidden)]
pub fn localize<'a>(
    id: &str,
    args: Option<&std::collections::HashMap<&str, FluentValue<'a>>>,
) -> String {
    if let Some(custom_localizer) = CUSTOM_LOCALIZER.get()
        && let Some(message) = custom_localizer(id, args)
    {
        return message;
    }

    if let Some(context) = CONTEXT.get()
        && let Some(message) = context.load().localize(id, args)
    {
        return message;
    }

    tracing::warn!("Translation for '{}' not found or context not set.", id);
    id.to_string()
}

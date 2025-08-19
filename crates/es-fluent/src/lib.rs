// In: crates/es-fluent/src/lib.rs

#[cfg(feature = "derive")]
pub use es_fluent_derive::{EsFluent, EsFluentChoice};

pub use es_fluent_manager_core::{FluentManager, I18nModule, LocalizationError, Localizer};

pub use fluent_bundle::FluentValue;

mod traits;
pub use traits::*;

use std::sync::{Arc, OnceLock, RwLock};

// Global context for derive macros
static CONTEXT: OnceLock<Arc<RwLock<FluentManager>>> = OnceLock::new();

// Alternative localization function for custom backends (like Bevy)
static CUSTOM_LOCALIZER: OnceLock<
    Box<
        dyn Fn(&str, Option<&std::collections::HashMap<&str, FluentValue>>) -> String + Send + Sync,
    >,
> = OnceLock::new();

/// Sets the global FluentManager context for use with derive macros.
/// This is only needed when using the derive macros.
pub fn set_context(manager: FluentManager) {
    CONTEXT
        .set(Arc::new(RwLock::new(manager)))
        .map_err(|_| "Context already set")
        .expect("Failed to set context");
}

/// Sets the global FluentManager context using a shared Arc<RwLock<FluentManager>>.
/// This allows sharing the same manager instance between the global context and other resources.
/// This is only needed when using the derive macros.
pub fn set_shared_context(manager: Arc<RwLock<FluentManager>>) {
    CONTEXT
        .set(manager)
        .map_err(|_| "Context already set")
        .expect("Failed to set shared context");
}

/// Sets a custom localization function for use with derive macros.
/// This allows integration with alternative backends like Bevy's asset system.
/// This is only needed when using the derive macros.
pub fn set_custom_localizer<F>(localizer: F)
where
    F: Fn(&str, Option<&std::collections::HashMap<&str, FluentValue>>) -> String
        + Send
        + Sync
        + 'static,
{
    CUSTOM_LOCALIZER
        .set(Box::new(localizer))
        .map_err(|_| "Custom localizer already set")
        .expect("Failed to set custom localizer");
}

/// Updates the global FluentManager context for use with derive macros.
/// This is only needed when using the derive macros.
pub fn update_context<F>(f: F)
where
    F: FnOnce(&mut FluentManager),
{
    if let Some(context_arc) = CONTEXT.get() {
        let mut context = context_arc
            .write()
            .expect("Failed to acquire write lock on context");
        f(&mut context);
    }
}

/// Localizes a message ID with optional arguments using the global context.
/// This function is used by the derive macros.
/// For the new API, users should call `manager.localize()` directly.
pub fn localize<'a>(
    id: &str,
    args: Option<&std::collections::HashMap<&str, FluentValue<'a>>>,
) -> String {
    // Try custom localizer first (for backends like Bevy)
    if let Some(custom_localizer) = CUSTOM_LOCALIZER.get() {
        return custom_localizer(id, args);
    }

    // Fall back to FluentManager context
    let context_arc = CONTEXT
        .get()
        .expect("Context not set. Call es_fluent::set_context() or es_fluent::set_custom_localizer() before using derive macros.");

    let context = context_arc
        .read()
        .expect("Failed to acquire read lock on context");

    context.localize(id, args).unwrap_or_else(|| {
        log::warn!("Translation for '{}' not found or context not set.", id);
        id.to_string()
    })
}

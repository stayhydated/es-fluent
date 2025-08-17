// In: crates/es-fluent/src/lib.rs

#[cfg(feature = "derive")]
pub use es_fluent_derive::{EsFluent, EsFluentChoice};

pub use es_fluent_manager_core::localization::{FluentManager, Localizer, I18nModule, LocalizationError};

pub use fluent_bundle::FluentValue;

mod traits;
pub use traits::*;

use std::sync::{Arc, RwLock, OnceLock};

// Global context for derive macros
static CONTEXT: OnceLock<Arc<RwLock<FluentManager>>> = OnceLock::new();

/// Sets the global FluentManager context for use with derive macros.
/// This is only needed when using the derive macros.
pub fn set_context(manager: FluentManager) {
    CONTEXT.set(Arc::new(RwLock::new(manager)))
        .map_err(|_| "Context already set")
        .expect("Failed to set context");
}

/// Updates the global FluentManager context for use with derive macros.
/// This is only needed when using the derive macros.
pub fn update_context<F>(f: F) 
where 
    F: FnOnce(&mut FluentManager)
{
    if let Some(context_arc) = CONTEXT.get() {
        let mut context = context_arc.write()
            .expect("Failed to acquire write lock on context");
        f(&mut context);
    }
}

/// Localizes a message ID with optional arguments using the global context.
/// This function is used by the derive macros.
/// For the new API, users should call `manager.localize()` directly.
pub fn localize<'a>(
    id: &str,
    args: Option<&std::collections::HashMap<&str, FluentValue<'a>>>
) -> String {
    let context_arc = CONTEXT.get()
        .expect("Context not set. Call es_fluent::set_context() before using derive macros.");

    let context = context_arc.read()
        .expect("Failed to acquire read lock on context");

    context.localize(id, args)
        .unwrap_or_else(|| {
            log::warn!("Translation for '{}' not found or context not set.", id);
            id.to_string()
        })
}

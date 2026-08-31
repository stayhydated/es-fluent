//! Shared helpers for proc-macro crates built on `es-fluent-derive-core`.

mod crate_path;
mod fallback;
mod test_context;
mod tokens;

pub use crate_path::{ResolvedCratePath, resolve_crate_path, resolve_crate_path_with_self_alias};
pub use fallback::{
    FallbackValidation, FallbackValidationDerive, fallback_validation,
    fallback_validation_for_derive,
};
pub use tokens::{
    core_error_to_compile_error, static_argument_name_tokens, static_domain_tokens,
    static_entry_id_tokens, static_message_key_tokens, static_variant_key_tokens,
};

use test_context::derive_requires_test;

use crate::error::EsFluentCoreError;
use es_fluent_shared::fluent::{
    FluentArgumentName, FluentDomain, FluentMessageId, FluentVariantKey,
};
use es_fluent_shared::resource::{
    FALLBACK_CATALOG_ENV, INVENTORY_RUNNER_ENV, fallback_catalog_contains,
};
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};

#[cfg(test)]
#[serial_test::serial(manifest)]
mod tests {
    include!("macro_support/tests.rs");
}

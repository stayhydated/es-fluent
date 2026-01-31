//! This module provides types for representing FTL variants and type information.

pub use es_fluent_derive_core::registry::{FtlTypeInfo, FtlVariant, NamespaceRule};

/// A wrapper type for `FtlTypeInfo` that enables inventory collection.
/// This is necessary because `inventory::collect!` requires a type defined
/// in the current crate.
#[derive(Debug)]
pub struct RegisteredFtlType(pub &'static FtlTypeInfo);

// Collect all registered FtlTypeInfo from derive macros
inventory::collect!(RegisteredFtlType);

/// Returns an iterator over all registered FTL type infos.
pub fn get_all_ftl_type_infos() -> impl Iterator<Item = &'static FtlTypeInfo> {
    inventory::iter::<RegisteredFtlType>().map(|r| r.0)
}

//! This module provides types for representing FTL variants and type information.

pub use es_fluent_derive_core::registry::{FtlTypeInfo, FtlVariant};

/// A wrapper type for `FtlTypeInfo` that enables inventory collection.
/// This is necessary because `FtlTypeInfo` is defined in a different crate,
/// and Rust's orphan rules prevent implementing foreign traits on foreign types.
#[derive(Debug)]
pub struct RegisteredFtlType(pub &'static FtlTypeInfo);

// Collect all registered FtlTypeInfo from derive macros
inventory::collect!(RegisteredFtlType);

/// Returns an iterator over all registered FTL type infos.
pub fn get_all_ftl_type_infos() -> impl Iterator<Item = &'static FtlTypeInfo> {
    inventory::iter::<RegisteredFtlType>().map(|r| r.0)
}

//! This module provides types for representing FTL variants and type information.

use crate::meta::TypeKind;
use crate::namer::FluentKey;
use bon::Builder;

/// A variant of an FTL type.
#[derive(Builder, Clone, Debug, Eq, Hash, PartialEq, serde::Serialize)]
pub struct FtlVariant {
    /// The name of the variant.
    pub name: String,
    /// The FTL key of the variant.
    pub ftl_key: FluentKey,
    /// The arguments of the variant.
    pub arguments: Option<Vec<String>>,
}

/// Information about an FTL type.
#[derive(Builder, Clone, Debug, Eq, Hash, PartialEq, serde::Serialize)]
pub struct FtlTypeInfo {
    /// The kind of the type.
    pub type_kind: TypeKind,
    /// The name of the type.
    pub type_name: String,
    /// The variants of the type.
    pub variants: Vec<FtlVariant>,
}

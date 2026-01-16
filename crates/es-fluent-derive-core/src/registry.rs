//! This module provides types for representing FTL variants and type information.

use crate::meta::TypeKind;
use bon::Builder;

#[derive(Builder, Clone, Debug, Eq, Hash, PartialEq, serde::Serialize)]
pub struct FtlVariant {
    pub name: String,
    pub ftl_key: String,
    #[builder(default)]
    pub args: Vec<String>,
    /// The module path where this type is defined.
    #[builder(default)]
    pub module_path: String,
    /// The line number where this variant is defined in the Rust source.
    #[builder(default)]
    pub line: u32,
}

#[derive(Builder, Clone, Debug, Eq, Hash, PartialEq, serde::Serialize)]
pub struct FtlTypeInfo {
    pub type_kind: TypeKind,

    pub type_name: String,

    pub variants: Vec<FtlVariant>,

    /// The file path where this type is defined.
    pub file_path: Option<String>,

    /// The module path where this type is defined.
    pub module_path: String,
}

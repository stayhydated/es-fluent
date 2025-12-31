//! This module provides types for representing FTL variants and type information.

use crate::meta::TypeKind;
use bon::Builder;

#[derive(Builder, Clone, Debug, Eq, Hash, PartialEq, serde::Serialize)]
pub struct FtlVariant {
    pub name: String,
    pub ftl_key: crate::namer::FluentKey,
    #[builder(default)]
    pub args: Vec<String>,
    /// The module path where this type is defined.
    #[builder(default)]
    pub module_path: String,
    /// Whether this variant is a "this" variant (represents the type itself).
    #[builder(default)]
    pub is_this: bool,
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

    /// Whether this type info is for a "this" type (e.g., `Foo_this`).
    #[builder(default)]
    pub is_this: bool,
}

// Static versions for inventory submission from derive macros
// These use &'static str and &'static [..] to be const-constructible

/// A static variant of `FtlVariant` that can be constructed at compile time.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StaticFtlVariant {
    pub name: &'static str,
    pub ftl_key: &'static str,
    pub args: &'static [&'static str],
    /// The module path from `module_path!()`.
    pub module_path: &'static str,
    /// Whether this variant is a "this" variant (represents the type itself).
    pub is_this: bool,
}

/// A static variant of `FtlTypeInfo` that can be constructed at compile time
/// and submitted to inventory.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StaticFtlTypeInfo {
    pub type_kind: TypeKind,
    pub type_name: &'static str,
    pub variants: &'static [StaticFtlVariant],
    /// The file path where this type is defined (from `file!()` macro).
    pub file_path: &'static str,
    /// The module path where this type is defined (from `module_path!()` macro).
    pub module_path: &'static str,
    /// Whether this type info is for a "this" type (e.g., `Foo_this`).
    pub is_this: bool,
}

// Collect all registered FtlTypeInfo from derive macros
inventory::collect!(&'static StaticFtlTypeInfo);

/// Returns an iterator over all registered static FTL type infos.
pub fn get_all_static_ftl_type_infos() -> impl Iterator<Item = &'static &'static StaticFtlTypeInfo>
{
    inventory::iter::<&'static StaticFtlTypeInfo>()
}

/// Converts a `StaticFtlTypeInfo` to an owned `FtlTypeInfo`.
impl From<&StaticFtlTypeInfo> for FtlTypeInfo {
    fn from(static_info: &StaticFtlTypeInfo) -> Self {
        FtlTypeInfo {
            type_kind: static_info.type_kind.clone(),
            type_name: static_info.type_name.to_string(),
            variants: static_info
                .variants
                .iter()
                .map(|v| FtlVariant {
                    name: v.name.to_string(),
                    ftl_key: crate::namer::FluentKey(v.ftl_key.to_string()),
                    args: v.args.iter().map(|s| s.to_string()).collect(),
                    module_path: v.module_path.to_string(),
                    is_this: v.is_this,
                })
                .collect(),
            file_path: Some(static_info.file_path.to_string()),
            module_path: static_info.module_path.to_string(),
            is_this: static_info.is_this,
        }
    }
}

/// Returns all registered FTL type infos as owned `FtlTypeInfo` instances.
pub fn get_all_ftl_type_infos() -> Vec<FtlTypeInfo> {
    get_all_static_ftl_type_infos()
        .map(|info| FtlTypeInfo::from(*info))
        .collect()
}

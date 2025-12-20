//! This module provides functions for analyzing Rust source code and extracting
//! information about types that can be translated with `es-fluent`.

pub mod r#enum;
pub mod enum_kv;
pub mod r#struct;
pub mod struct_kv;

use crate::error::EsFluentCoreResult;
use crate::options::r#enum::{EnumKvOpts, EnumOpts};
use crate::options::r#struct::StructOpts;
use crate::registry::FtlTypeInfo;

/// Analyzes a struct and returns a list of `FtlTypeInfo` objects.
pub fn analyze_struct(opts: &StructOpts) -> Vec<FtlTypeInfo> {
    let mut type_infos = Vec::new();

    r#struct::analyze_struct(opts, &mut type_infos);

    type_infos
}

/// Analyzes an enum and returns a list of `FtlTypeInfo` objects.
pub fn analyze_enum(opts: &EnumOpts) -> Vec<FtlTypeInfo> {
    let mut type_infos = Vec::new();

    r#enum::analyze_enum(opts, &mut type_infos);

    type_infos
}

/// Analyzes an enum with EsFluentKv and returns a list of `FtlTypeInfo` objects.
pub fn analyze_enum_kv(opts: &EnumKvOpts) -> EsFluentCoreResult<Vec<FtlTypeInfo>> {
    let mut type_infos = Vec::new();

    enum_kv::analyze_enum_kv(opts, &mut type_infos)?;

    Ok(type_infos)
}

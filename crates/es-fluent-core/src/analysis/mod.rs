//! This module provides functions for analyzing Rust source code and extracting
//! information about types that can be translated with `es-fluent`.

mod r#enum;
mod r#struct;

use crate::options::r#enum::EnumOpts;
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

mod r#enum;
mod r#struct;

use crate::options::r#enum::EnumOpts;
use crate::options::r#struct::StructOpts;
use crate::registry::FtlTypeInfo;

pub fn analyze_struct(opts: &StructOpts) -> Vec<FtlTypeInfo> {
    let mut type_infos = Vec::new();

    r#struct::analyze_struct(opts, &mut type_infos);

    type_infos
}

pub fn analyze_enum(opts: &EnumOpts) -> Vec<FtlTypeInfo> {
    let mut type_infos = Vec::new();

    r#enum::analyze_enum(opts, &mut type_infos);

    type_infos
}

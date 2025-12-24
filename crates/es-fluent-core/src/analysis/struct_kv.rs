use crate::meta::TypeKind;
use crate::namer;
use crate::options::r#struct::StructKvOpts;
use crate::registry::{FtlTypeInfo, FtlVariant};

use crate::error::EsFluentCoreResult;

pub fn analyze_struct_kv(
    opts: &StructKvOpts,
    type_infos: &mut Vec<FtlTypeInfo>,
) -> EsFluentCoreResult<()> {
    let target_ident = opts.ident();
    let keyyed_idents = opts.keyyed_idents()?;
    let has_keys = !keyyed_idents.is_empty();
    // `this` generates this_ftl on the original struct type
    let keyyed_idents = opts.keyyed_idents()?;
    let has_keys = !keyyed_idents.is_empty();

    let field_names: Vec<String> = opts
        .fields()
        .iter()
        .filter_map(|field_opt| {
            field_opt
                .ident()
                .as_ref()
                .map(|field_ident| field_ident.to_string())
        })
        .collect();

    // For empty structs, only generate the `this` variant if is_this is set
    if field_names.is_empty() {
        return Ok(());
    }

    if keyyed_idents.is_empty() {
        let ftl_enum_ident = opts.ftl_enum_ident();
        let mut variants: Vec<FtlVariant> = field_names
            .iter()
            .map(|name_str| {
                let ftl_key = namer::FluentKey::new(&ftl_enum_ident, name_str);
                FtlVariant::builder()
                    .name(name_str.clone())
                    .ftl_key(ftl_key)
                    .build()
            })
            .collect();

        log::debug!(
            "Generating FtlTypeInfo ({}) for '{}' (keys based on '{}') during {}",
            TypeKind::Enum,
            ftl_enum_ident,
            target_ident,
            "struct_kv analysis (FTL enum generation)"
        );
        type_infos.push(
            FtlTypeInfo::builder()
                .type_kind(TypeKind::Enum)
                .type_name(ftl_enum_ident.to_string())
                .variants(variants)
                .build(),
        );
    } else {
        for keyyed_ident in keyyed_idents {
            let mut variants: Vec<FtlVariant> = field_names
                .iter()
                .map(|name_str| {
                    let ftl_key = namer::FluentKey::new(&keyyed_ident, name_str);
                    FtlVariant::builder()
                        .name(name_str.clone())
                        .ftl_key(ftl_key)
                        .build()
                })
                .collect();

            log::debug!(
                "Generating FtlTypeInfo ({}) for '{}' (keys based on '{}') during {}",
                TypeKind::Enum,
                keyyed_ident,
                target_ident,
                "struct_kv analysis (FTL enum generation with keys)"
            );
            type_infos.push(
                FtlTypeInfo::builder()
                    .type_kind(TypeKind::Enum)
                    .type_name(keyyed_ident.to_string())
                    .variants(variants)
                    .build(),
            );
        }
    }

    Ok(())
}

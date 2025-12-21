use crate::meta::TypeKind;
use crate::namer;
use crate::options::r#enum::EnumKvOpts;
use crate::registry::{FtlTypeInfo, FtlVariant};

use crate::error::EsFluentCoreResult;

pub fn analyze_enum_kv(
    opts: &EnumKvOpts,
    type_infos: &mut Vec<FtlTypeInfo>,
) -> EsFluentCoreResult<()> {
    let target_ident = opts.ident();
    let keyyed_idents = opts.keyyed_idents()?;
    let has_keys = !keyyed_idents.is_empty();
    // `this` generates this_ftl on the original type (e.g., Country)
    let is_this = opts.attr_args().is_this();
    // `keys_this` generates this_ftl on the generated KV enums (e.g., CountryLabelKvFtl)
    let is_keys_this = opts.attr_args().is_keys_this();

    let variant_names: Vec<String> = opts
        .variants()
        .iter()
        .map(|variant_opt| variant_opt.ident().to_string())
        .collect();

    // For empty enums, only generate the `this` variant if is_this is set
    if variant_names.is_empty() {
        if is_this {
            let this_ident = quote::format_ident!("{}_this", target_ident);
            let main_ftl_key = namer::FluentKey::new(&this_ident, "");
            let main_variant = FtlVariant::builder()
                .name(target_ident.to_string())
                .ftl_key(main_ftl_key)
                .build();

            log::debug!(
                "Generating FtlTypeInfo ({}) for empty enum '{}' with this during {}",
                TypeKind::Enum,
                target_ident,
                "enum_kv analysis"
            );

            type_infos.push(
                FtlTypeInfo::builder()
                    .type_kind(TypeKind::Enum)
                    .type_name(target_ident.to_string())
                    .variants(vec![main_variant])
                    .build(),
            );
        }
        return Ok(());
    }

    if keyyed_idents.is_empty() {
        let ftl_enum_ident = opts.ftl_enum_ident();
        let mut variants: Vec<FtlVariant> = variant_names
            .iter()
            .map(|name_str| {
                let ftl_key = namer::FluentKey::new(&ftl_enum_ident, name_str);
                FtlVariant::builder()
                    .name(name_str.clone())
                    .ftl_key(ftl_key)
                    .build()
            })
            .collect();

        if is_keys_this {
            let this_ident = quote::format_ident!("{}_this", ftl_enum_ident);
            let this_ftl_key = namer::FluentKey::new(&this_ident, "");
            let this_variant = FtlVariant::builder()
                .name(ftl_enum_ident.to_string())
                .ftl_key(this_ftl_key)
                .build();
            variants.insert(0, this_variant);
        }

        log::debug!(
            "Generating FtlTypeInfo ({}) for '{}' (keys based on '{}') during {}",
            TypeKind::Enum,
            ftl_enum_ident,
            target_ident,
            "enum_kv analysis (FTL enum generation)"
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
            let mut variants: Vec<FtlVariant> = variant_names
                .iter()
                .map(|name_str| {
                    let ftl_key = namer::FluentKey::new(&keyyed_ident, name_str);
                    FtlVariant::builder()
                        .name(name_str.clone())
                        .ftl_key(ftl_key)
                        .build()
                })
                .collect();

            if is_keys_this {
                let this_ident = quote::format_ident!("{}_this", keyyed_ident);
                let this_ftl_key = namer::FluentKey::new(&this_ident, "");
                let this_variant = FtlVariant::builder()
                    .name(keyyed_ident.to_string())
                    .ftl_key(this_ftl_key)
                    .build();
                variants.insert(0, this_variant);
            }

            log::debug!(
                "Generating FtlTypeInfo ({}) for '{}' (keys based on '{}') during {}",
                TypeKind::Enum,
                keyyed_ident,
                target_ident,
                "enum_kv analysis (FTL enum generation with keys)"
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

    // Generate this_ftl for the original type only if this is set
    if is_this {
        let this_ident = quote::format_ident!("{}_this", target_ident);
        let main_ftl_key = namer::FluentKey::new(&this_ident, "");
        let main_variant = FtlVariant::builder()
            .name(target_ident.to_string())
            .ftl_key(main_ftl_key)
            .build();

        log::debug!(
            "Generating FtlTypeInfo ({}) for '{}' (keys based on '{}') during {}",
            TypeKind::Enum,
            target_ident,
            target_ident,
            if has_keys {
                "enum_kv analysis (main enum variant with this)"
            } else {
                "enum_kv analysis (main enum variant with this, no keys)"
            }
        );
        type_infos.push(
            FtlTypeInfo::builder()
                .type_kind(TypeKind::Enum)
                .type_name(target_ident.to_string())
                .variants(vec![main_variant])
                .build(),
        );
    }

    Ok(())
}

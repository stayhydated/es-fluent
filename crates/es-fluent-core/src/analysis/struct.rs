use crate::meta::TypeKind;
use crate::namer;
use crate::options::r#struct::StructOpts;
use crate::registry::{FtlTypeInfo, FtlVariant};

pub fn analyze_struct(opts: &StructOpts, type_infos: &mut Vec<FtlTypeInfo>) {
    let target_ident = opts.ident();
    let keyyed_idents = opts.keyyed_idents();
    let is_this = opts.attr_args().is_this();

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

    if field_names.is_empty() {
        return;
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

        if is_this {
            let this_ftl_key = namer::FluentKey::new(&ftl_enum_ident, "");
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
            "struct analysis (FTL enum generation)"
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

            if is_this {
                let this_ftl_key = namer::FluentKey::new(&keyyed_ident, "");
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
                "struct analysis (FTL enum generation with keys)"
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
}

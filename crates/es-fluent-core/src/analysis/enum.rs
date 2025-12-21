use crate::meta::TypeKind;
use crate::namer;
use crate::options::r#enum::EnumOpts;
use crate::registry::{FtlTypeInfo, FtlVariant};

pub fn analyze_enum(opts: &EnumOpts, type_infos: &mut Vec<FtlTypeInfo>) {
    let target_ident = opts.ident();
    let target_name = target_ident.to_string();
    let opts_variants = opts.variants();
    let is_this = opts.attr_args().is_this();
    let base_key = opts.base_key();

    let mut unit_variants: Vec<FtlVariant> = opts_variants
        .iter()
        .filter_map(|variant_opt| {
            if variant_opt.is_skipped() {
                return None;
            }
            if matches!(variant_opt.style(), darling::ast::Style::Unit) {
                let name_str = variant_opt.ident().to_string();
                let key_suffix = variant_opt
                    .key()
                    .map(|k| k.to_string())
                    .unwrap_or_else(|| name_str.clone());
                let ftl_key = namer::FluentKey::with_base(&base_key, &key_suffix);
                Some(
                    FtlVariant::builder()
                        .name(name_str)
                        .ftl_key(ftl_key)
                        .build(),
                )
            } else {
                None
            }
        })
        .collect();

    if is_this {
        let this_base_key = format!("{}_this", base_key);
        let this_ftl_key = namer::FluentKey::with_base(&this_base_key, "");
        let this_variant = FtlVariant::builder()
            .name(target_name.clone())
            .ftl_key(this_ftl_key)
            .build();
        unit_variants.insert(0, this_variant);
    }

    if !unit_variants.is_empty() {
        log::debug!(
            "Generating FtlTypeInfo ({}) for '{}' (keys based on '{}') during {}",
            TypeKind::Enum,
            target_name,
            target_ident,
            "enum analysis (thiserror mode, unit variants)"
        );
        type_infos.push(
            FtlTypeInfo::builder()
                .type_kind(TypeKind::Enum)
                .type_name(target_name.clone())
                .variants(unit_variants)
                .build(),
        );
    }

    let mut struct_variants: Vec<FtlVariant> = opts_variants
        .iter()
        .filter_map(|variant_opt| {
            if variant_opt.is_skipped() {
                return None;
            }
            if matches!(variant_opt.style(), darling::ast::Style::Unit) {
                return None;
            }

            let name_str = variant_opt.ident().to_string();
            let arguments = match variant_opt.style() {
                darling::ast::Style::Struct => Some(
                    variant_opt
                        .fields()
                        .iter()
                        .filter_map(|f| f.ident().as_ref().map(|id| id.to_string()))
                        .collect(),
                ),
                darling::ast::Style::Tuple => Some(
                    variant_opt
                        .all_fields()
                        .iter()
                        .enumerate()
                        .filter_map(|(index, field)| {
                            if !field.is_skipped() {
                                Some(namer::UnnamedItem::from(index).to_string())
                            } else {
                                None
                            }
                        })
                        .collect(),
                ),
                darling::ast::Style::Unit => {
                    unreachable!("Unit variants should have been filtered out")
                },
            };

            let key_suffix = variant_opt
                .key()
                .map(|k| k.to_string())
                .unwrap_or_else(|| name_str.clone());
            let ftl_key = namer::FluentKey::with_base(&base_key, &key_suffix);
            Some(
                FtlVariant::builder()
                    .name(name_str)
                    .ftl_key(ftl_key)
                    .args(arguments.unwrap_or_default())
                    .build(),
            )
        })
        .collect();

    if is_this {
        let this_base_key = format!("{}_this", base_key);
        let this_ftl_key = namer::FluentKey::with_base(&this_base_key, "");
        let this_variant = FtlVariant::builder()
            .name(target_name.clone())
            .ftl_key(this_ftl_key)
            .build();
        struct_variants.insert(0, this_variant);
    }

    if !struct_variants.is_empty() {
        log::debug!(
            "Generating FtlTypeInfo ({}) for '{}' (keys based on '{}') during {}",
            TypeKind::Struct,
            target_name,
            target_ident,
            "enum analysis (struct/tuple variants)"
        );
        type_infos.push(
            FtlTypeInfo::builder()
                .type_kind(TypeKind::Struct)
                .type_name(target_name)
                .variants(struct_variants)
                .build(),
        );
    }
}

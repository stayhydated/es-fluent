use crate::meta::TypeKind;
use crate::namer;
use crate::options::r#struct::StructOpts;
use crate::registry::{FtlTypeInfo, FtlVariant};

pub fn analyze_struct(opts: &StructOpts, type_infos: &mut Vec<FtlTypeInfo>) {
    let target_ident = opts.ident();
    let is_this = opts.attr_args().is_this();

    let field_names: Vec<String> = opts
        .indexed_fields()
        .into_iter()
        .map(|(index, field_opt)| field_opt.fluent_arg_name(index))
        .collect();

    // For empty structs, only generate the `this` variant if is_this is set
    if field_names.is_empty() {
        if is_this {
            let this_ident = quote::format_ident!("{}_this", target_ident);
            let this_ftl_key = namer::FluentKey::new(&this_ident, "");
            let this_variant = FtlVariant::builder()
                .name(target_ident.to_string())
                .ftl_key(this_ftl_key)
                .build();

            log::debug!(
                "Generating FtlTypeInfo ({}) for empty struct '{}' with this during {}",
                TypeKind::Struct,
                target_ident,
                "struct analysis"
            );

            type_infos.push(
                FtlTypeInfo::builder()
                    .type_kind(TypeKind::Struct)
                    .type_name(target_ident.to_string())
                    .variants(vec![this_variant])
                    .build(),
            );
        }
        return;
    }

    let mut variants = Vec::new();
    if is_this {
        let this_ident = quote::format_ident!("{}_this", target_ident);
        let this_ftl_key = namer::FluentKey::new(&this_ident, "");
        let this_variant = FtlVariant::builder()
            .name(target_ident.to_string())
            .ftl_key(this_ftl_key)
            .build();
        variants.push(this_variant);
    }

    let ftl_key = namer::FluentKey::new(target_ident, "");
    let main_variant = FtlVariant::builder()
        .name(target_ident.to_string())
        .ftl_key(ftl_key)
        .args(field_names)
        .build();
    variants.push(main_variant);

    log::debug!(
        "Generating FtlTypeInfo ({}) for '{}' during {}",
        TypeKind::Struct,
        target_ident,
        "struct analysis"
    );

    type_infos.push(
        FtlTypeInfo::builder()
            .type_kind(TypeKind::Struct)
            .type_name(target_ident.to_string())
            .variants(variants)
            .build(),
    );
}

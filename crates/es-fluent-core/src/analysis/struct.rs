use crate::meta::TypeKind;
use crate::namer;
use crate::options::r#struct::StructOpts;
use crate::registry::{FtlTypeInfo, FtlVariant};

pub fn analyze_struct(opts: &StructOpts, type_infos: &mut Vec<FtlTypeInfo>) {
    let target_ident = opts.ident();
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

    let mut variants = Vec::new();
    if is_this {
        let this_ftl_key = namer::FluentKey::new(target_ident, "");
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

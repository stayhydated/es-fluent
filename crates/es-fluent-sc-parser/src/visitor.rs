use crate::processor::{FluentProcessKind as _, FtlProcessor};
use darling::{FromDeriveInput as _, FromMeta};
use es_fluent_core::{
    meta::{EnumKind, StructKind, TypeKind},
    namer,
    options::r#enum::EnumOpts,
    registry::{FtlTypeInfo, FtlVariant},
};
use getset::Getters;
use quote::{ToTokens as _, format_ident};
use std::path::Path;
use syn::visit::Visit;
use syn::{Data, DeriveInput, Item, parse::ParseStream};

#[derive(Debug, Default, FromMeta)]
#[darling(allow_unknown_fields)]
struct StrumDiscriminantsArgs {
    #[darling(default)]
    derive: darling::util::PathList,
}

#[derive(Default, Getters)]
pub struct FtlVisitor {
    current_file: std::path::PathBuf,
    #[getset(get = "pub")]
    type_infos: Vec<FtlTypeInfo>,
}

impl FtlVisitor {
    pub fn new(current_file: &std::path::Path) -> Self {
        Self {
            current_file: current_file.to_path_buf(),
            type_infos: Vec::new(),
        }
    }

    fn process_enum_discriminants(&mut self, derive_input: &DeriveInput) {
        let strum_attrs: Vec<_> = derive_input
            .attrs
            .iter()
            .filter(|a| a.path().is_ident("strum_discriminants"))
            .collect();

        if strum_attrs.is_empty() {
            return;
        }

        let mut all_derive_paths = Vec::new();

        for attr in strum_attrs {
            let args = match StrumDiscriminantsArgs::from_meta(&attr.meta) {
                Ok(args) => args,
                Err(e) => {
                    log::warn!(
                        "In file '{}': Could not parse `strum_discriminants` attribute for enum '{}': {}. Skipping this attribute.",
                        self.current_file.display(),
                        derive_input.ident,
                        e
                    );
                    continue;
                },
            };

            all_derive_paths.extend(args.derive.iter().cloned());
        }

        let derives_es_fluent = all_derive_paths
            .iter()
            .any(|p| p.segments.last().is_some_and(|s| s.ident == "EsFluent"));

        if derives_es_fluent {
            let enum_opts = match EnumOpts::from_derive_input(derive_input) {
                Ok(opts) => opts,
                Err(e) => {
                    log::error!(
                        "In file '{}': Error parsing enum opts for '{}' for strum discriminant check: {}. Skipping.",
                        self.current_file.display(),
                        derive_input.ident,
                        e
                    );
                    return;
                },
            };

            let original_ident = enum_opts.ident();
            let discriminant_ident = format_ident!("{}Discriminants", original_ident);
            let discriminant_ident_str = discriminant_ident.to_string();

            let mut variants: Vec<FtlVariant> = enum_opts
                .variants()
                .iter()
                .filter_map(|variant_opt| {
                    if variant_opt.is_skipped() {
                        return None;
                    }
                    let name_str = variant_opt.ident().to_string();
                    let ftl_key = namer::FluentKey::new(&discriminant_ident, &name_str);
                    Some(
                        FtlVariant::builder()
                            .name(name_str)
                            .ftl_key(ftl_key)
                            .build(),
                    )
                })
                .collect();

            if enum_opts.attr_args().is_this() {
                let this_ftl_key = namer::FluentKey::new(&discriminant_ident, "");
                let this_variant = FtlVariant::builder()
                    .name(discriminant_ident_str.clone())
                    .ftl_key(this_ftl_key)
                    .build();
                variants.push(this_variant);
            }

            if !variants.is_empty() {
                log::debug!(
                    "Generating FtlTypeInfo ({}) for '{}' (keys based on '{}') from strum_discriminants",
                    TypeKind::Enum,
                    discriminant_ident_str,
                    discriminant_ident,
                );
                self.type_infos.push(
                    FtlTypeInfo::builder()
                        .type_kind(TypeKind::Enum)
                        .type_name(discriminant_ident_str)
                        .variants(variants)
                        .build(),
                );
            }
        }
    }
}

impl<'ast> Visit<'ast> for FtlVisitor {
    fn visit_item(&mut self, item: &'ast Item) {
        let mut processed_item = item.clone();

        match &mut processed_item {
            Item::Enum(item_enum) => {
                item_enum.attrs = preprocess_item_attributes(
                    std::mem::take(&mut item_enum.attrs),
                    &self.current_file,
                );
            },
            Item::Struct(item_struct) => {
                item_struct.attrs = preprocess_item_attributes(
                    std::mem::take(&mut item_struct.attrs),
                    &self.current_file,
                );
            },
            Item::Mod(_) => {
                syn::visit::visit_item(self, item);
                return;
            },
            _ => {},
        }

        let has_es_fluent_derive = is_es_fluent_derived(&processed_item);

        let has_strum_discriminants_attr = if let Item::Enum(item_enum) = &processed_item {
            item_enum
                .attrs
                .iter()
                .any(|a| a.path().is_ident("strum_discriminants"))
        } else {
            false
        };

        if !has_es_fluent_derive && !has_strum_discriminants_attr {
            syn::visit::visit_item(self, item);
            return;
        }

        let enum_processor = FtlProcessor::<EnumKind>::builder()
            .current_file(self.current_file.clone())
            .build();
        let struct_processor = FtlProcessor::<StructKind>::builder()
            .current_file(self.current_file.clone())
            .build();

        match processed_item {
            Item::Enum(item_enum) => {
                let derive_input = DeriveInput::from(item_enum);

                if has_es_fluent_derive {
                    if let Data::Enum(_) = derive_input.data {
                        match enum_processor.process(&derive_input) {
                            Ok(type_info) => {
                                self.type_infos.extend(type_info);
                            },
                            Err(e) => {
                                log::error!(
                                    "Error processing enum '{}' in file '{}': {}",
                                    derive_input.ident,
                                    self.current_file.display(),
                                    e
                                );
                            },
                        }
                    } else {
                        unreachable!(
                            "Internal error: Item::Enum expected for enum '{}' in file '{}'",
                            derive_input.ident,
                            self.current_file.display()
                        );
                    }
                }
                if has_strum_discriminants_attr {
                    self.process_enum_discriminants(&derive_input);
                }
            },
            Item::Struct(item_struct) => {
                if !has_es_fluent_derive {
                    return;
                }
                let derive_input = DeriveInput::from(item_struct);

                if let Data::Struct(_) = derive_input.data {
                    match struct_processor.process(&derive_input) {
                        Ok(type_info) => {
                            self.type_infos.extend(type_info);
                        },
                        Err(e) => {
                            log::error!(
                                "Error processing struct '{}' in file '{}': {}",
                                derive_input.ident,
                                self.current_file.display(),
                                e
                            );
                        },
                    }
                } else {
                    unreachable!(
                        "Internal error: Item::Struct expected for struct '{}' in file '{}'",
                        derive_input.ident,
                        self.current_file.display()
                    );
                }
            },
            _ => {},
        }
    }
}

fn is_es_fluent_derived(item: &Item) -> bool {
    let attrs = match item {
        Item::Enum(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        _ => return false,
    };

    for attr in attrs {
        if attr.path().is_ident("derive") {
            if let Ok(paths) = attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
            ) {
                for path in paths {
                    if path.segments.last().is_some_and(|s| s.ident == "EsFluent") {
                        return true;
                    }
                }
            }
        }
    }

    false
}

fn preprocess_item_attributes(
    item_attrs: Vec<syn::Attribute>,
    current_file_path_for_log: &Path,
) -> Vec<syn::Attribute> {
    let mut processed_attrs = Vec::new();

    for attr in item_attrs {
        if attr.path().is_ident("cfg_attr") {
            let cfg_attr_parser = |input: ParseStream| -> syn::Result<Vec<syn::Meta>> {
                let _condition: syn::Meta = input.parse()?;

                let mut inner_metas = Vec::new();
                while !input.is_empty() {
                    input.parse::<syn::Token![,]>()?;
                    if input.is_empty() {
                        break;
                    }
                    let meta: syn::Meta = input.parse()?;
                    inner_metas.push(meta);
                }
                Ok(inner_metas)
            };

            match attr.parse_args_with(cfg_attr_parser) {
                Ok(inner_metas_to_apply) => {
                    for meta_to_apply in inner_metas_to_apply {
                        let new_attr: syn::Attribute = syn::parse_quote!(#[#meta_to_apply]);
                        processed_attrs.push(new_attr);
                    }
                },
                Err(e) => {
                    let path_str = attr
                        .path()
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");
                    log::warn!(
                        "In file '{}': Failed to parse arguments of cfg_attr attribute with path '{}' (tokens: '{}'): {}. Keeping original attribute.",
                        current_file_path_for_log.display(),
                        path_str,
                        attr.clone().into_token_stream(),
                        e
                    );
                    processed_attrs.push(attr);
                },
            }
        } else {
            processed_attrs.push(attr);
        }
    }
    processed_attrs
}

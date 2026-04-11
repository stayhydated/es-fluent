use darling::FromDeriveInput as _;
use es_fluent_derive_core::namer;
use es_fluent_derive_core::options::r#enum::{EnumOpts, EnumVariantsOpts};
use es_fluent_derive_core::options::namespace::NamespaceValue;
use es_fluent_derive_core::options::r#struct::{StructOpts, StructVariantsOpts};
use es_fluent_derive_core::options::this::ThisOpts;

use heck::ToPascalCase as _;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

use crate::macros::utils::{
    InventoryModuleInput, generate_from_impls, generate_inventory_module, generate_this_ftl_impl,
    namespace_rule_tokens,
};

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let this_opts = ThisOpts::from_derive_input(&input).ok();
    let fluent_namespace = match fluent_namespace(&input) {
        Ok(namespace) => namespace,
        Err(err) => return err.write_errors().into(),
    };

    let tokens = match &input.data {
        Data::Struct(_) => {
            let opts = match StructVariantsOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors().into(),
            };

            process_struct(&opts, this_opts.as_ref(), fluent_namespace.as_ref())
        },
        Data::Enum(_) => {
            let opts = match EnumVariantsOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors().into(),
            };

            process_enum(&opts, this_opts.as_ref(), fluent_namespace.as_ref())
        },
        Data::Union(_) => panic!("EsFluentVariants cannot be used on unions"),
    };

    tokens.into()
}

fn fluent_namespace(input: &DeriveInput) -> Result<Option<NamespaceValue>, darling::Error> {
    match &input.data {
        Data::Struct(_) => {
            StructOpts::from_derive_input(input).map(|opts| opts.attr_args().namespace().cloned())
        },
        Data::Enum(_) => {
            EnumOpts::from_derive_input(input).map(|opts| opts.attr_args().namespace().cloned())
        },
        Data::Union(_) => panic!("EsFluentVariants cannot be used on unions"),
    }
}

fn emit_generated_enums(
    default_ident: &syn::Ident,
    keys: &[syn::Ident],
    key_strings: &[String],
    mut emit: impl FnMut(&syn::Ident, Option<&String>) -> TokenStream,
) -> TokenStream {
    if keys.is_empty() {
        return emit(default_ident, None);
    }

    let enums = keys
        .iter()
        .zip(key_strings.iter())
        .map(|(key, key_str)| emit(key, Some(key_str)));

    quote! {
        #(#enums)*
    }
}

pub fn process_struct(
    opts: &StructVariantsOpts,
    this_opts: Option<&ThisOpts>,
    fluent_namespace: Option<&NamespaceValue>,
) -> TokenStream {
    let keys = match opts.keyed_idents() {
        Ok(keys) => keys,
        Err(err) => err.abort(),
    };
    let key_strings = opts.attr_args().key_strings().unwrap_or_default();

    if opts.fields().is_empty() {
        return quote! {};
    }

    let ftl_enum_ident = opts.ftl_enum_ident();
    emit_generated_enums(&ftl_enum_ident, &keys, &key_strings, |ident, key_name| {
        generate_struct_unit_enum(opts, ident, key_name, this_opts, fluent_namespace)
    })
}

#[derive(Clone)]
struct GeneratedVariant {
    ident: syn::Ident,
    doc_name: String,
    ftl_key: String,
}

fn resolve_variants_namespace(
    attr_namespace: Option<&NamespaceValue>,
    this_opts: Option<&ThisOpts>,
    fluent_namespace: Option<&NamespaceValue>,
) -> TokenStream {
    namespace_rule_tokens(
        fluent_namespace
            .or(attr_namespace)
            .or_else(|| this_opts.and_then(|opts| opts.attr_args().namespace())),
    )
}

fn variants_this_key(this_opts: Option<&ThisOpts>, base_key: &namer::FluentKey) -> Option<String> {
    this_opts
        .filter(|opts| opts.attr_args().is_variants())
        .map(|_| format!("{}{}", base_key, namer::FluentKey::THIS_SUFFIX))
}

struct EmitGeneratedEnumInput<'a> {
    ident: &'a syn::Ident,
    origin_ident: &'a syn::Ident,
    key_name: Option<&'a String>,
    derives: &'a [syn::Path],
    variant_entries: &'a [GeneratedVariant],
    namespace_expr: TokenStream,
    this_key: Option<String>,
}

impl EmitGeneratedEnumInput<'_> {
    fn emit(self) -> TokenStream {
        let Self {
            ident,
            origin_ident,
            key_name,
            derives,
            variant_entries,
            namespace_expr,
            this_key,
        } = self;

        let match_arms = variant_entries.iter().map(|entry| {
            let variant_ident = &entry.ident;
            let ftl_key = &entry.ftl_key;
            quote! {
                Self::#variant_ident => write!(f, "{}", ::es_fluent::localize(#ftl_key, None))
            }
        });

        let cleaned_variants = variant_entries.iter().map(|entry| &entry.ident);
        let derive_attr = if !derives.is_empty() {
            quote! { #[derive(#(#derives),*)] }
        } else {
            quote! {}
        };

        let enum_doc = match key_name {
            Some(key) => format!("`{key}` variants of [`{origin_ident}`]."),
            None => format!("Variants of [`{origin_ident}`]."),
        };
        let variant_docs: Vec<_> = variant_entries
            .iter()
            .map(|entry| match key_name {
                Some(key) => format!(
                    "The `{}` `{key}` variant of [`{origin_ident}`].",
                    entry.doc_name
                ),
                None => format!("The `{}` variant of [`{origin_ident}`].", entry.doc_name),
            })
            .collect();

        let new_enum = quote! {
            #[doc = #enum_doc]
            #derive_attr
            pub enum #ident {
                #(#[doc = #variant_docs] #cleaned_variants),*
            }
        };

        let display_impl = {
            let trait_impl = quote! { ::es_fluent::FluentDisplay };
            let trait_fmt_fn_ident = quote! { fluent_fmt };

            quote! {
                impl #trait_impl for #ident {
                    fn #trait_fmt_fn_ident(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                        match self {
                            #(#match_arms),*
                        }
                    }
                }
            }
        };

        let static_variants: Vec<_> = variant_entries
            .iter()
            .map(|entry| {
                let variant_name = entry.ident.to_string();
                let ftl_key = &entry.ftl_key;
                quote! {
                    ::es_fluent::registry::FtlVariant {
                        name: #variant_name,
                        ftl_key: #ftl_key,
                        args: &[],
                        module_path: module_path!(),
                        line: line!(),
                    }
                }
            })
            .collect();

        let inventory_submit = generate_inventory_module(InventoryModuleInput {
            ident,
            module_name_prefix: "inventory",
            type_kind: quote! { ::es_fluent::meta::TypeKind::Enum },
            variants: static_variants,
            namespace_expr: namespace_expr.clone(),
        });

        let empty_generics = syn::Generics::default();
        let this_impl = generate_this_ftl_impl(ident, &empty_generics, this_key.as_deref());
        let this_inventory = if let Some(this_key) = this_key {
            let this_variant = quote! {
                ::es_fluent::registry::FtlVariant {
                    name: stringify!(#ident),
                    ftl_key: #this_key,
                    args: &[],
                    module_path: module_path!(),
                    line: line!(),
                }
            };

            generate_inventory_module(InventoryModuleInput {
                ident,
                module_name_prefix: "this_inventory",
                type_kind: quote! { ::es_fluent::meta::TypeKind::Enum },
                variants: vec![this_variant],
                namespace_expr: namespace_expr.clone(),
            })
        } else {
            quote! {}
        };
        let from_impls = generate_from_impls(ident, &empty_generics);

        quote! {
            #new_enum

            #display_impl

            #inventory_submit

            #this_impl
            #this_inventory

            #from_impls
        }
    }
}

fn build_struct_variant_entries(
    opts: &StructVariantsOpts,
    base_key: &namer::FluentKey,
) -> Vec<GeneratedVariant> {
    opts.fields()
        .iter()
        .map(|field_opt| {
            let field_ident = field_opt.ident().as_ref().unwrap();
            let original_field_name = field_ident.to_string();
            let pascal_case_name = original_field_name.to_pascal_case();
            let variant_ident = syn::Ident::new(&pascal_case_name, field_ident.span());
            let ftl_key = base_key.join(&original_field_name).to_string();
            GeneratedVariant {
                ident: variant_ident,
                doc_name: original_field_name,
                ftl_key,
            }
        })
        .collect()
}

fn generate_struct_unit_enum(
    opts: &StructVariantsOpts,
    ident: &syn::Ident,
    key_name: Option<&String>,
    this_opts: Option<&ThisOpts>,
    fluent_namespace: Option<&NamespaceValue>,
) -> TokenStream {
    let origin_ident = opts.ident();
    let base_key = namer::FluentKey::from(ident);
    let variant_entries = build_struct_variant_entries(opts, &base_key);
    let derives: Vec<syn::Path> = (*opts.attr_args().derive()).to_vec();
    let namespace_expr =
        resolve_variants_namespace(opts.attr_args().namespace(), this_opts, fluent_namespace);

    EmitGeneratedEnumInput {
        ident,
        origin_ident,
        key_name,
        derives: &derives,
        variant_entries: &variant_entries,
        namespace_expr,
        this_key: variants_this_key(this_opts, &base_key),
    }
    .emit()
}

pub fn process_enum(
    opts: &EnumVariantsOpts,
    this_opts: Option<&ThisOpts>,
    fluent_namespace: Option<&NamespaceValue>,
) -> TokenStream {
    let keys = match opts.keyed_idents() {
        Ok(keys) => keys,
        Err(err) => err.abort(),
    };
    let key_strings = opts.attr_args().key_strings().unwrap_or_default();

    if opts.variants().is_empty() {
        return quote! {};
    }

    let ftl_enum_ident = opts.ftl_enum_ident();
    emit_generated_enums(&ftl_enum_ident, &keys, &key_strings, |ident, key_name| {
        generate_enum_unit_enum(opts, ident, key_name, this_opts, fluent_namespace)
    })
}

fn build_enum_variant_entries(
    opts: &EnumVariantsOpts,
    base_key: &namer::FluentKey,
) -> Vec<GeneratedVariant> {
    opts.variants()
        .iter()
        .map(|variant_opt| {
            let variant_ident = variant_opt.ident();
            let variant_key = variant_ident.to_string();
            let ftl_key = base_key.join(&variant_key).to_string();
            GeneratedVariant {
                ident: variant_ident.clone(),
                doc_name: variant_key,
                ftl_key,
            }
        })
        .collect()
}

fn generate_enum_unit_enum(
    opts: &EnumVariantsOpts,
    ident: &syn::Ident,
    key_name: Option<&String>,
    this_opts: Option<&ThisOpts>,
    fluent_namespace: Option<&NamespaceValue>,
) -> TokenStream {
    let origin_ident = opts.ident();
    let base_key = namer::FluentKey::from(ident);
    let variant_entries = build_enum_variant_entries(opts, &base_key);
    let derives: Vec<syn::Path> = (*opts.attr_args().derive()).to_vec();
    let namespace_expr =
        resolve_variants_namespace(opts.attr_args().namespace(), this_opts, fluent_namespace);

    EmitGeneratedEnumInput {
        ident,
        origin_ident,
        key_name,
        derives: &derives,
        variant_entries: &variant_entries,
        namespace_expr,
        this_key: variants_this_key(this_opts, &base_key),
    }
    .emit()
}

#[cfg(test)]
mod tests {
    use super::{fluent_namespace, process_enum, process_struct};
    use darling::FromDeriveInput as _;
    use es_fluent_derive_core::options::{
        r#enum::EnumVariantsOpts, r#struct::StructVariantsOpts, this::ThisOpts,
    };
    use syn::parse_quote;

    #[test]
    fn process_struct_emits_keyed_generated_enums() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "ui")]
            #[fluent_variants(keys = ["label", "placeholder"], derive(Debug))]
            struct LoginForm {
                username: String,
                password: String,
            }
        };

        let opts = StructVariantsOpts::from_derive_input(&input).expect("StructVariantsOpts");
        let this_opts = ThisOpts::from_derive_input(&input).ok();
        let fluent_namespace = fluent_namespace(&input).expect("parent namespace");

        let tokens =
            process_struct(&opts, this_opts.as_ref(), fluent_namespace.as_ref()).to_string();
        assert!(tokens.contains("pub enum LoginFormLabelVariants"));
        assert!(tokens.contains("pub enum LoginFormPlaceholderVariants"));
        assert!(tokens.contains("__es_fluent_inventory_login_form_label_variants"));
        assert!(
            tokens
                .contains("impl From < & LoginFormLabelVariants > for :: es_fluent :: FluentValue")
        );
    }

    #[test]
    fn process_enum_emits_variants_this_registration() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "ui")]
            #[fluent_this(variants)]
            enum Status {
                Ready,
                Failed,
            }
        };

        let opts = EnumVariantsOpts::from_derive_input(&input).expect("EnumVariantsOpts");
        let this_opts = ThisOpts::from_derive_input(&input).ok();
        let fluent_namespace = fluent_namespace(&input).expect("parent namespace");

        let tokens = process_enum(&opts, this_opts.as_ref(), fluent_namespace.as_ref()).to_string();
        assert!(tokens.contains("pub enum StatusVariants"));
        assert!(tokens.contains("impl :: es_fluent :: ThisFtl for StatusVariants"));
        assert!(tokens.contains("status_variants_this"));
        assert!(tokens.contains("__es_fluent_this_inventory_status_variants"));
    }
}

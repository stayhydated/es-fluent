use darling::FromDeriveInput as _;
use es_fluent_derive_core::namer;
use es_fluent_derive_core::options::r#enum::{EnumOpts, EnumVariantsOpts};
use es_fluent_derive_core::options::namespace::NamespaceValue;
use es_fluent_derive_core::options::r#struct::{StructOpts, StructVariantsOpts};
use es_fluent_derive_core::options::this::ThisOpts;

use heck::{ToPascalCase as _, ToSnakeCase as _};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

use crate::macros::utils::namespace_rule_tokens;

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let this_opts = ThisOpts::from_derive_input(&input).ok();
    let fluent_namespace = match &input.data {
        Data::Struct(_) => match StructOpts::from_derive_input(&input) {
            Ok(opts) => opts.attr_args().namespace().cloned(),
            Err(err) => return err.write_errors().into(),
        },
        Data::Enum(_) => match EnumOpts::from_derive_input(&input) {
            Ok(opts) => opts.attr_args().namespace().cloned(),
            Err(err) => return err.write_errors().into(),
        },
        Data::Union(_) => panic!("EsFluentVariants cannot be used on unions"),
    };

    let tokens = match &input.data {
        Data::Struct(data) => {
            let opts = match StructVariantsOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors().into(),
            };

            process_struct(&opts, data, this_opts.as_ref(), fluent_namespace.as_ref())
        },
        Data::Enum(_data) => {
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

pub fn process_struct(
    opts: &StructVariantsOpts,
    data: &syn::DataStruct,
    this_opts: Option<&ThisOpts>,
    fluent_namespace: Option<&NamespaceValue>,
) -> TokenStream {
    let keys = match opts.keyyed_idents() {
        Ok(keys) => keys,
        Err(err) => err.abort(),
    };
    let key_strings = opts.attr_args().key_strings().unwrap_or_default();

    // For empty structs, don't generate any enums
    let is_empty = opts.fields().is_empty();
    if is_empty {
        return quote! {};
    }

    if keys.is_empty() {
        let ftl_enum_ident = opts.ftl_enum_ident();
        let ftl_enum = generate_unit_enum(
            opts,
            data,
            &ftl_enum_ident,
            None,
            this_opts,
            fluent_namespace,
        );
        quote! {
            #ftl_enum
        }
    } else {
        let enums = keys.iter().zip(key_strings.iter()).map(|(key, key_str)| {
            generate_unit_enum(opts, data, key, Some(key_str), this_opts, fluent_namespace)
        });

        quote! {
            #(#enums)*
        }
    }
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
            .or_else(|| this_opts.and_then(|o| o.attr_args().namespace())),
    )
}

fn emit_generated_enum(
    ident: &syn::Ident,
    origin_ident: &syn::Ident,
    key_name: Option<&String>,
    derives: &[syn::Path],
    variant_entries: &[GeneratedVariant],
    namespace_expr: TokenStream,
    variants_this: bool,
    base_key: &namer::FluentKey,
) -> TokenStream {
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

    let type_name = ident.to_string();
    let mod_name = quote::format_ident!("__es_fluent_inventory_{}", type_name.to_snake_case());
    let namespace_expr_for_inventory = namespace_expr.clone();
    let inventory_submit = quote! {
        #[doc(hidden)]
        mod #mod_name {
            use super::*;

            static VARIANTS: &[::es_fluent::registry::FtlVariant] = &[
                #(#static_variants),*
            ];

            static TYPE_INFO: ::es_fluent::registry::FtlTypeInfo =
                ::es_fluent::registry::FtlTypeInfo {
                    type_kind: ::es_fluent::meta::TypeKind::Enum,
                    type_name: #type_name,
                    variants: VARIANTS,
                    file_path: file!(),
                    module_path: module_path!(),
                    namespace: #namespace_expr_for_inventory,
                };

            ::es_fluent::__inventory::submit!(::es_fluent::registry::RegisteredFtlType(&TYPE_INFO));
        }
    };

    let this_impl = if variants_this {
        let this_key = format!("{}{}", base_key, namer::FluentKey::THIS_SUFFIX);
        quote! {
            impl ::es_fluent::ThisFtl for #ident {
                fn this_ftl() -> String {
                    ::es_fluent::localize(#this_key, None)
                }
            }
        }
    } else {
        quote! {}
    };

    let this_inventory = if variants_this {
        let this_key = format!("{}{}", base_key, namer::FluentKey::THIS_SUFFIX);
        let this_mod_name =
            quote::format_ident!("__es_fluent_this_inventory_{}", type_name.to_snake_case());
        let namespace_expr_for_this = namespace_expr.clone();
        quote! {
            #[doc(hidden)]
            mod #this_mod_name {
                use super::*;

                static VARIANTS: &[::es_fluent::registry::FtlVariant] = &[
                    ::es_fluent::registry::FtlVariant {
                        name: #type_name,
                        ftl_key: #this_key,
                        args: &[],
                        module_path: module_path!(),
                        line: line!(),
                    }
                ];

                static TYPE_INFO: ::es_fluent::registry::FtlTypeInfo =
                    ::es_fluent::registry::FtlTypeInfo {
                        type_kind: ::es_fluent::meta::TypeKind::Enum,
                        type_name: #type_name,
                        variants: VARIANTS,
                        file_path: file!(),
                        module_path: module_path!(),
                        namespace: #namespace_expr_for_this,
                    };

                ::es_fluent::__inventory::submit!(::es_fluent::registry::RegisteredFtlType(&TYPE_INFO));
            }
        }
    } else {
        quote! {}
    };

    quote! {
      #new_enum

      #display_impl

      #inventory_submit

      #this_impl
      #this_inventory

      impl From<& #ident> for ::es_fluent::FluentValue<'_> {
            fn from(value: & #ident) -> Self {
              use ::es_fluent::ToFluentString as _;
              value.to_fluent_string().into()
            }
      }

      impl From<#ident> for ::es_fluent::FluentValue<'_> {
            fn from(value: #ident) -> Self {
                (&value).into()
            }
      }
    }
}

fn generate_unit_enum(
    opts: &StructVariantsOpts,
    _data: &syn::DataStruct,
    ident: &syn::Ident,
    key_name: Option<&String>,
    this_opts: Option<&ThisOpts>,
    fluent_namespace: Option<&NamespaceValue>,
) -> TokenStream {
    let field_opts = opts.fields();
    let origin_ident = opts.ident();
    let base_key = namer::FluentKey::from(ident);

    let variant_entries: Vec<GeneratedVariant> = field_opts
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
        .collect();
    let derives: Vec<syn::Path> = (*opts.attr_args().derive()).to_vec();
    let variants_this = this_opts.is_some_and(|opts| opts.attr_args().is_variants());
    let namespace_expr =
        resolve_variants_namespace(opts.attr_args().namespace(), this_opts, fluent_namespace);

    emit_generated_enum(
        ident,
        origin_ident,
        key_name,
        &derives,
        &variant_entries,
        namespace_expr,
        variants_this,
        &base_key,
    )
}

pub fn process_enum(
    opts: &EnumVariantsOpts,
    this_opts: Option<&ThisOpts>,
    fluent_namespace: Option<&NamespaceValue>,
) -> TokenStream {
    let keys = match opts.keyyed_idents() {
        Ok(keys) => keys,
        Err(err) => err.abort(),
    };
    let key_strings = opts.attr_args().key_strings().unwrap_or_default();

    // For empty enums, don't generate any new enums
    let is_empty = opts.variants().is_empty();
    if is_empty {
        return quote! {};
    }

    if keys.is_empty() {
        let ftl_enum_ident = opts.ftl_enum_ident();
        let ftl_enum =
            generate_enum_unit_enum(opts, &ftl_enum_ident, None, this_opts, fluent_namespace);
        quote! {
            #ftl_enum
        }
    } else {
        let enums = keys.iter().zip(key_strings.iter()).map(|(key, key_str)| {
            generate_enum_unit_enum(opts, key, Some(key_str), this_opts, fluent_namespace)
        });

        quote! {
            #(#enums)*
        }
    }
}

fn generate_enum_unit_enum(
    opts: &EnumVariantsOpts,
    ident: &syn::Ident,
    key_name: Option<&String>,
    this_opts: Option<&ThisOpts>,
    fluent_namespace: Option<&NamespaceValue>,
) -> TokenStream {
    let variant_opts = opts.variants();
    let origin_ident = opts.ident();
    let base_key = namer::FluentKey::from(ident);
    let variant_entries: Vec<GeneratedVariant> = variant_opts
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
        .collect();
    let derives: Vec<syn::Path> = (*opts.attr_args().derive()).to_vec();
    let variants_this = this_opts.is_some_and(|opts| opts.attr_args().is_variants());
    let namespace_expr =
        resolve_variants_namespace(opts.attr_args().namespace(), this_opts, fluent_namespace);

    emit_generated_enum(
        ident,
        origin_ident,
        key_name,
        &derives,
        &variant_entries,
        namespace_expr,
        variants_this,
        &base_key,
    )
}

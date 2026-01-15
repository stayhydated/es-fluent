use darling::FromDeriveInput as _;
use es_fluent_derive_core::namer;
use es_fluent_derive_core::options::r#enum::EnumKvOpts;
use es_fluent_derive_core::options::r#struct::StructKvOpts;
use es_fluent_derive_core::options::this::ThisOpts;

use heck::{ToPascalCase as _, ToSnakeCase as _};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let this_opts = ThisOpts::from_derive_input(&input).ok();

    let tokens = match &input.data {
        Data::Struct(data) => {
            let opts = match StructKvOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors().into(),
            };

            process_struct(&opts, data, this_opts.as_ref())
        },
        Data::Enum(_data) => {
            let opts = match EnumKvOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors().into(),
            };

            process_enum(&opts, this_opts.as_ref())
        },
        Data::Union(_) => panic!("EsFluentKv cannot be used on unions"),
    };

    tokens.into()
}

pub fn process_struct(
    opts: &StructKvOpts,
    data: &syn::DataStruct,
    this_opts: Option<&ThisOpts>,
) -> TokenStream {
    let keys = match opts.keyyed_idents() {
        Ok(keys) => keys,
        Err(err) => err.abort(),
    };

    // For empty structs, don't generate any enums
    let is_empty = opts.fields().is_empty();
    if is_empty {
        return quote! {};
    }

    if keys.is_empty() {
        let ftl_enum_ident = opts.ftl_enum_ident();
        let ftl_enum = generate_unit_enum(opts, data, &ftl_enum_ident, this_opts);
        quote! {
            #ftl_enum
        }
    } else {
        let enums = keys
            .iter()
            .map(|key| generate_unit_enum(opts, data, key, this_opts));

        quote! {
            #(#enums)*
        }
    }
}

fn generate_unit_enum(
    opts: &StructKvOpts,
    _data: &syn::DataStruct,
    ident: &syn::Ident,
    this_opts: Option<&ThisOpts>,
) -> TokenStream {
    let field_opts = opts.fields();

    // For structs, we store (variant_ident, original_field_name, field_opt)
    // variant_ident is PascalCase for the enum, original_field_name is snake_case for FTL key
    let variants: Vec<_> = field_opts
        .iter()
        .map(|field_opt| {
            let field_ident = field_opt.ident().as_ref().unwrap();
            let original_field_name = field_ident.to_string(); // Keep original snake_case
            let pascal_case_name = original_field_name.to_pascal_case();
            let variant_ident = syn::Ident::new(&pascal_case_name, field_ident.span());
            (variant_ident, original_field_name, field_opt)
        })
        .collect();

    let match_arms = variants
        .iter()
        .map(|(variant_ident, original_field_name, _)| {
            // Use original field name (snake_case) for FTL key
            let ftl_key = namer::FluentKey::from(ident)
                .join(original_field_name)
                .to_string();
            quote! {
                Self::#variant_ident => write!(f, "{}", ::es_fluent::localize(#ftl_key, None))
            }
        });

    let cleaned_variants = variants.iter().map(|(ident, _, _)| ident);
    let mut derives: Vec<syn::Path> = (*opts.attr_args().derive()).to_vec();

    // If fields_this is true, add EsFluentThis to derives
    if let Some(this_opts) = this_opts
        && this_opts.attr_args().is_members()
    {
        derives.push(syn::parse_quote!(::es_fluent::EsFluentThis));
    }

    let derive_attr = if !derives.is_empty() {
        quote! { #[derive(#(#derives),*)] }
    } else {
        quote! {}
    };

    let new_enum = quote! {
      #derive_attr
      pub enum #ident {
          #(#cleaned_variants),*
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

    // Generate inventory submission for the new enum
    let static_variants: Vec<_> = variants
        .iter()
        .map(|(variant_ident, original_field_name, _)| {
            let variant_name = variant_ident.to_string();
            // Use original field name (snake_case) for FTL key
            let ftl_key = namer::FluentKey::from(ident)
                .join(original_field_name)
                .to_string();
            quote! {
                ::es_fluent::registry::StaticFtlVariant {
                    name: #variant_name,
                    ftl_key: #ftl_key,
                    args: &[],
                    module_path: module_path!(),
                }
            }
        })
        .collect();

    let type_name = ident.to_string();
    let mod_name = quote::format_ident!("__es_fluent_inventory_{}", type_name.to_snake_case());

    let inventory_submit = quote! {
        #[doc(hidden)]
        mod #mod_name {
            use super::*;

            static VARIANTS: &[::es_fluent::registry::StaticFtlVariant] = &[
                #(#static_variants),*
            ];

            static TYPE_INFO: ::es_fluent::registry::StaticFtlTypeInfo =
                ::es_fluent::registry::StaticFtlTypeInfo {
                    type_kind: ::es_fluent::meta::TypeKind::Enum,
                    type_name: #type_name,
                    variants: VARIANTS,
                    file_path: file!(),
                    module_path: module_path!(),
                };

            ::es_fluent::__inventory::submit!(&TYPE_INFO);
        }
    };

    quote! {
      #new_enum

      #display_impl

      #inventory_submit

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

pub fn process_enum(opts: &EnumKvOpts, this_opts: Option<&ThisOpts>) -> TokenStream {
    let keys = match opts.keyyed_idents() {
        Ok(keys) => keys,
        Err(err) => err.abort(),
    };

    // For empty enums, don't generate any new enums
    let is_empty = opts.variants().is_empty();
    if is_empty {
        return quote! {};
    }

    if keys.is_empty() {
        let ftl_enum_ident = opts.ftl_enum_ident();
        let ftl_enum = generate_enum_unit_enum(opts, &ftl_enum_ident, this_opts);
        quote! {
            #ftl_enum
        }
    } else {
        let enums = keys
            .iter()
            .map(|key| generate_enum_unit_enum(opts, key, this_opts));

        quote! {
            #(#enums)*
        }
    }
}

fn generate_enum_unit_enum(
    opts: &EnumKvOpts,
    ident: &syn::Ident,
    this_opts: Option<&ThisOpts>,
) -> TokenStream {
    let variant_opts = opts.variants();

    let variants: Vec<_> = variant_opts
        .iter()
        .map(|variant_opt| {
            let variant_ident = variant_opt.ident();
            // Keep original variant name (PascalCase for enums)
            (variant_ident.clone(), variant_opt)
        })
        .collect();

    let match_arms = variants.iter().map(|(variant_ident, _)| {
        // Use original variant name for the key (preserves PascalCase)
        let base_key = variant_ident.to_string();
        let ftl_key = namer::FluentKey::from(ident).join(&base_key).to_string();
        quote! {
            Self::#variant_ident => write!(f, "{}", ::es_fluent::localize(#ftl_key, None))
        }
    });

    let cleaned_variants = variants.iter().map(|(ident, _)| ident);
    let mut derives: Vec<syn::Path> = (*opts.attr_args().derive()).to_vec();

    // If variants_this is true, add EsFluentThis
    if let Some(this_opts) = this_opts
        && this_opts.attr_args().is_members()
    {
        derives.push(syn::parse_quote!(::es_fluent::EsFluentThis));
    }

    let derive_attr = if !derives.is_empty() {
        quote! { #[derive(#(#derives),*)] }
    } else {
        quote! {}
    };

    let new_enum = quote! {
      #derive_attr
      pub enum #ident {
          #(#cleaned_variants),*
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

    // Generate inventory submission for the new enum
    let static_variants: Vec<_> = variants
        .iter()
        .map(|(variant_ident, _)| {
            let variant_name = variant_ident.to_string();
            // Use original variant name for the key (preserves PascalCase for enums)
            let base_key = variant_ident.to_string();
            let ftl_key = namer::FluentKey::from(ident).join(&base_key).to_string();
            quote! {
                ::es_fluent::registry::StaticFtlVariant {
                    name: #variant_name,
                    ftl_key: #ftl_key,
                    args: &[],
                    module_path: module_path!(),
                }
            }
        })
        .collect();

    let type_name = ident.to_string();
    let mod_name = quote::format_ident!("__es_fluent_inventory_{}", type_name.to_snake_case());

    let inventory_submit = quote! {
        #[doc(hidden)]
        mod #mod_name {
            use super::*;

            static VARIANTS: &[::es_fluent::registry::StaticFtlVariant] = &[
                #(#static_variants),*
            ];

            static TYPE_INFO: ::es_fluent::registry::StaticFtlTypeInfo =
                ::es_fluent::registry::StaticFtlTypeInfo {
                    type_kind: ::es_fluent::meta::TypeKind::Enum,
                    type_name: #type_name,
                    variants: VARIANTS,
                    file_path: file!(),
                    module_path: module_path!(),
                };

            ::es_fluent::__inventory::submit!(&TYPE_INFO);
        }
    };

    quote! {
      #new_enum

      #display_impl

      #inventory_submit

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

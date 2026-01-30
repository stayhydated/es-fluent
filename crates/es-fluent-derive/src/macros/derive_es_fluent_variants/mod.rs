use darling::FromDeriveInput as _;
use es_fluent_derive_core::namer;
use es_fluent_derive_core::options::r#enum::EnumKvOpts;
use es_fluent_derive_core::options::r#struct::StructVariantsOpts;
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
            let opts = match StructVariantsOpts::from_derive_input(&input) {
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
        Data::Union(_) => panic!("EsFluentVariants cannot be used on unions"),
    };

    tokens.into()
}

pub fn process_struct(
    opts: &StructVariantsOpts,
    data: &syn::DataStruct,
    this_opts: Option<&ThisOpts>,
) -> TokenStream {
    let keys = match opts.keyyed_idents() {
        Ok(keys) => keys,
        Err(err) => err.abort(),
    };
    let base_idents = match opts.keyed_base_idents() {
        Ok(base_idents) => base_idents,
        Err(err) => err.abort(),
    };

    // For empty structs, don't generate any enums
    let is_empty = opts.fields().is_empty();
    if is_empty {
        return quote! {};
    }

    if keys.is_empty() {
        let ftl_enum_ident = opts.ftl_enum_ident();
        let ftl_enum = generate_unit_enum(opts, data, &ftl_enum_ident, opts.ident(), this_opts);
        quote! {
            #ftl_enum
        }
    } else {
        let enums = keys
            .iter()
            .zip(base_idents.iter())
            .map(|(key, base_ident)| generate_unit_enum(opts, data, key, base_ident, this_opts));

        quote! {
            #(#enums)*
        }
    }
}

fn generate_unit_enum(
    opts: &StructVariantsOpts,
    _data: &syn::DataStruct,
    ident: &syn::Ident,
    base_ident: &syn::Ident,
    this_opts: Option<&ThisOpts>,
) -> TokenStream {
    let field_opts = opts.fields();
    let base_key = namer::FluentKey::from(base_ident);

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
            let ftl_key = base_key.join(original_field_name).to_string();
            quote! {
                Self::#variant_ident => write!(f, "{}", ::es_fluent::localize(#ftl_key, None))
            }
        });

    let cleaned_variants = variants.iter().map(|(ident, _, _)| ident);
    let derives: Vec<syn::Path> = (*opts.attr_args().derive()).to_vec();

    let members_this = this_opts.is_some_and(|opts| opts.attr_args().is_members());

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
            let ftl_key = base_key.join(original_field_name).to_string();
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

    // Generate namespace expression - prefer fluent_variants namespace, fall back to fluent_this
    let namespace_expr = opts
        .attr_args()
        .namespace()
        .map(|ns| match ns {
            es_fluent_derive_core::options::namespace::NamespaceValue::Literal(s) => {
                quote! { Some(#s) }
            },
            es_fluent_derive_core::options::namespace::NamespaceValue::File => {
                quote! {
                    Some({
                        const FILE_PATH: &str = file!();
                        const NAMESPACE: &str = ::es_fluent::__namespace_from_file_path(FILE_PATH);
                        NAMESPACE
                    })
                }
            },
            es_fluent_derive_core::options::namespace::NamespaceValue::FileRelative => {
                quote! {
                    Some({
                        const FILE_PATH: &str = file!();
                        const NAMESPACE: &str = ::es_fluent::__namespace_from_file_path_relative(FILE_PATH);
                        NAMESPACE
                    })
                }
            },
        })
        .or_else(|| {
            this_opts.and_then(|o| match o.attr_args().namespace() {
                Some(es_fluent_derive_core::options::namespace::NamespaceValue::Literal(s)) => {
                    Some(quote! { Some(#s) })
                },
                Some(es_fluent_derive_core::options::namespace::NamespaceValue::File) => {
                    Some(quote! {
                        Some({
                            const FILE_PATH: &str = file!();
                            const NAMESPACE: &str = ::es_fluent::__namespace_from_file_path(FILE_PATH);
                            NAMESPACE
                        })
                    })
                },
                Some(es_fluent_derive_core::options::namespace::NamespaceValue::FileRelative) => {
                    Some(quote! {
                        Some({
                            const FILE_PATH: &str = file!();
                            const NAMESPACE: &str = ::es_fluent::__namespace_from_file_path_relative(FILE_PATH);
                            NAMESPACE
                        })
                    })
                },
                None => None,
            })
        })
        .unwrap_or_else(|| quote! { None });

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
                    namespace: #namespace_expr,
                };

            ::es_fluent::__inventory::submit!(::es_fluent::registry::RegisteredFtlType(&TYPE_INFO));
        }
    };

    let this_impl = if members_this {
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

    let this_inventory = if members_this {
        let this_key = format!("{}{}", base_key, namer::FluentKey::THIS_SUFFIX);
        let this_mod_name =
            quote::format_ident!("__es_fluent_this_inventory_{}", type_name.to_snake_case());
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
                        namespace: #namespace_expr,
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

pub fn process_enum(opts: &EnumKvOpts, this_opts: Option<&ThisOpts>) -> TokenStream {
    let keys = match opts.keyyed_idents() {
        Ok(keys) => keys,
        Err(err) => err.abort(),
    };
    let base_idents = match opts.keyed_base_idents() {
        Ok(base_idents) => base_idents,
        Err(err) => err.abort(),
    };

    // For empty enums, don't generate any new enums
    let is_empty = opts.variants().is_empty();
    if is_empty {
        return quote! {};
    }

    if keys.is_empty() {
        let ftl_enum_ident = opts.ftl_enum_ident();
        let ftl_enum = generate_enum_unit_enum(opts, &ftl_enum_ident, opts.ident(), this_opts);
        quote! {
            #ftl_enum
        }
    } else {
        let enums = keys
            .iter()
            .zip(base_idents.iter())
            .map(|(key, base_ident)| generate_enum_unit_enum(opts, key, base_ident, this_opts));

        quote! {
            #(#enums)*
        }
    }
}

fn generate_enum_unit_enum(
    opts: &EnumKvOpts,
    ident: &syn::Ident,
    base_ident: &syn::Ident,
    this_opts: Option<&ThisOpts>,
) -> TokenStream {
    let variant_opts = opts.variants();
    let base_key = namer::FluentKey::from(base_ident);

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
        let variant_key = variant_ident.to_string();
        let ftl_key = base_key.join(&variant_key).to_string();
        quote! {
            Self::#variant_ident => write!(f, "{}", ::es_fluent::localize(#ftl_key, None))
        }
    });

    let cleaned_variants = variants.iter().map(|(ident, _)| ident);
    let derives: Vec<syn::Path> = (*opts.attr_args().derive()).to_vec();

    let members_this = this_opts.is_some_and(|opts| opts.attr_args().is_members());

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
            let variant_key = variant_ident.to_string();
            let ftl_key = base_key.join(&variant_key).to_string();
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

    // Generate namespace expression - prefer fluent_variants namespace, fall back to fluent_this
    let namespace_expr = opts
        .attr_args()
        .namespace()
        .map(|ns| match ns {
            es_fluent_derive_core::options::namespace::NamespaceValue::Literal(s) => {
                quote! { Some(#s) }
            },
            es_fluent_derive_core::options::namespace::NamespaceValue::File => {
                quote! {
                    Some({
                        const FILE_PATH: &str = file!();
                        const NAMESPACE: &str = ::es_fluent::__namespace_from_file_path(FILE_PATH);
                        NAMESPACE
                    })
                }
            },
            es_fluent_derive_core::options::namespace::NamespaceValue::FileRelative => {
                quote! {
                    Some({
                        const FILE_PATH: &str = file!();
                        const NAMESPACE: &str = ::es_fluent::__namespace_from_file_path_relative(FILE_PATH);
                        NAMESPACE
                    })
                }
            },
        })
        .or_else(|| {
            this_opts.and_then(|o| match o.attr_args().namespace() {
                Some(es_fluent_derive_core::options::namespace::NamespaceValue::Literal(s)) => {
                    Some(quote! { Some(#s) })
                },
                Some(es_fluent_derive_core::options::namespace::NamespaceValue::File) => {
                    Some(quote! {
                        Some({
                            const FILE_PATH: &str = file!();
                            const NAMESPACE: &str = ::es_fluent::__namespace_from_file_path(FILE_PATH);
                            NAMESPACE
                        })
                    })
                },
                Some(es_fluent_derive_core::options::namespace::NamespaceValue::FileRelative) => {
                    Some(quote! {
                        Some({
                            const FILE_PATH: &str = file!();
                            const NAMESPACE: &str = ::es_fluent::__namespace_from_file_path_relative(FILE_PATH);
                            NAMESPACE
                        })
                    })
                },
                None => None,
            })
        })
        .unwrap_or_else(|| quote! { None });

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
                    namespace: #namespace_expr,
                };

            ::es_fluent::__inventory::submit!(::es_fluent::registry::RegisteredFtlType(&TYPE_INFO));
        }
    };

    let this_impl = if members_this {
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

    let this_inventory = if members_this {
        let this_key = format!("{}{}", base_key, namer::FluentKey::THIS_SUFFIX);
        let this_mod_name =
            quote::format_ident!("__es_fluent_this_inventory_{}", type_name.to_snake_case());
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
                        namespace: #namespace_expr,
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

use darling::FromDeriveInput as _;
use es_fluent_core::namer;
use es_fluent_core::options::r#enum::EnumKvOpts;
use es_fluent_core::options::r#struct::StructKvOpts;
use es_fluent_core::validation;

use heck::{ToPascalCase as _, ToSnakeCase as _};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let tokens = match &input.data {
        Data::Struct(data) => {
            let opts = match StructKvOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors().into(),
            };

            if let Err(err) = validation::validate_struct_kv(&opts, data) {
                err.abort();
            }

            process_struct(&opts, data)
        },
        Data::Enum(data) => {
            let opts = match EnumKvOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors().into(),
            };

            if let Err(err) = validation::validate_enum_kv(&opts, data) {
                err.abort();
            }

            process_enum(&opts)
        },
        Data::Union(_) => panic!("EsFluentKv cannot be used on unions"),
    };

    tokens.into()
}

pub fn process_struct(opts: &StructKvOpts, data: &syn::DataStruct) -> TokenStream {
    let keys = match opts.keyyed_idents() {
        Ok(keys) => keys,
        Err(err) => err.abort(),
    };

    let this_ftl_struct_impl = if opts.attr_args().is_this() {
        let original_ident = opts.ident();
        let (impl_generics, ty_generics, where_clause) = opts.generics().split_for_impl();
        let this_ident = quote::format_ident!("{}_this", original_ident);
        let this_ftl_key = namer::FluentKey::new(&this_ident, "").to_string();
        quote! {
            impl #impl_generics ::es_fluent::ThisFtl for #original_ident #ty_generics #where_clause {
                fn this_ftl() -> String {
                    ::es_fluent::localize(#this_ftl_key, None)
                }
            }
        }
    } else {
        quote! {}
    };

    // For empty structs, don't generate any enums - only the this_ftl impl
    let is_empty = opts.fields().is_empty();
    if is_empty {
        return quote! {
            #this_ftl_struct_impl
        };
    }

    if keys.is_empty() {
        let ftl_enum_ident = opts.ftl_enum_ident();
        let ftl_enum = generate_unit_enum(opts, data, &ftl_enum_ident);
        quote! {
            #ftl_enum

            #this_ftl_struct_impl
        }
    } else {
        let enums = keys.iter().map(|key| generate_unit_enum(opts, data, key));

        quote! {
            #(#enums)*

            #this_ftl_struct_impl
        }
    }
}

fn generate_unit_enum(
    opts: &StructKvOpts,
    _data: &syn::DataStruct,
    ident: &syn::Ident,
) -> TokenStream {
    let field_opts = opts.fields();

    let variants: Vec<_> = field_opts
        .iter()
        .map(|field_opt| {
            let ident = field_opt.ident().as_ref().unwrap();
            let pascal_case_name = ident.to_string().to_pascal_case();
            let variant_ident = syn::Ident::new(&pascal_case_name, ident.span());
            (variant_ident, field_opt)
        })
        .collect();

    let match_arms = variants.iter().map(|(variant_ident, _)| {
        let base_key = variant_ident.to_string().to_snake_case();
        let ftl_key = namer::FluentKey::new(ident, &base_key).to_string();
        quote! {
            Self::#variant_ident => write!(f, "{}", ::es_fluent::localize(#ftl_key, None))
        }
    });

    let cleaned_variants = variants.iter().map(|(ident, _)| ident);
    let derives = opts.attr_args().derive();
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

    // `keys_this` generates ThisFtl on the generated KV enums
    let this_ftl_impl = if opts.attr_args().is_keys_this() {
        let this_ident = quote::format_ident!("{}_this", ident);
        let this_ftl_key = namer::FluentKey::new(&this_ident, "").to_string();
        quote! {
            impl ::es_fluent::ThisFtl for #ident {
                fn this_ftl() -> String {
                    ::es_fluent::localize(#this_ftl_key, None)
                }
            }
        }
    } else {
        quote! {}
    };

    quote! {
      #new_enum

      #display_impl

      #this_ftl_impl

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

pub fn process_enum(opts: &EnumKvOpts) -> TokenStream {
    let keys = match opts.keyyed_idents() {
        Ok(keys) => keys,
        Err(err) => err.abort(),
    };

    let original_ident = opts.ident();
    let (impl_generics, ty_generics, where_clause) = opts.generics().split_for_impl();

    // `this` generates ThisFtl on the original type (e.g., Country)
    // `keys_this` generates ThisFtl on the generated KV enums (e.g., CountryLabelKvFtl)
    let this_ftl_enum_impl = if opts.attr_args().is_this() {
        let this_ident = quote::format_ident!("{}_this", original_ident);
        let this_ftl_key = namer::FluentKey::new(&this_ident, "").to_string();
        quote! {
            impl #impl_generics ::es_fluent::ThisFtl for #original_ident #ty_generics #where_clause {
                fn this_ftl() -> String {
                    ::es_fluent::localize(#this_ftl_key, None)
                }
            }
        }
    } else {
        quote! {}
    };

    // For empty enums, don't generate any new enums - only the this_ftl impl
    let is_empty = opts.variants().is_empty();
    if is_empty {
        return quote! {
            #this_ftl_enum_impl
        };
    }

    if keys.is_empty() {
        let ftl_enum_ident = opts.ftl_enum_ident();
        let ftl_enum = generate_enum_unit_enum(opts, &ftl_enum_ident);
        quote! {
            #ftl_enum

            #this_ftl_enum_impl
        }
    } else {
        let enums = keys.iter().map(|key| generate_enum_unit_enum(opts, key));

        quote! {
            #(#enums)*

            #this_ftl_enum_impl
        }
    }
}

fn generate_enum_unit_enum(opts: &EnumKvOpts, ident: &syn::Ident) -> TokenStream {
    let variant_opts = opts.variants();

    let variants: Vec<_> = variant_opts
        .iter()
        .map(|variant_opt| {
            let variant_ident = variant_opt.ident();
            let pascal_case_name = variant_ident.to_string().to_pascal_case();
            let new_variant_ident = syn::Ident::new(&pascal_case_name, variant_ident.span());
            (new_variant_ident, variant_opt)
        })
        .collect();

    let match_arms = variants.iter().map(|(variant_ident, _)| {
        let base_key = variant_ident.to_string().to_snake_case();
        let ftl_key = namer::FluentKey::new(ident, &base_key).to_string();
        quote! {
            Self::#variant_ident => write!(f, "{}", ::es_fluent::localize(#ftl_key, None))
        }
    });

    let cleaned_variants = variants.iter().map(|(ident, _)| ident);
    let derives = opts.attr_args().derive();
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

    // `keys_this` generates ThisFtl on the generated KV enums
    let this_ftl_impl = if opts.attr_args().is_keys_this() {
        let this_ident = quote::format_ident!("{}_this", ident);
        let this_ftl_key = namer::FluentKey::new(&this_ident, "").to_string();
        quote! {
            impl ::es_fluent::ThisFtl for #ident {
                fn this_ftl() -> String {
                    ::es_fluent::localize(#this_ftl_key, None)
                }
            }
        }
    } else {
        quote! {}
    };

    quote! {
      #new_enum

      #display_impl

      #this_ftl_impl

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

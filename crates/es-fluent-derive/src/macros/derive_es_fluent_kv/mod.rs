use darling::FromDeriveInput as _;
use es_fluent_core::namer;
use es_fluent_core::options::r#enum::EnumKvOpts;
use es_fluent_core::options::r#struct::StructKvOpts;
use es_fluent_core::options::this::ThisOpts;
use es_fluent_core::validation;

use heck::{ToPascalCase as _, ToSnakeCase as _};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let this_opts = match ThisOpts::from_derive_input(&input) {
        Ok(opts) => Some(opts),
        Err(_) => None, // Ignore errors, assume no relevant options if parsing fails (or let EsFluentThis handle validation)
    };

    let tokens = match &input.data {
        Data::Struct(data) => {
            let opts = match StructKvOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors().into(),
            };

            if let Err(err) = validation::validate_struct_kv(&opts, data) {
                err.abort();
            }

            process_struct(&opts, data, this_opts.as_ref())
        },
        Data::Enum(data) => {
            let opts = match EnumKvOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors().into(),
            };

            if let Err(err) = validation::validate_enum_kv(&opts, data) {
                err.abort();
            }

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
    let mut derives: Vec<syn::Path> = (*opts.attr_args().derive()).to_vec();

    // If fields_this is true, add EsFluentThis to derives
    if let Some(this_opts) = this_opts {
        if this_opts.attr_args().is_members() {
            derives.push(syn::parse_quote!(::es_fluent::EsFluentThis));
        }
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

    quote! {
      #new_enum

      #display_impl

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
    let mut derives: Vec<syn::Path> = (*opts.attr_args().derive()).to_vec();

    // If variants_this is true, add EsFluentThis
    if let Some(this_opts) = this_opts {
        if this_opts.attr_args().is_members() {
            derives.push(syn::parse_quote!(::es_fluent::EsFluentThis));
        }
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

    quote! {
      #new_enum

      #display_impl

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

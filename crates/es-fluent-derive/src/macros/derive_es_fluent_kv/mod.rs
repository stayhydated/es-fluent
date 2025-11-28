use darling::FromDeriveInput as _;
use es_fluent_core::namer;
use es_fluent_core::options::r#struct::StructKvOpts;
use es_fluent_core::strategy::DisplayStrategy;
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

            process_struct(&opts, data)
        },
        _ => panic!("EsFluentKv can only be used on structs"),
    };

    tokens.into()
}

pub fn process_struct(opts: &StructKvOpts, data: &syn::DataStruct) -> TokenStream {
    let strategy = DisplayStrategy::from(opts);
    let use_fluent_display = matches!(strategy, DisplayStrategy::FluentDisplay);

    let keys = match opts.keyyed_idents() {
        Ok(keys) => keys,
        Err(err) => err.abort(),
    };

    let this_ftl_struct_impl = if opts.attr_args().is_this() {
        let original_ident = opts.ident();
        let (impl_generics, ty_generics, where_clause) = opts.generics().split_for_impl();
        let this_ftl_key = namer::FluentKey::new(original_ident, "").to_string();
        quote! {
            impl #impl_generics #original_ident #ty_generics #where_clause {
                pub fn this_ftl() -> String {
                    ::es_fluent::localize(#this_ftl_key, None)
                }
            }
        }
    } else {
        quote! {}
    };
    if keys.is_empty() {
        let ftl_enum_ident = opts.ftl_enum_ident();
        let ftl_enum = generate_unit_enum(opts, data, use_fluent_display, &ftl_enum_ident);
        quote! {
            #ftl_enum

            #this_ftl_struct_impl
        }
    } else {
        let enums = keys
            .iter()
            .map(|key| generate_unit_enum(opts, data, use_fluent_display, key));

        quote! {
            #(#enums)*

            #this_ftl_struct_impl
        }
    }
}

fn generate_unit_enum(
    opts: &StructKvOpts,
    _data: &syn::DataStruct,
    use_fluent_display: bool,
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
        let trait_impl = if use_fluent_display {
            quote! { ::es_fluent::FluentDisplay }
        } else {
            quote! { ::std::fmt::Display }
        };

        let trait_fmt_fn_ident = if use_fluent_display {
            quote! { fluent_fmt }
        } else {
            quote! { fmt }
        };
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

    let this_ftl_impl =
        if opts.attr_args().is_this() && ident != &opts.ftl_enum_ident() {
            let this_ftl_key = namer::FluentKey::new(ident, "").to_string();
            quote! {
                impl #ident {
                    pub fn this_ftl() -> String {
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
              value.to_string().into()
            }
      }

      impl From<#ident> for ::es_fluent::FluentValue<'_> {
            fn from(value: #ident) -> Self {
                (&value).into()
            }
      }
    }
}

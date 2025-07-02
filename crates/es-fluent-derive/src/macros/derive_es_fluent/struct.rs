use es_fluent_core::namer;
use es_fluent_core::options::r#struct::StructOpts;
use es_fluent_core::strategy::DisplayStrategy;
use heck::{ToPascalCase as _, ToSnakeCase as _};
use proc_macro2::TokenStream;
use quote::quote;

pub fn process_struct(opts: &StructOpts, data: &syn::DataStruct) -> TokenStream {
    let strategy = DisplayStrategy::from(opts);
    let use_fluent_display = matches!(strategy, DisplayStrategy::FluentDisplay);

    let keys = opts.keyyed_idents();
    if keys.is_empty() {
        let ftl_enum_ident = opts.ftl_enum_ident();
        generate_unit_enum(opts, data, use_fluent_display, &ftl_enum_ident)
    } else {
        let enums = keys
            .iter()
            .map(|key| generate_unit_enum(opts, data, use_fluent_display, key));

        quote! {
            #(#enums)*
        }
    }
}

fn generate_unit_enum(
    opts: &StructOpts,
    _data: &syn::DataStruct,
    use_fluent_display: bool,
    ident: &syn::Ident,
) -> TokenStream {
    let fl_path = opts.attr_args().fl();

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
        let ftl_key = namer::FluentKey::new(ident, &base_key);
        quote! {
            Self::#variant_ident => write!(f, "{}", #fl_path!(#ftl_key))
        }
    });

    let cleaned_variants = variants.iter().map(|(ident, _)| ident);

    let new_enum = quote! {
      pub enum #ident {
          #(#cleaned_variants),*
      }
    };

    let default_variant_ident = {
        let field_opts = opts.fields();
        let fluent_default_opt = field_opts.iter().find(|opts| opts.is_default());

        fluent_default_opt.and_then(|opts| {
            opts.ident().as_ref().map(|ident| {
                let pascal_case_name = ident.to_string().to_pascal_case();
                syn::Ident::new(&pascal_case_name, ident.span())
            })
        })
    };
    let default_impl = if let Some(default_variant_ident) = default_variant_ident {
        quote! {
            impl Default for #ident {
                fn default() -> Self {
                    Self::#default_variant_ident
                }
            }
        }
    } else {
        quote! {}
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

    quote! {
      #new_enum

      #default_impl

      #display_impl

      impl From<&#ident> for ::es_fluent::FluentValue<'_> {
            fn from(value: &#ident) -> Self {
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

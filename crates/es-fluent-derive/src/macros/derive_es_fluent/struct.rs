use es_fluent_core::namer;
use es_fluent_core::options::r#struct::StructOpts;
use es_fluent_core::strategy::DisplayStrategy;
use proc_macro2::TokenStream;
use quote::quote;

pub fn process_struct(opts: &StructOpts, _data: &syn::DataStruct) -> TokenStream {
    let strategy = opts.attr_args().display();
    match strategy {
        DisplayStrategy::FluentDisplay => generate(opts, true),
        DisplayStrategy::StdDisplay => generate(opts, false),
    }
}

fn generate(opts: &StructOpts, use_fluent_display: bool) -> TokenStream {
    let original_ident = opts.ident();

    let (impl_generics, ty_generics, where_clause) = opts.generics().split_for_impl();

    let fields = opts.fields();

    let ftl_key = namer::FluentKey::new(original_ident, "").to_string();

    let args: Vec<_> = fields
        .iter()
        .map(|field_opt| {
            let arg_name = field_opt.ident().as_ref().unwrap();
            let arg_key = arg_name.to_string();
            let field_ty = field_opt.ty();
            let is_choice = field_opt.is_choice();

            let value_expr = if is_choice {
                quote! { self.#arg_name.as_fluent_choice() }
            } else {
                let mut current_ty = field_ty;
                let mut deref_count = 0;
                while let syn::Type::Reference(type_ref) = current_ty {
                    deref_count += 1;
                    current_ty = &type_ref.elem;
                }

                if deref_count > 0 {
                    let mut inner = quote! { &self.#arg_name };
                    for _ in 0..deref_count {
                        inner = quote! { (*#inner) };
                    }
                    inner
                } else {
                    quote! { &self.#arg_name }
                }
            };

            quote! { args.insert(#arg_key, ::es_fluent::FluentValue::from(#value_expr)); }
        })
        .collect();

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
            impl #impl_generics #trait_impl for #original_ident #ty_generics #where_clause {
                fn #trait_fmt_fn_ident(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    let mut args = ::std::collections::HashMap::new();
                    #(#args)*
                    write!(f, "{}", ::es_fluent::localize(#ftl_key, Some(&args)))
                }
            }
        }
    };

    let fluent_value_inner_fn = if use_fluent_display {
        quote! {
          use ::es_fluent::ToFluentString as _;
          value.to_fluent_string().into()
        }
    } else {
        quote! {
          value.to_string().into()
        }
    };

    let this_ftl_impl = if opts.attr_args().is_this() {
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

    quote! {
      #display_impl

      #this_ftl_impl

      impl #impl_generics From<&#original_ident #ty_generics> for ::es_fluent::FluentValue<'_> #where_clause {
            fn from(value: &#original_ident #ty_generics) -> Self {
              #fluent_value_inner_fn
            }
      }

      impl #impl_generics From<#original_ident #ty_generics> for ::es_fluent::FluentValue<'_> #where_clause {
            fn from(value: #original_ident #ty_generics) -> Self {
                (&value).into()
            }
      }
    }
}

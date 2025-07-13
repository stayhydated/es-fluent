use es_fluent_core::namer;
use es_fluent_core::options::r#enum::EnumOpts;
use es_fluent_core::strategy::DisplayStrategy;
use proc_macro2::TokenStream;
use quote::quote;

pub fn process_enum(opts: &EnumOpts, data: &syn::DataEnum) -> TokenStream {
    let strategy = opts.attr_args().display();
    match strategy {
        DisplayStrategy::FluentDisplay => generate(opts, data, true),
        DisplayStrategy::StdDisplay => generate(opts, data, false),
    }
}

fn generate(opts: &EnumOpts, _data: &syn::DataEnum, use_fluent_display: bool) -> TokenStream {
    let original_ident = opts.ident();
    let fl_path = opts.attr_args().fl();

    let (impl_generics, ty_generics, where_clause) = opts.generics().split_for_impl();

    let variants = opts.variants();

    let match_arms = variants.iter().map(|variant_opt| {
        let variant_ident = variant_opt.ident();

        match variant_opt.style() {
            darling::ast::Style::Unit => {
                let ftl_key = namer::FluentKey::new(original_ident, &variant_ident.to_string());
                quote! {
                    Self::#variant_ident => write!(f, "{}", #fl_path!(#ftl_key))
                }
            },
            darling::ast::Style::Tuple => {
                let all_fields = variant_opt.all_fields();
                let field_pats: Vec<_> = all_fields
                    .iter()
                    .enumerate()
                    .map(|(index, field)| {
                        if field.is_skipped() {
                            quote! { _ }
                        } else {
                            let name = namer::UnnamedItem::from(index).to_ident();
                            quote! { #name }
                        }
                    })
                    .collect();

                let ftl_key = namer::FluentKey::new(original_ident, &variant_ident.to_string());

                let args: Vec<_> = all_fields
                    .iter()
                    .enumerate()
                    .filter_map(|(index, field)| {
                        if field.is_skipped() {
                            return None;
                        }

                        let arg_name = namer::UnnamedItem::from(index).to_ident();
                        let field_ty = field.ty();
                        let is_choice = field.is_choice();

                        Some(generate_fluent_arg(&arg_name, field_ty, is_choice))
                    })
                    .collect();

                let ftl_args = if !args.is_empty() {
                    quote! { , #(#args),* }
                } else {
                    quote! {}
                };

                quote! {
                    Self::#variant_ident(#(#field_pats),*) => {
                        write!(f, "{}", #fl_path!(#ftl_key #ftl_args))
                    }
                }
            },
            darling::ast::Style::Struct => {
                let fields = variant_opt.fields();
                let field_pats: Vec<_> =
                    fields.iter().map(|f| f.ident().as_ref().unwrap()).collect();

                let ftl_key = namer::FluentKey::new(original_ident, &variant_ident.to_string());

                let args: Vec<_> = fields
                    .iter()
                    .map(|field_opt| {
                        let arg_name = field_opt.ident().as_ref().unwrap();
                        let field_ty = field_opt.ty();
                        let is_choice = field_opt.is_choice();

                        generate_fluent_arg(arg_name, field_ty, is_choice)
                    })
                    .collect();

                let ftl_args = if !args.is_empty() {
                    quote! { , #(#args),* }
                } else {
                    quote! {}
                };

                let all_fields = variant_opt.all_fields();
                let has_skipped_fields = all_fields.len() > fields.len();

                let pattern = if has_skipped_fields {
                    quote! { Self::#variant_ident { #(#field_pats),*, .. } }
                } else {
                    quote! { Self::#variant_ident { #(#field_pats),* } }
                };

                quote! {
                    #pattern => {
                        write!(f, "{}", #fl_path!(#ftl_key #ftl_args))
                    }
                }
            },
        }
    });

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
                    match self {
                        #(#match_arms),*
                    }
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
        let this_ftl_key = namer::FluentKey::new(original_ident, "");
        quote! {
            impl #impl_generics #original_ident #ty_generics #where_clause {
                pub fn this_ftl() -> &'static str {
                    #this_ftl_key
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

fn generate_fluent_arg(
    arg_name: &proc_macro2::Ident,
    field_ty: &syn::Type,
    is_choice: bool,
) -> TokenStream {
    if is_choice {
        return quote! { #arg_name = #arg_name.as_fluent_choice() };
    }

    let mut current_ty = field_ty;
    let mut deref_count = 0;
    while let syn::Type::Reference(type_ref) = current_ty {
        deref_count += 1;
        current_ty = &type_ref.elem;
    }

    if deref_count > 0 {
        let mut inner = quote! { #arg_name };
        for _ in 0..deref_count {
            inner = quote! { (*#inner) };
        }
        quote! { #arg_name = #inner }
    } else {
        quote! { #arg_name = #arg_name }
    }
}

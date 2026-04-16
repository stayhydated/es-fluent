use heck::ToSnakeCase as _;
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input, spanned::Spanned as _};

fn bevy_fluent_text_registration_module(
    mod_name: &syn::Ident,
    register_call: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        #[doc(hidden)]
        mod #mod_name {
            use super::*;

            struct Registration;

            impl ::es_fluent_manager_bevy::BevyFluentTextRegistration for Registration {
                fn register(&self, app: &mut ::es_fluent_manager_bevy::bevy::prelude::App) {
                    #register_call
                }
            }

            ::es_fluent_manager_bevy::inventory::submit!(
                &Registration as &dyn ::es_fluent_manager_bevy::BevyFluentTextRegistration
            );
        }
    }
}

pub(crate) fn derive_bevy_fluent_text(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;
    let type_name = ident.to_string();

    // Collect all locale fields from all variants/fields
    let locale_fields = match collect_locale_fields(&input.data) {
        Ok(locale_fields) => locale_fields,
        Err(err) => return TokenStream::from(err.to_compile_error()),
    };

    let mod_name = quote::format_ident!(
        "__bevy_fluent_text_registration_{}",
        type_name.to_snake_case()
    );

    if locale_fields.is_empty() {
        // Simple registration without locale refresh
        let registration_module = bevy_fluent_text_registration_module(
            &mod_name,
            quote! {
                ::es_fluent_manager_bevy::FluentTextRegistration::register_fluent_text::<#ident>(app);
            },
        );
        TokenStream::from(quote! { #registration_module })
    } else {
        // Generate RefreshForLocale impl and use locale-aware registration
        let refresh_impl = generate_refresh_for_locale_impl(ident, &input.data, &locale_fields);
        let registration_module = bevy_fluent_text_registration_module(
            &mod_name,
            quote! {
                ::es_fluent_manager_bevy::FluentTextRegistration::register_fluent_text_from_locale::<#ident>(app);
            },
        );

        TokenStream::from(quote! {
            #refresh_impl
            #registration_module
        })
    }
}

/// Information about a field marked with #[locale]
#[derive(Debug)]
struct LocaleFieldInfo {
    /// The variant this field belongs to (for enums)
    variant_ident: Option<syn::Ident>,
    /// The locale-marked fields that should refresh together
    locale_fields: Vec<syn::Ident>,
    /// Other fields in the same variant (for pattern matching)
    other_fields: Vec<syn::Ident>,
}

/// Collects all fields marked with #[locale] from the data structure
fn collect_locale_fields(data: &syn::Data) -> syn::Result<Vec<LocaleFieldInfo>> {
    let mut locale_fields = Vec::new();

    match data {
        syn::Data::Enum(data_enum) => {
            for variant in &data_enum.variants {
                match &variant.fields {
                    syn::Fields::Named(fields) => {
                        let all_field_idents: Vec<_> = fields
                            .named
                            .iter()
                            .filter_map(|f| f.ident.clone())
                            .collect();
                        let locale_field_idents: Vec<_> = fields
                            .named
                            .iter()
                            .filter(|field| has_locale_attr(field))
                            .filter_map(|field| field.ident.clone())
                            .collect();

                        if !locale_field_idents.is_empty() {
                            let other_fields: Vec<_> = all_field_idents
                                .iter()
                                .filter(|id| !locale_field_idents.contains(id))
                                .cloned()
                                .collect();

                            locale_fields.push(LocaleFieldInfo {
                                variant_ident: Some(variant.ident.clone()),
                                locale_fields: locale_field_idents,
                                other_fields,
                            });
                        }
                    },
                    syn::Fields::Unnamed(fields) => {
                        if let Some(field) =
                            fields.unnamed.iter().find(|field| has_locale_attr(field))
                        {
                            return Err(unsupported_locale_field_error(field));
                        }
                    },
                    syn::Fields::Unit => {},
                }
            }
        },
        syn::Data::Struct(data_struct) => match &data_struct.fields {
            syn::Fields::Named(fields) => {
                let locale_field_idents: Vec<_> = fields
                    .named
                    .iter()
                    .filter(|field| has_locale_attr(field))
                    .filter_map(|field| field.ident.clone())
                    .collect();

                if !locale_field_idents.is_empty() {
                    locale_fields.push(LocaleFieldInfo {
                        variant_ident: None,
                        locale_fields: locale_field_idents,
                        other_fields: Vec::new(),
                    });
                }
            },
            syn::Fields::Unnamed(fields) => {
                if let Some(field) = fields.unnamed.iter().find(|field| has_locale_attr(field)) {
                    return Err(unsupported_locale_field_error(field));
                }
            },
            syn::Fields::Unit => {},
        },
        syn::Data::Union(_) => {},
    }

    Ok(locale_fields)
}

/// Checks if a field has the #[locale] attribute
fn has_locale_attr(field: &syn::Field) -> bool {
    field
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("locale"))
}

fn unsupported_locale_field_error(field: &syn::Field) -> syn::Error {
    let span = field
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("locale"))
        .map(|attr| attr.span())
        .unwrap_or_else(|| field.span());
    syn::Error::new(
        span,
        "#[locale] is only supported on named struct fields and named enum variant fields",
    )
}

/// Generates the RefreshForLocale implementation
fn generate_refresh_for_locale_impl(
    ident: &syn::Ident,
    data: &syn::Data,
    locale_fields: &[LocaleFieldInfo],
) -> proc_macro2::TokenStream {
    match data {
        syn::Data::Enum(_) => {
            // Group locale fields by variant
            let match_arms: Vec<_> = locale_fields
                .iter()
                .map(|info| {
                    let variant_ident =
                        info.variant_ident.as_ref().expect("enum field has variant");
                    let locale_field_idents = &info.locale_fields;
                    let other_fields = &info.other_fields;

                    let locale_patterns: Vec<_> = locale_field_idents
                        .iter()
                        .map(|field| quote! { #field })
                        .collect();
                    let other_patterns: Vec<_> =
                        other_fields.iter().map(|f| quote! { #f: _ }).collect();
                    let field_updates: Vec<_> = locale_field_idents
                        .iter()
                        .map(|field_ident| {
                            quote! {
                                if let Ok(value) = ::std::convert::TryFrom::try_from(lang) {
                                    *#field_ident = value;
                                }
                            }
                        })
                        .collect();

                    quote! {
                        Self::#variant_ident { #(#locale_patterns,)* #(#other_patterns),* } => {
                            #(#field_updates)*
                        }
                    }
                })
                .collect();

            quote! {
                impl ::es_fluent_manager_bevy::RefreshForLocale for #ident {
                    fn refresh_for_locale(&mut self, lang: &::es_fluent_manager_bevy::unic_langid::LanguageIdentifier) {
                        match self {
                            #(#match_arms)*
                            _ => {}
                        }
                    }
                }
            }
        },
        syn::Data::Struct(_) => {
            let field_updates: Vec<_> = locale_fields
                .iter()
                .flat_map(|info| info.locale_fields.iter())
                .map(|field_ident| {
                    quote! {
                        if let Ok(value) = ::std::convert::TryFrom::try_from(lang) {
                            self.#field_ident = value;
                        }
                    }
                })
                .collect();

            quote! {
                impl ::es_fluent_manager_bevy::RefreshForLocale for #ident {
                    fn refresh_for_locale(&mut self, lang: &::es_fluent_manager_bevy::unic_langid::LanguageIdentifier) {
                        #(#field_updates)*
                    }
                }
            }
        },
        syn::Data::Union(_) => quote! {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn locale_field_collection_and_generation_cover_enum_struct_and_union() {
        let enum_input: DeriveInput = syn::parse_quote! {
            enum Example {
                A {
                    #[locale]
                    current_language: Lang,
                    #[locale]
                    fallback_language: Lang,
                    count: usize,
                },
                B { value: usize },
            }
        };

        let enum_fields = collect_locale_fields(&enum_input.data).expect("collect locale fields");
        assert_eq!(enum_fields.len(), 1);
        assert_eq!(
            enum_fields[0]
                .variant_ident
                .as_ref()
                .expect("variant")
                .to_string(),
            "A"
        );
        assert_eq!(
            enum_fields[0]
                .locale_fields
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec![
                "current_language".to_string(),
                "fallback_language".to_string()
            ]
        );
        assert_eq!(enum_fields[0].other_fields.len(), 1);
        let enum_tokens =
            generate_refresh_for_locale_impl(&enum_input.ident, &enum_input.data, &enum_fields)
                .to_string();
        assert!(enum_tokens.contains("match"));
        assert!(enum_tokens.contains("current_language"));
        assert!(enum_tokens.contains("fallback_language"));
        assert_eq!(enum_tokens.matches("Self :: A").count(), 1);

        let struct_input: DeriveInput = syn::parse_quote! {
            struct ExampleStruct {
                #[locale]
                locale: Lang,
                value: usize,
            }
        };
        let struct_fields =
            collect_locale_fields(&struct_input.data).expect("collect struct locale fields");
        assert_eq!(struct_fields.len(), 1);
        assert!(struct_fields[0].variant_ident.is_none());
        let struct_tokens = generate_refresh_for_locale_impl(
            &struct_input.ident,
            &struct_input.data,
            &struct_fields,
        )
        .to_string();
        assert!(struct_tokens.contains("self . locale"));

        let union_input: DeriveInput = syn::parse_quote! {
            union ExampleUnion {
                a: u32,
                b: f32,
            }
        };
        let union_fields = collect_locale_fields(&union_input.data).expect("collect union fields");
        assert!(union_fields.is_empty());
        let union_tokens =
            generate_refresh_for_locale_impl(&union_input.ident, &union_input.data, &union_fields)
                .to_string();
        assert_eq!(union_tokens, "");
    }

    #[test]
    fn locale_attr_and_registration_module_helpers_emit_expected_tokens() {
        let locale_field: syn::Field = syn::parse_quote! {
            #[locale]
            language: Lang
        };
        let plain_field: syn::Field = syn::parse_quote! {
            language: Lang
        };
        assert!(has_locale_attr(&locale_field));
        assert!(!has_locale_attr(&plain_field));

        let module_tokens = bevy_fluent_text_registration_module(
            &syn::Ident::new("__test_module", proc_macro2::Span::call_site()),
            quote! { register_me(app); },
        )
        .to_string();

        assert!(module_tokens.contains("__test_module"));
        assert!(module_tokens.contains("register_me"));
        assert!(module_tokens.contains("inventory"));
    }

    #[test]
    fn locale_field_collection_rejects_tuple_struct_and_tuple_variant_fields() {
        let tuple_struct_input: DeriveInput = syn::parse_quote! {
            struct ExampleTupleStruct(#[locale] Lang, usize);
        };
        let tuple_struct_err =
            collect_locale_fields(&tuple_struct_input.data).expect_err("tuple struct should error");
        assert!(tuple_struct_err.to_string().contains("named struct fields"));

        let tuple_enum_input: DeriveInput = syn::parse_quote! {
            enum ExampleTupleEnum {
                A(#[locale] Lang, usize),
            }
        };
        let tuple_enum_err =
            collect_locale_fields(&tuple_enum_input.data).expect_err("tuple variant should error");
        assert!(
            tuple_enum_err
                .to_string()
                .contains("named enum variant fields")
        );
    }
}

use heck::ToSnakeCase as _;
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

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
    let locale_fields = collect_locale_fields(&input.data);

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
struct LocaleFieldInfo {
    /// The variant this field belongs to (for enums)
    variant_ident: Option<syn::Ident>,
    /// The field identifier
    field_ident: syn::Ident,
    /// Other fields in the same variant (for pattern matching)
    other_fields: Vec<syn::Ident>,
}

/// Collects all fields marked with #[locale] from the data structure
fn collect_locale_fields(data: &syn::Data) -> Vec<LocaleFieldInfo> {
    let mut locale_fields = Vec::new();

    match data {
        syn::Data::Enum(data_enum) => {
            for variant in &data_enum.variants {
                if let syn::Fields::Named(fields) = &variant.fields {
                    let all_field_idents: Vec<_> = fields
                        .named
                        .iter()
                        .filter_map(|f| f.ident.clone())
                        .collect();

                    for field in &fields.named {
                        if has_locale_attr(field)
                            && let Some(field_ident) = &field.ident
                        {
                            let other_fields: Vec<_> = all_field_idents
                                .iter()
                                .filter(|id| *id != field_ident)
                                .cloned()
                                .collect();

                            locale_fields.push(LocaleFieldInfo {
                                variant_ident: Some(variant.ident.clone()),
                                field_ident: field_ident.clone(),
                                other_fields,
                            });
                        }
                    }
                }
            }
        },
        syn::Data::Struct(data_struct) => {
            if let syn::Fields::Named(fields) = &data_struct.fields {
                let all_field_idents: Vec<_> = fields
                    .named
                    .iter()
                    .filter_map(|f| f.ident.clone())
                    .collect();

                for field in &fields.named {
                    if has_locale_attr(field)
                        && let Some(field_ident) = &field.ident
                    {
                        let other_fields: Vec<_> = all_field_idents
                            .iter()
                            .filter(|id| *id != field_ident)
                            .cloned()
                            .collect();

                        locale_fields.push(LocaleFieldInfo {
                            variant_ident: None,
                            field_ident: field_ident.clone(),
                            other_fields,
                        });
                    }
                }
            }
        },
        syn::Data::Union(_) => {},
    }

    locale_fields
}

/// Checks if a field has the #[locale] attribute
fn has_locale_attr(field: &syn::Field) -> bool {
    field
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("locale"))
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
                    let field_ident = &info.field_ident;
                    let other_fields = &info.other_fields;

                    let other_patterns: Vec<_> =
                        other_fields.iter().map(|f| quote! { #f: _ }).collect();

                    quote! {
                        Self::#variant_ident { #field_ident, #(#other_patterns),* } => {
                            if let Ok(value) = ::std::convert::TryFrom::try_from(lang) {
                                *#field_ident = value;
                            }
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
                .map(|info| {
                    let field_ident = &info.field_ident;
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
                    count: usize,
                },
                B { value: usize },
            }
        };

        let enum_fields = collect_locale_fields(&enum_input.data);
        assert_eq!(enum_fields.len(), 1);
        assert_eq!(
            enum_fields[0]
                .variant_ident
                .as_ref()
                .expect("variant")
                .to_string(),
            "A"
        );
        assert_eq!(enum_fields[0].field_ident.to_string(), "current_language");
        assert_eq!(enum_fields[0].other_fields.len(), 1);
        let enum_tokens =
            generate_refresh_for_locale_impl(&enum_input.ident, &enum_input.data, &enum_fields)
                .to_string();
        assert!(enum_tokens.contains("match"));
        assert!(enum_tokens.contains("current_language"));

        let struct_input: DeriveInput = syn::parse_quote! {
            struct ExampleStruct {
                #[locale]
                locale: Lang,
                value: usize,
            }
        };
        let struct_fields = collect_locale_fields(&struct_input.data);
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
        let union_fields = collect_locale_fields(&union_input.data);
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
}

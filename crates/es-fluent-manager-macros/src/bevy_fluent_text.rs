use heck::ToSnakeCase as _;
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

use es_fluent_derive_core::attribute::AttributeLocation;

fn bevy_fluent_text_registration_module(
    mod_name: &syn::Ident,
    register_call: proc_macro2::TokenStream,
    manager_path: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        #[doc(hidden)]
        mod #mod_name {
            use super::*;

            struct Registration;

            impl #manager_path::BevyFluentTextRegistration for Registration {
                fn register(&self, app: &mut #manager_path::bevy::prelude::App) {
                    #register_call
                }
            }

            #manager_path::inventory::submit!(
                &Registration as &dyn #manager_path::BevyFluentTextRegistration
            );
        }
    }
}

pub(crate) fn derive_bevy_fluent_text(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;
    let type_name = ident.to_string();
    let manager_path = crate::support::bevy_manager_path();
    let manager_path_tokens = manager_path.tokens();

    if matches!(&input.data, syn::Data::Union(_)) {
        return TokenStream::from(
            syn::Error::new(
                input.ident.span(),
                "BevyFluentText can only be derived for structs and enums",
            )
            .to_compile_error(),
        );
    }

    // Collect all locale fields from all variants/fields
    let locale_fields = match collect_locale_fields(&input.data) {
        Ok(locale_fields) => locale_fields,
        Err(err) => return TokenStream::from(crate::support::core_error_to_compile_error(err)),
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
                #manager_path_tokens::FluentTextRegistration::register_fluent_text::<#ident>(app);
            },
            manager_path_tokens,
        );
        TokenStream::from(quote! { #registration_module })
    } else {
        // Generate RefreshForLocale impl and use locale-aware registration
        let refresh_impl = generate_refresh_for_locale_impl(
            ident,
            &input.data,
            &locale_fields,
            manager_path_tokens,
        );
        let registration_module = bevy_fluent_text_registration_module(
            &mod_name,
            quote! {
                #manager_path_tokens::FluentTextRegistration::register_fluent_text_from_locale::<#ident>(app);
            },
            manager_path_tokens,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LocaleFieldDirective {
    Locale,
    NotLocale,
}

impl LocaleFieldDirective {
    fn is_locale(self) -> bool {
        matches!(self, Self::Locale)
    }
}

/// Collects all fields marked with #[locale] from the data structure
fn collect_locale_fields(
    data: &syn::Data,
) -> Result<Vec<LocaleFieldInfo>, es_fluent_derive_core::error::EsFluentCoreError> {
    let mut locale_fields = Vec::new();

    match data {
        syn::Data::Enum(data_enum) => {
            for variant in &data_enum.variants {
                match &variant.fields {
                    syn::Fields::Named(fields) => {
                        let mut all_field_idents = Vec::new();
                        let mut locale_field_idents = Vec::new();

                        for field in &fields.named {
                            if let Some(ident) = field.ident.clone() {
                                all_field_idents.push(ident.clone());
                                if locale_field_directive(
                                    field,
                                    AttributeLocation::LocaleNamedEnumVariantField,
                                )?
                                .is_locale()
                                {
                                    locale_field_idents.push(ident);
                                }
                            }
                        }

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
                        for field in &fields.unnamed {
                            locale_field_directive(
                                field,
                                AttributeLocation::LocaleTupleEnumVariantField,
                            )?;
                        }
                    },
                    syn::Fields::Unit => {},
                }
            }
        },
        syn::Data::Struct(data_struct) => match &data_struct.fields {
            syn::Fields::Named(fields) => {
                let mut locale_field_idents = Vec::new();
                for field in &fields.named {
                    if locale_field_directive(field, AttributeLocation::LocaleNamedStructField)?
                        .is_locale()
                    {
                        if let Some(ident) = field.ident.clone() {
                            locale_field_idents.push(ident);
                        }
                    }
                }

                if !locale_field_idents.is_empty() {
                    locale_fields.push(LocaleFieldInfo {
                        variant_ident: None,
                        locale_fields: locale_field_idents,
                        other_fields: Vec::new(),
                    });
                }
            },
            syn::Fields::Unnamed(fields) => {
                for field in &fields.unnamed {
                    locale_field_directive(field, AttributeLocation::LocaleTupleStructField)?;
                }
            },
            syn::Fields::Unit => {},
        },
        syn::Data::Union(_) => {},
    }

    Ok(locale_fields)
}

fn locale_field_directive(
    field: &syn::Field,
    location: AttributeLocation,
) -> Result<LocaleFieldDirective, es_fluent_derive_core::error::EsFluentCoreError> {
    for attr in &field.attrs {
        if crate::support::validate_locale_marker(attr, location)? {
            return Ok(LocaleFieldDirective::Locale);
        }
    }

    Ok(LocaleFieldDirective::NotLocale)
}

/// Generates the RefreshForLocale implementation
fn generate_refresh_for_locale_impl(
    ident: &syn::Ident,
    data: &syn::Data,
    locale_fields: &[LocaleFieldInfo],
    manager_path: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    match data {
        syn::Data::Enum(_) => {
            // Group locale fields by variant
            let match_arms: Vec<_> = locale_fields
                .iter()
                .map(|info| {
                    let Some(variant_ident) = info.variant_ident.as_ref() else {
                        return quote! {
                            compile_error!("internal error: enum locale field missing variant");
                        };
                    };
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
                impl #manager_path::RefreshForLocale for #ident {
                    fn refresh_for_locale(&mut self, lang: &#manager_path::unic_langid::LanguageIdentifier) {
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
                impl #manager_path::RefreshForLocale for #ident {
                    fn refresh_for_locale(&mut self, lang: &#manager_path::unic_langid::LanguageIdentifier) {
                        #(#field_updates)*
                    }
                }
            }
        },
        syn::Data::Union(_) => quote! {},
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use quote::quote;

    fn pretty_tokens(tokens: proc_macro2::TokenStream) -> String {
        let file: syn::File =
            syn::parse2(tokens).expect("generated tokens should parse as a Rust file");
        prettyplease::unparse(&file).trim().to_string()
    }

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
        let manager_path = crate::support::bevy_manager_path();
        let enum_tokens = generate_refresh_for_locale_impl(
            &enum_input.ident,
            &enum_input.data,
            &enum_fields,
            manager_path.tokens(),
        );
        assert_snapshot!(
            "generate_refresh_for_locale_impl_enum",
            pretty_tokens(enum_tokens)
        );

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
            manager_path.tokens(),
        );
        assert_snapshot!(
            "generate_refresh_for_locale_impl_struct",
            pretty_tokens(struct_tokens)
        );

        let union_input: DeriveInput = syn::parse_quote! {
            union ExampleUnion {
                a: u32,
                b: f32,
            }
        };
        let union_fields = collect_locale_fields(&union_input.data).expect("collect union fields");
        assert!(union_fields.is_empty());
        let union_tokens = generate_refresh_for_locale_impl(
            &union_input.ident,
            &union_input.data,
            &union_fields,
            manager_path.tokens(),
        )
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
        assert!(
            locale_field_directive(&locale_field, AttributeLocation::LocaleNamedStructField)
                .expect("locale attr")
                .is_locale()
        );
        assert!(
            !locale_field_directive(&plain_field, AttributeLocation::LocaleNamedStructField)
                .expect("plain field")
                .is_locale()
        );

        let invalid_locale_field: syn::Field = syn::parse_quote! {
            #[locale(true)]
            language: Lang
        };
        let err = locale_field_directive(
            &invalid_locale_field,
            AttributeLocation::LocaleNamedStructField,
        )
        .expect_err("invalid locale attr");
        assert!(err.to_string().contains("wrong value shape"));
        assert!(err.to_string().contains("help: use a bare marker"));

        let module_tokens = bevy_fluent_text_registration_module(
            &syn::Ident::new("__test_module", proc_macro2::Span::call_site()),
            quote! { register_me(app); },
            crate::support::bevy_manager_path().tokens(),
        );
        assert_snapshot!(
            "bevy_fluent_text_registration_module",
            pretty_tokens(module_tokens)
        );
    }

    #[test]
    fn locale_field_collection_rejects_tuple_struct_and_tuple_variant_fields() {
        let tuple_struct_input: DeriveInput = syn::parse_quote! {
            struct ExampleTupleStruct(#[locale] Lang, usize);
        };
        let tuple_struct_err =
            collect_locale_fields(&tuple_struct_input.data).expect_err("tuple struct should error");
        assert_snapshot!(
            "locale_field_collection_rejects_tuple_struct_fields",
            tuple_struct_err.to_string()
        );

        let tuple_enum_input: DeriveInput = syn::parse_quote! {
            enum ExampleTupleEnum {
                A(#[locale] Lang, usize),
            }
        };
        let tuple_enum_err =
            collect_locale_fields(&tuple_enum_input.data).expect_err("tuple variant should error");
        assert_snapshot!(
            "locale_field_collection_rejects_tuple_variant_fields",
            tuple_enum_err.to_string()
        );

        let invalid_attr_input: DeriveInput = syn::parse_quote! {
            struct ExampleStruct {
                #[locale = true]
                language: Lang,
            }
        };
        let invalid_attr_err =
            collect_locale_fields(&invalid_attr_input.data).expect_err("attr syntax should error");
        assert!(invalid_attr_err.to_string().contains("wrong value shape"));
        assert!(
            invalid_attr_err
                .to_string()
                .contains("help: use a bare marker")
        );
    }
}

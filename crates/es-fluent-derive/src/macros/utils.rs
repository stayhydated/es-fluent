use darling::FromDeriveInput as _;
use es_fluent_derive_core::options::{
    FluentField, GeneratedVariantsOptions, r#enum::EnumOpts, r#struct::StructOpts,
};
use es_fluent_shared::{namer, namespace::NamespaceRule};
use heck::ToSnakeCase as _;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput};

use crate::macros::ir::{FluentArgument, GeneratedUnitEnumVariant, InventoryVariantSpec};

pub struct InventoryModuleInput<'a> {
    pub ident: &'a syn::Ident,
    pub module_name_prefix: &'a str,
    pub type_kind: TokenStream,
    pub variants: Vec<TokenStream>,
    pub namespace_expr: TokenStream,
}

pub struct GeneratedUnitEnumInput<'a> {
    pub ident: &'a syn::Ident,
    pub origin_ident: &'a syn::Ident,
    pub key_name: Option<&'a str>,
    pub domain_override: Option<&'a str>,
    pub derives: &'a [syn::Path],
    pub variants: &'a [GeneratedUnitEnumVariant],
    pub namespace_expr: TokenStream,
    pub this_key: Option<String>,
}

pub fn keyed_variant_idents_or_abort(opts: &impl GeneratedVariantsOptions) -> Vec<syn::Ident> {
    match opts.keyed_idents() {
        Ok(keys) => keys,
        Err(err) => err.abort(),
    }
}

pub fn emit_default_or_keyed_items(
    default_ident: &syn::Ident,
    keys: &[syn::Ident],
    key_strings: &[String],
    mut emit: impl FnMut(&syn::Ident, Option<&str>) -> TokenStream,
) -> TokenStream {
    if keys.is_empty() {
        return emit(default_ident, None);
    }

    let items = keys
        .iter()
        .zip(key_strings.iter())
        .map(|(key, key_str)| emit(key, Some(key_str.as_str())));

    quote! {
        #(#items)*
    }
}

/// Generates the `ThisFtl` trait implementation.
pub fn generate_this_ftl_impl(
    ident: &syn::Ident,
    generics: &syn::Generics,
    ftl_key: Option<&str>,
    domain_override: Option<&str>,
) -> TokenStream {
    let Some(ftl_key) = ftl_key else {
        return quote! {};
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let domain_expr = match domain_override {
        Some(domain) => quote! { #domain },
        None => quote! { env!("CARGO_PKG_NAME") },
    };
    quote! {
        impl #impl_generics ::es_fluent::ThisFtl for #ident #ty_generics #where_clause {
            fn this_ftl<__EsFluentLocalizer: ::es_fluent::FluentLocalizer + ?Sized>(
                localizer: &__EsFluentLocalizer,
            ) -> String {
                ::es_fluent::__private::localize_this(localizer, #domain_expr, #ftl_key)
            }
        }
    }
}

pub fn generate_field_value_expr(
    access_expr: TokenStream,
    transform_arg_expr: TokenStream,
    value_expr: Option<&syn::Expr>,
    is_choice: bool,
) -> TokenStream {
    if let Some(expr) = value_expr {
        quote! { (#expr)(#transform_arg_expr) }
    } else if is_choice {
        quote! { { use ::es_fluent::EsFluentChoice as _; (#access_expr).as_fluent_choice() } }
    } else {
        quote! { (#access_expr).clone() }
    }
}

pub fn generate_field_argument(
    field: &impl FluentField,
    index: usize,
    access_expr: TokenStream,
    transform_arg_expr: TokenStream,
) -> FluentArgument {
    let value_expr = generate_field_value_expr(
        access_expr,
        transform_arg_expr,
        field.value(),
        field.is_choice(),
    );

    FluentArgument {
        key: field.fluent_arg_name(index),
        value_expr,
    }
}

pub fn inventory_arg_name(field: &impl FluentField, index: usize) -> String {
    field.fluent_arg_name(index)
}

pub fn variant_ftl_key(
    base_key: &str,
    variant_ident: &syn::Ident,
    override_key: Option<&str>,
) -> String {
    let variant_key_suffix = override_key
        .map(str::to_owned)
        .unwrap_or_else(|| variant_ident.to_string());
    namer::FluentKey::from(base_key)
        .join(&variant_key_suffix)
        .to_string()
}

pub fn inventory_variant_tokens(
    name: impl Into<String>,
    ftl_key: String,
    arg_names: Vec<String>,
) -> TokenStream {
    InventoryVariantSpec {
        name: name.into(),
        ftl_key,
        arg_names,
    }
    .tokens()
}

pub fn generate_fluent_message_impl(
    ident: &syn::Ident,
    generics: &syn::Generics,
    body: TokenStream,
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics ::es_fluent::FluentMessage for #ident #ty_generics #where_clause {
            fn to_fluent_string_with(
                &self,
                localize: &mut dyn for<'__es_fluent_message> FnMut(
                    &str,
                    &str,
                    Option<&::std::collections::HashMap<&str, ::es_fluent::FluentValue<'__es_fluent_message>>>,
                ) -> String,
            ) -> String {
                #body
            }
        }
    }
}

pub fn generate_unit_enum_definition(
    ident: &syn::Ident,
    origin_ident: &syn::Ident,
    key_name: Option<&str>,
    derives: &[syn::Path],
    variants: &[GeneratedUnitEnumVariant],
) -> TokenStream {
    let cleaned_variants = variants.iter().map(|entry| &entry.ident);
    let derive_attr = if !derives.is_empty() {
        quote! { #[derive(#(#derives),*)] }
    } else {
        quote! {}
    };

    let enum_doc = match key_name {
        Some(key) => format!("`{key}` variants of [`{origin_ident}`]."),
        None => format!("Variants of [`{origin_ident}`]."),
    };
    let variant_docs: Vec<_> = variants
        .iter()
        .map(|entry| match key_name {
            Some(key) => format!(
                "The `{}` `{key}` variant of [`{origin_ident}`].",
                entry.doc_name
            ),
            None => format!("The `{}` variant of [`{origin_ident}`].", entry.doc_name),
        })
        .collect();

    quote! {
        #[doc = #enum_doc]
        #derive_attr
        pub enum #ident {
            #(#[doc = #variant_docs] #cleaned_variants),*
        }
    }
}

pub fn generate_optional_this_inventory_module(
    ident: &syn::Ident,
    namespace_expr: TokenStream,
    this_key: Option<&str>,
) -> TokenStream {
    let Some(this_key) = this_key else {
        return quote! {};
    };

    let this_variant =
        inventory_variant_tokens(ident.to_string(), this_key.to_string(), Vec::new());

    generate_inventory_module(InventoryModuleInput {
        ident,
        module_name_prefix: "this_inventory",
        type_kind: quote! { ::es_fluent::meta::TypeKind::Enum },
        variants: vec![this_variant],
        namespace_expr,
    })
}

pub fn emit_generated_unit_enum(input: GeneratedUnitEnumInput<'_>) -> TokenStream {
    let GeneratedUnitEnumInput {
        ident,
        origin_ident,
        key_name,
        domain_override,
        derives,
        variants,
        namespace_expr,
        this_key,
    } = input;

    let empty_generics = syn::Generics::default();
    let new_enum = generate_unit_enum_definition(ident, origin_ident, key_name, derives, variants);
    let localize_with_match_arms = variants
        .iter()
        .map(|variant| variant.localize_with_match_arm(domain_override));
    let message_impl = generate_fluent_message_impl(
        ident,
        &empty_generics,
        quote! {
            match self {
                #(#localize_with_match_arms),*
            }
        },
    );
    let inventory_submit = generate_inventory_module(InventoryModuleInput {
        ident,
        module_name_prefix: "inventory",
        type_kind: quote! { ::es_fluent::meta::TypeKind::Enum },
        variants: variants
            .iter()
            .map(GeneratedUnitEnumVariant::inventory_variant_tokens)
            .collect(),
        namespace_expr: namespace_expr.clone(),
    });
    let this_impl =
        generate_this_ftl_impl(ident, &empty_generics, this_key.as_deref(), domain_override);
    let this_inventory =
        generate_optional_this_inventory_module(ident, namespace_expr, this_key.as_deref());

    quote! {
        #new_enum

        #message_impl

        #inventory_submit

        #this_impl
        #this_inventory
    }
}

pub fn emit_message_inventory_impls(
    ident: &syn::Ident,
    generics: &syn::Generics,
    fluent_message_body: TokenStream,
    inventory_submit: TokenStream,
) -> TokenStream {
    let message_impl = generate_fluent_message_impl(ident, generics, fluent_message_body);

    quote! {
        #message_impl

        #inventory_submit
    }
}

pub fn generate_inventory_module(input: InventoryModuleInput<'_>) -> TokenStream {
    let InventoryModuleInput {
        ident,
        module_name_prefix,
        type_kind,
        variants,
        namespace_expr,
    } = input;

    let type_name = ident.to_string();
    let mod_name = format_ident!(
        "__es_fluent_{}_{}",
        module_name_prefix,
        type_name.to_snake_case()
    );

    quote! {
        #[doc(hidden)]
        mod #mod_name {
            use super::*;

            static VARIANTS: &[::es_fluent::registry::FtlVariant] = &[
                #(#variants),*
            ];

            static TYPE_INFO: ::es_fluent::registry::FtlTypeInfo =
                ::es_fluent::registry::FtlTypeInfo {
                    type_kind: #type_kind,
                    type_name: #type_name,
                    variants: VARIANTS,
                    file_path: file!(),
                    module_path: module_path!(),
                    namespace: #namespace_expr,
                };

            ::es_fluent::__inventory::submit!(::es_fluent::registry::RegisteredFtlType(&TYPE_INFO));
        }
    }
}

pub fn namespace_rule_tokens(namespace: Option<&NamespaceRule>) -> TokenStream {
    match namespace {
        Some(NamespaceRule::Literal(s)) => {
            quote! {
                Some(::es_fluent::registry::NamespaceRule::Literal(::std::borrow::Cow::Borrowed(#s)))
            }
        },
        Some(NamespaceRule::File) => {
            quote! { Some(::es_fluent::registry::NamespaceRule::File) }
        },
        Some(NamespaceRule::FileRelative) => {
            quote! { Some(::es_fluent::registry::NamespaceRule::FileRelative) }
        },
        Some(NamespaceRule::Folder) => {
            quote! { Some(::es_fluent::registry::NamespaceRule::Folder) }
        },
        Some(NamespaceRule::FolderRelative) => {
            quote! { Some(::es_fluent::registry::NamespaceRule::FolderRelative) }
        },
        None => quote! { None },
    }
}

pub fn inherited_fluent_namespace(
    input: &DeriveInput,
) -> Result<Option<NamespaceRule>, darling::Error> {
    match &input.data {
        Data::Struct(_) => {
            StructOpts::from_derive_input(input).map(|opts| opts.attr_args().namespace().cloned())
        },
        Data::Enum(_) => {
            EnumOpts::from_derive_input(input).map(|opts| opts.attr_args().namespace().cloned())
        },
        Data::Union(_) => panic!("namespace lookup is not supported for unions"),
    }
}

pub fn inherited_fluent_domain(input: &DeriveInput) -> Result<Option<String>, darling::Error> {
    match &input.data {
        Data::Struct(_) => Ok(None),
        Data::Enum(_) => EnumOpts::from_derive_input(input)
            .map(|opts| opts.attr_args().domain().map(str::to_owned)),
        Data::Union(_) => panic!("domain lookup is not supported for unions"),
    }
}

pub fn preferred_namespace<'a>(
    namespaces: impl IntoIterator<Item = Option<&'a NamespaceRule>>,
) -> Option<&'a NamespaceRule> {
    namespaces.into_iter().flatten().next()
}

#[cfg(test)]
mod tests {
    use super::{
        generate_this_ftl_impl, inherited_fluent_domain, inherited_fluent_namespace,
        preferred_namespace,
    };
    use crate::snapshot_support::pretty_file_tokens;
    use es_fluent_shared::namespace::NamespaceRule;
    use insta::assert_snapshot;
    use syn::parse_quote;

    #[test]
    fn inherited_fluent_namespace_reads_parent_attr_on_structs_and_enums() {
        let struct_input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "ui")]
            struct LoginForm;
        };
        let enum_input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "errors")]
            enum Problem {
                A
            }
        };

        assert!(matches!(
            inherited_fluent_namespace(&struct_input).expect("struct namespace"),
            Some(NamespaceRule::Literal(value)) if value == "ui"
        ));
        assert!(matches!(
            inherited_fluent_namespace(&enum_input).expect("enum namespace"),
            Some(NamespaceRule::Literal(value)) if value == "errors"
        ));
    }

    #[test]
    fn preferred_namespace_picks_the_first_available_namespace() {
        let parent = NamespaceRule::Literal("parent".into());
        let child = NamespaceRule::Literal("child".into());
        let fallback = NamespaceRule::Literal("fallback".into());

        assert_eq!(
            preferred_namespace([Some(&parent), Some(&child), Some(&fallback)]),
            Some(&parent)
        );
        assert_eq!(
            preferred_namespace([None, Some(&child), Some(&fallback)]),
            Some(&child)
        );
        assert_eq!(
            preferred_namespace([None, None, Some(&fallback)]),
            Some(&fallback)
        );
        assert_eq!(preferred_namespace([None, None, None]), None);
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn generate_this_ftl_impl_routes_through_the_current_crate_domain() {
        let tokens = pretty_file_tokens(generate_this_ftl_impl(
            &parse_quote!(Greeting),
            &parse_quote!(),
            Some("hello"),
            None,
        ));

        assert_snapshot!("generate_this_ftl_impl_current_crate_domain", tokens);
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn generate_this_ftl_impl_uses_explicit_domain_override_when_present() {
        let tokens = pretty_file_tokens(generate_this_ftl_impl(
            &parse_quote!(Languages),
            &parse_quote!(),
            Some("es-fluent-lang-this"),
            Some("es-fluent-lang"),
        ));

        assert_snapshot!("generate_this_ftl_impl_explicit_domain_override", tokens);
    }

    #[test]
    fn inherited_fluent_domain_reads_parent_attr_on_enums() {
        let enum_input: syn::DeriveInput = parse_quote! {
            #[fluent(domain = "es-fluent-lang")]
            enum Languages {
                En
            }
        };
        let struct_input: syn::DeriveInput = parse_quote! {
            struct LoginForm;
        };

        assert_eq!(
            inherited_fluent_domain(&enum_input).expect("enum domain"),
            Some("es-fluent-lang".to_string())
        );
        assert_eq!(
            inherited_fluent_domain(&struct_input).expect("struct domain"),
            None
        );
    }
}

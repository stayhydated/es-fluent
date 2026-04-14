use darling::FromDeriveInput as _;
use es_fluent_derive_core::options::namespace::NamespaceValue;
use es_fluent_derive_core::options::{
    FluentField, GeneratedVariantsOptions, r#enum::EnumOpts, r#struct::StructOpts,
};
use es_fluent_shared::namer;
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
) -> TokenStream {
    let Some(ftl_key) = ftl_key else {
        return quote! {};
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        impl #impl_generics ::es_fluent::ThisFtl for #ident #ty_generics #where_clause {
            fn this_ftl() -> String {
                ::es_fluent::localize(#ftl_key, None)
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

pub fn generate_from_impls(ident: &syn::Ident, generics: &syn::Generics) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics From<&#ident #ty_generics> for ::es_fluent::FluentValue<'_> #where_clause {
            fn from(value: &#ident #ty_generics) -> Self {
                use ::es_fluent::ToFluentString as _;
                value.to_fluent_string().into()
            }
        }

        impl #impl_generics From<#ident #ty_generics> for ::es_fluent::FluentValue<'_> #where_clause {
            fn from(value: #ident #ty_generics) -> Self {
                (&value).into()
            }
        }
    }
}

pub fn generate_fluent_display_impl(
    ident: &syn::Ident,
    generics: &syn::Generics,
    body: TokenStream,
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let trait_impl = quote! { ::es_fluent::FluentDisplay };
    let trait_fmt_fn_ident = quote! { fluent_fmt };

    quote! {
        impl #impl_generics #trait_impl for #ident #ty_generics #where_clause {
            fn #trait_fmt_fn_ident(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
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
        derives,
        variants,
        namespace_expr,
        this_key,
    } = input;

    let empty_generics = syn::Generics::default();
    let new_enum = generate_unit_enum_definition(ident, origin_ident, key_name, derives, variants);
    let match_arms = variants
        .iter()
        .map(GeneratedUnitEnumVariant::display_match_arm);
    let display_impl = generate_fluent_display_impl(
        ident,
        &empty_generics,
        quote! {
            match self {
                #(#match_arms),*
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
    let this_impl = generate_this_ftl_impl(ident, &empty_generics, this_key.as_deref());
    let this_inventory =
        generate_optional_this_inventory_module(ident, namespace_expr, this_key.as_deref());
    let from_impls = generate_from_impls(ident, &empty_generics);

    quote! {
        #new_enum

        #display_impl

        #inventory_submit

        #this_impl
        #this_inventory

        #from_impls
    }
}

pub fn emit_display_inventory_and_from_impls(
    ident: &syn::Ident,
    generics: &syn::Generics,
    display_body: TokenStream,
    inventory_submit: TokenStream,
) -> TokenStream {
    let display_impl = generate_fluent_display_impl(ident, generics, display_body);
    let from_impls = generate_from_impls(ident, generics);

    quote! {
        #display_impl

        #inventory_submit

        #from_impls
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

pub fn namespace_rule_tokens(namespace: Option<&NamespaceValue>) -> TokenStream {
    match namespace {
        Some(NamespaceValue::Literal(s)) => {
            quote! {
                Some(::es_fluent::registry::NamespaceRule::Literal(::std::borrow::Cow::Borrowed(#s)))
            }
        },
        Some(NamespaceValue::File) => {
            quote! { Some(::es_fluent::registry::NamespaceRule::File) }
        },
        Some(NamespaceValue::FileRelative) => {
            quote! { Some(::es_fluent::registry::NamespaceRule::FileRelative) }
        },
        Some(NamespaceValue::Folder) => {
            quote! { Some(::es_fluent::registry::NamespaceRule::Folder) }
        },
        Some(NamespaceValue::FolderRelative) => {
            quote! { Some(::es_fluent::registry::NamespaceRule::FolderRelative) }
        },
        None => quote! { None },
    }
}

pub fn inherited_fluent_namespace(
    input: &DeriveInput,
) -> Result<Option<NamespaceValue>, darling::Error> {
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

pub fn preferred_namespace<'a>(
    namespaces: impl IntoIterator<Item = Option<&'a NamespaceValue>>,
) -> Option<&'a NamespaceValue> {
    namespaces.into_iter().flatten().next()
}

#[cfg(test)]
mod tests {
    use super::{inherited_fluent_namespace, preferred_namespace};
    use es_fluent_derive_core::options::namespace::NamespaceValue;
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
            Some(NamespaceValue::Literal(value)) if value == "ui"
        ));
        assert!(matches!(
            inherited_fluent_namespace(&enum_input).expect("enum namespace"),
            Some(NamespaceValue::Literal(value)) if value == "errors"
        ));
    }

    #[test]
    fn preferred_namespace_picks_the_first_available_namespace() {
        let parent = NamespaceValue::Literal("parent".into());
        let child = NamespaceValue::Literal("child".into());
        let fallback = NamespaceValue::Literal("fallback".into());

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
}

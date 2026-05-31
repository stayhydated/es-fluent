use darling::FromDeriveInput as _;
use es_fluent_derive_core::error::{AttrContext, EsFluentCoreError};
use es_fluent_derive_core::options::{
    GeneratedVariantsOptions, r#enum::EnumOpts, r#struct::StructOpts,
};
use es_fluent_derive_core::semantic::{
    ArgumentModel, ArgumentValueStrategy, DerivePathList, DomainName, FluentMessageId,
    GeneratedEnumModel, GeneratedKeyIdent, GeneratedKeyName, InventoryPolicy, MessageModel,
    SpannedValue,
};
use es_fluent_shared::{namer, namespace::NamespaceRule};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{Data, DeriveInput};

use crate::macros::ir::inventory_variant_tokens_for_model;
use crate::macros::ir::{FluentArgument, GeneratedUnitEnumVariant, MessageEntrySpec};

pub use es_fluent_derive_core::validation::{
    NamespaceSource, SpannedNamespaceRuleRef, resolve_single_namespace_source,
};

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
    pub domain_override: Option<&'a DomainName>,
    pub derives: &'a [syn::Path],
    pub variants: &'a [GeneratedUnitEnumVariant],
    pub namespace: Option<&'a NamespaceRule>,
    pub label_key: Option<FluentMessageId>,
}

pub struct GeneratedVariantsEnumTarget<'a> {
    pub ident: syn::Ident,
    pub key_name: Option<&'a GeneratedKeyName>,
}

#[derive(Clone, Debug)]
pub struct SpannedNamespaceRule {
    rule: NamespaceRule,
    span: Span,
}

impl SpannedNamespaceRule {
    pub fn new(rule: NamespaceRule, span: Span) -> Self {
        Self { rule, span }
    }

    pub fn as_ref(&self) -> SpannedNamespaceRuleRef<'_> {
        SpannedNamespaceRuleRef::new(&self.rule, self.span)
    }
}

pub fn generated_variants_enum_targets<'a>(
    opts: &'a impl GeneratedVariantsOptions,
) -> Vec<GeneratedVariantsEnumTarget<'a>> {
    let Some(keys) = opts.variants_attr_args().keys() else {
        return vec![GeneratedVariantsEnumTarget {
            ident: opts.ftl_enum_ident(),
            key_name: None,
        }];
    };

    keys.iter()
        .map(|key| GeneratedVariantsEnumTarget {
            ident: GeneratedKeyIdent::variants(opts.variants_ident(), key, "Variants").into_ident(),
            key_name: Some(key.value()),
        })
        .collect()
}

/// Generates the `FluentLabel` trait implementation.
pub fn generate_localize_label_impl(
    ident: &syn::Ident,
    generics: &syn::Generics,
    ftl_key: Option<&FluentMessageId>,
    domain_override: Option<&DomainName>,
) -> TokenStream {
    let Some(ftl_key) = ftl_key else {
        return quote! {};
    };
    let ftl_key = ftl_key.as_str();

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let domain_expr = match domain_override {
        Some(domain) => {
            let domain = domain.as_str();
            quote! { #domain }
        },
        None => quote! { env!("CARGO_PKG_NAME") },
    };
    quote! {
        impl #impl_generics ::es_fluent::FluentLabel for #ident #ty_generics #where_clause {
            fn localize_label<__EsFluentLocalizer: ::es_fluent::FluentLocalizer + ?Sized>(
                localizer: &__EsFluentLocalizer,
            ) -> String {
                ::es_fluent::__private::localize_label(localizer, #domain_expr, #ftl_key)
            }
        }
    }
}

pub fn generate_field_value_expr(
    value_strategy: &ArgumentValueStrategy,
    access_expr: TokenStream,
    transform_arg_expr: TokenStream,
) -> TokenStream {
    match value_strategy {
        ArgumentValueStrategy::Transform(transform) => {
            let expr = transform.expr();
            quote! {
                ::es_fluent::__private::FluentArgumentValue::new((#expr)(#transform_arg_expr))
            }
        },
        ArgumentValueStrategy::Choice => {
            quote! {
                ::es_fluent::__private::FluentArgumentValue::new({
                    use ::es_fluent::EsFluentChoice as _;
                    (#access_expr).as_fluent_choice()
                })
            }
        },
        ArgumentValueStrategy::Optional => {
            quote! {
                ::es_fluent::__private::FluentOptionalArgumentValue::new((#transform_arg_expr).as_ref())
            }
        },
        ArgumentValueStrategy::Borrowed => {
            quote! {
                ::es_fluent::__private::FluentBorrowedArgumentValue::new(#transform_arg_expr)
            }
        },
    }
}

pub fn generate_field_argument(
    metadata: ArgumentModel,
    access_expr: TokenStream,
    transform_arg_expr: TokenStream,
) -> FluentArgument {
    let value_expr =
        generate_field_value_expr(metadata.value_strategy(), access_expr, transform_arg_expr);

    FluentArgument {
        metadata,
        value_expr,
    }
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
    model: &GeneratedEnumModel,
    variants: &[GeneratedUnitEnumVariant],
) -> TokenStream {
    let cleaned_variants = variants.iter().map(|entry| &entry.ident);
    let derives = model.derives().paths().iter().map(|derive| derive.path());
    let derive_attr = if !model.derives().is_empty() {
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

pub fn generate_optional_label_inventory_module(
    ident: &syn::Ident,
    source_span: proc_macro2::Span,
    namespace: Option<&NamespaceRule>,
    label_key: Option<&FluentMessageId>,
) -> TokenStream {
    let Some(label_key) = label_key else {
        return quote! {};
    };

    let label_entry = MessageEntrySpec::new(
        namer::rust_ident_name(ident),
        SpannedValue::new(label_key.clone(), source_span),
        Vec::new(),
    );
    let label_variant = inventory_variant_tokens_for_model(&label_entry.metadata);
    let label_model = MessageModel::new(
        namer::rust_ident_name(ident),
        es_fluent_shared::meta::TypeKind::Enum,
        None,
        namespace.cloned(),
        Vec::new(),
        Some(label_entry.metadata),
        InventoryPolicy::Emit,
    );

    generate_inventory_module(InventoryModuleInput {
        ident,
        module_name_prefix: "label_inventory",
        type_kind: quote! { ::es_fluent::meta::TypeKind::Enum },
        variants: vec![label_variant],
        namespace_expr: namespace_rule_tokens(label_model.namespace()),
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
        namespace,
        label_key,
    } = input;

    let empty_generics = syn::Generics::default();
    let label_model = label_key.as_ref().map(|label_key| {
        MessageEntrySpec::new(
            namer::rust_ident_name(ident),
            SpannedValue::new(label_key.clone(), origin_ident.span()),
            Vec::new(),
        )
        .metadata
    });
    let derive_paths =
        match DerivePathList::from_paths(derives.iter().cloned(), AttrContext::VariantsContainer) {
            Ok(derive_paths) => derive_paths,
            Err(error) => return core_error_to_compile_error(error),
        };
    let generated_model = GeneratedEnumModel::new(
        ident.to_string(),
        origin_ident.to_string(),
        derive_paths,
        variants
            .iter()
            .map(|variant| variant.message_entry.metadata.clone())
            .collect(),
        label_model,
        domain_override.cloned(),
        namespace.cloned(),
    );
    let new_enum =
        generate_unit_enum_definition(ident, origin_ident, key_name, &generated_model, variants);
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
        variants: generated_model
            .messages()
            .iter()
            .map(inventory_variant_tokens_for_model)
            .collect(),
        namespace_expr: namespace_rule_tokens(generated_model.namespace()),
    });
    let label_impl =
        generate_localize_label_impl(ident, &empty_generics, label_key.as_ref(), domain_override);
    let label_inventory = generate_optional_label_inventory_module(
        ident,
        origin_ident.span(),
        namespace,
        label_key.as_ref(),
    );

    quote! {
        #new_enum

        #message_impl

        #inventory_submit

        #label_impl
        #label_inventory
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

    let type_name = namer::rust_ident_name(ident);
    let mod_name = format_ident!("__es_fluent_{}_{}", module_name_prefix, type_name);

    quote! {
        #[doc(hidden)]
        #[allow(non_snake_case)]
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

#[cfg(test)]
pub fn inherited_fluent_namespace(
    input: &DeriveInput,
) -> Result<Option<NamespaceRule>, darling::Error> {
    inherited_fluent_namespace_with_span(input)
        .map(|namespace| namespace.map(|namespace| namespace.rule.clone()))
}

pub fn inherited_fluent_namespace_with_span(
    input: &DeriveInput,
) -> Result<Option<SpannedNamespaceRule>, darling::Error> {
    match &input.data {
        Data::Struct(_) => StructOpts::from_derive_input(input).map(|opts| {
            opts.attr_args().namespace().map(|namespace| {
                SpannedNamespaceRule::new(
                    namespace.clone(),
                    opts.attr_args()
                        .namespace_span()
                        .unwrap_or_else(|| input.ident.span()),
                )
            })
        }),
        Data::Enum(_) => EnumOpts::from_derive_input(input).map(|opts| {
            opts.attr_args().namespace().map(|namespace| {
                SpannedNamespaceRule::new(
                    namespace.clone(),
                    opts.attr_args()
                        .namespace_span()
                        .unwrap_or_else(|| input.ident.span()),
                )
            })
        }),
        Data::Union(_) => Err(darling::Error::custom(
            "namespace lookup is not supported for unions",
        )),
    }
}

pub fn inherited_fluent_domain(input: &DeriveInput) -> Result<Option<DomainName>, darling::Error> {
    match &input.data {
        Data::Struct(_) => Ok(None),
        Data::Enum(_) => EnumOpts::from_derive_input(input).map(|opts| {
            opts.attr_args()
                .domain_name()
                .map(|domain| domain.value().clone())
        }),
        Data::Union(_) => Err(darling::Error::custom(
            "domain lookup is not supported for unions",
        )),
    }
}

pub fn core_error_to_compile_error(error: EsFluentCoreError) -> TokenStream {
    let message = error.to_string();
    match error.span() {
        Some(span) => quote_spanned! { span=> compile_error!(#message); },
        None => quote! { compile_error!(#message); },
    }
}

#[cfg(test)]
mod tests {
    use es_fluent_derive_core::error::AttrContext;
    use es_fluent_derive_core::semantic::{
        ArgumentValueStrategy, DerivePathList, GeneratedEnumModel, ValueTransform,
    };
    use es_fluent_shared::namespace::NamespaceRule;
    use insta::assert_snapshot;
    use quote::quote;
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
            super::inherited_fluent_namespace(&struct_input).expect("struct namespace"),
            Some(NamespaceRule::Literal(value)) if value == "ui"
        ));
        assert!(matches!(
            super::inherited_fluent_namespace(&enum_input).expect("enum namespace"),
            Some(NamespaceRule::Literal(value)) if value == "errors"
        ));
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn generate_localize_label_impl_routes_through_the_current_crate_domain() {
        let message_id = es_fluent_derive_core::semantic::spanned_message_id_from_value(
            "hello",
            proc_macro2::Span::call_site(),
            AttrContext::LabelContainer,
        )
        .expect("message id")
        .into_value();
        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::generate_localize_label_impl(
                &parse_quote!(Greeting),
                &parse_quote!(),
                Some(&message_id),
                None,
            ));

        assert_snapshot!("generate_localize_label_impl_current_crate_domain", tokens);
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn generate_localize_label_impl_uses_explicit_domain_override_when_present() {
        let message_id = es_fluent_derive_core::semantic::spanned_message_id_from_value(
            "es-fluent-lang-label",
            proc_macro2::Span::call_site(),
            AttrContext::LabelContainer,
        )
        .expect("message id")
        .into_value();
        let domain = es_fluent_derive_core::semantic::parse_domain_name_in_context(
            "es-fluent-lang",
            proc_macro2::Span::call_site(),
            AttrContext::LabelContainer,
        )
        .expect("domain");
        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::generate_localize_label_impl(
                &parse_quote!(Languages),
                &parse_quote!(),
                Some(&message_id),
                Some(&domain),
            ));

        assert_snapshot!(
            "generate_localize_label_impl_explicit_domain_override",
            tokens
        );
    }

    #[test]
    fn generate_unit_enum_definition_uses_model_derive_paths() {
        let model = GeneratedEnumModel::new(
            "StatusFtl",
            "Status",
            DerivePathList::from_paths(
                vec![parse_quote!(Debug), parse_quote!(Clone)],
                AttrContext::VariantsContainer,
            )
            .expect("derive paths"),
            Vec::new(),
            None,
            None,
            None,
        );

        let tokens = super::generate_unit_enum_definition(
            &parse_quote!(StatusFtl),
            &parse_quote!(Status),
            None,
            &model,
            &[],
        );
        let file: syn::File = syn::parse2(tokens).expect("generated enum should parse");
        let enum_item = file
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Enum(item) => Some(item),
                _ => None,
            })
            .expect("generated enum item");
        let derive_attr = enum_item
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("derive"))
            .expect("derive attr");
        let derives = derive_attr
            .parse_args_with(
                syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
            )
            .expect("derive paths")
            .iter()
            .map(|path| quote!(#path).to_string())
            .collect::<Vec<_>>();

        assert_eq!(derives, vec!["Debug", "Clone"]);
    }

    #[test]
    fn generate_field_value_expr_uses_argument_value_strategy() {
        let borrowed = super::generate_field_value_expr(
            &ArgumentValueStrategy::Borrowed,
            quote!(field),
            quote!(field),
        )
        .to_string();
        assert!(borrowed.contains("FluentBorrowedArgumentValue"));

        let optional = super::generate_field_value_expr(
            &ArgumentValueStrategy::Optional,
            quote!(field),
            quote!(field),
        )
        .to_string();
        assert!(optional.contains("FluentOptionalArgumentValue"));

        let choice = super::generate_field_value_expr(
            &ArgumentValueStrategy::Choice,
            quote!(field),
            quote!(field),
        )
        .to_string();
        assert!(choice.contains("as_fluent_choice"));

        let transform = super::generate_field_value_expr(
            &ArgumentValueStrategy::Transform(ValueTransform::new(
                parse_quote!(|value: &String| value.len()),
                proc_macro2::Span::call_site(),
            )),
            quote!(field),
            quote!(field),
        )
        .to_string();
        assert!(transform.contains("value . len"));
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
            super::inherited_fluent_domain(&enum_input).expect("enum domain"),
            Some(
                es_fluent_derive_core::semantic::parse_domain_name_in_context(
                    "es-fluent-lang",
                    proc_macro2::Span::call_site(),
                    AttrContext::MessageContainer,
                )
                .expect("domain")
            )
        );
        assert_eq!(
            super::inherited_fluent_domain(&struct_input).expect("struct domain"),
            None
        );
    }
}

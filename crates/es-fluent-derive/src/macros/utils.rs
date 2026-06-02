use es_fluent_derive_core::macro_support::{self, ResolvedCratePath};
use es_fluent_derive_core::semantic::{
    ArgName, ArgumentModel, ArgumentValueStrategy, DomainName, FluentMessageId, GeneratedEnumModel,
    GeneratedKeyName, MessageEntryModel, MessageModel,
};
use es_fluent_shared::meta::TypeKind;
use es_fluent_shared::{namer, namespace::NamespaceRule};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};

use crate::macros::ir::inventory_variant_tokens_for_model;
use crate::macros::ir::{FluentArgument, GeneratedUnitEnumVariant};

#[derive(Clone)]
pub struct CodegenContext {
    facade_path: ResolvedCratePath,
}

impl CodegenContext {
    pub fn resolve() -> Self {
        Self {
            facade_path: ResolvedCratePath::resolve("es-fluent", "es_fluent"),
        }
    }

    #[cfg(test)]
    pub fn fallback() -> Self {
        Self {
            facade_path: ResolvedCratePath::fallback("es_fluent"),
        }
    }

    pub fn facade_path(&self) -> &ResolvedCratePath {
        &self.facade_path
    }
}

pub struct InventoryModuleInput<'a> {
    pub ident: &'a syn::Ident,
    pub module_name_prefix: &'a str,
    pub type_kind: TypeKind,
    pub entries: Vec<MessageEntryModel>,
    pub namespace: Option<NamespaceRule>,
}

pub enum InventoryOutput<'a> {
    None,
    MessageEntries(InventoryModuleInput<'a>),
    LabelEntry(InventoryModuleInput<'a>),
    GeneratedEnum {
        messages: InventoryModuleInput<'a>,
        label: Option<InventoryModuleInput<'a>>,
    },
}

pub struct GeneratedUnitEnumInput<'a> {
    pub ident: &'a syn::Ident,
    pub origin_ident: &'a syn::Ident,
    pub key_name: Option<&'a GeneratedKeyName>,
    pub model: &'a GeneratedEnumModel,
    pub variants: &'a [GeneratedUnitEnumVariant],
}

/// Generates the `FluentLabel` trait implementation.
pub fn generate_localize_label_impl(
    context: &CodegenContext,
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
    let es_fluent = context.facade_path().tokens();
    let domain_expr = static_domain_tokens(context, domain_override);
    quote! {
        impl #impl_generics #es_fluent::FluentLabel for #ident #ty_generics #where_clause {
            fn localize_label<__EsFluentLocalizer: #es_fluent::FluentLocalizer + ?Sized>(
                localizer: &__EsFluentLocalizer,
            ) -> String {
                #es_fluent::__private::localize_label(localizer, (#domain_expr).as_str(), #ftl_key)
            }
        }
    }
}

pub(crate) fn static_domain_tokens(
    context: &CodegenContext,
    domain_override: Option<&DomainName>,
) -> TokenStream {
    let es_fluent = context.facade_path().tokens();
    es_fluent_derive_core::macro_support::static_domain_tokens(es_fluent, domain_override)
}

pub(crate) fn static_entry_id_tokens(
    context: &CodegenContext,
    entry_id: &FluentMessageId,
) -> TokenStream {
    let es_fluent = context.facade_path().tokens();
    es_fluent_derive_core::macro_support::static_entry_id_tokens(es_fluent, entry_id)
}

pub(crate) fn static_argument_name_tokens(
    context: &CodegenContext,
    argument_name: &ArgName,
) -> TokenStream {
    let es_fluent = context.facade_path().tokens();
    es_fluent_derive_core::macro_support::static_argument_name_tokens(es_fluent, argument_name)
}

pub fn generate_field_value_expr(
    context: &CodegenContext,
    value_strategy: &ArgumentValueStrategy,
    access_expr: TokenStream,
    transform_arg_expr: TokenStream,
) -> TokenStream {
    let es_fluent = context.facade_path().tokens();
    match value_strategy {
        ArgumentValueStrategy::Transform(transform) => {
            let expr = transform.expr();
            let span = transform.span();
            quote_spanned! { span=>
                #es_fluent::__private::FluentArgumentValue::new((#expr)(#transform_arg_expr))
            }
        },
        ArgumentValueStrategy::Choice { span } => {
            quote_spanned! { *span=>
                #es_fluent::__private::FluentArgumentValue::new({
                    use #es_fluent::EsFluentChoice as _;
                    (#access_expr).as_fluent_choice()
                })
            }
        },
        ArgumentValueStrategy::Optional { span } => {
            quote_spanned! { *span=>
                #es_fluent::__private::FluentOptionalArgumentValue::new((#transform_arg_expr).as_ref())
            }
        },
        ArgumentValueStrategy::Borrowed { span } => {
            quote_spanned! { *span=>
                #es_fluent::__private::FluentBorrowedArgumentValue::new(#transform_arg_expr)
            }
        },
    }
}

pub fn generate_field_argument(
    context: &CodegenContext,
    metadata: ArgumentModel,
    access_expr: TokenStream,
    transform_arg_expr: TokenStream,
) -> FluentArgument {
    let value_expr = generate_field_value_expr(
        context,
        metadata.value_strategy(),
        access_expr,
        transform_arg_expr,
    );

    FluentArgument {
        metadata,
        value_expr,
    }
}

pub fn generate_fluent_message_impl(
    context: &CodegenContext,
    ident: &syn::Ident,
    generics: &syn::Generics,
    body: TokenStream,
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let es_fluent = context.facade_path().tokens();

    quote! {
        impl #impl_generics #es_fluent::FluentMessage for #ident #ty_generics #where_clause {
            fn to_fluent_string_with(
                &self,
                localize: &mut dyn for<'__es_fluent_message> FnMut(
                    #es_fluent::registry::StaticFluentDomain,
                    #es_fluent::registry::StaticFluentEntryId,
                    Option<&#es_fluent::FluentArgs<'__es_fluent_message>>,
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
    key_name: Option<&GeneratedKeyName>,
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
        Some(key) => format!("`{}` variants of [`{origin_ident}`].", key.as_str()),
        None => format!("Variants of [`{origin_ident}`]."),
    };
    let variant_docs: Vec<_> = variants
        .iter()
        .map(|entry| match key_name {
            Some(key) => format!(
                "The `{}` `{}` variant of [`{origin_ident}`].",
                entry.doc_name.as_str(),
                key.as_str()
            ),
            None => format!(
                "The `{}` variant of [`{origin_ident}`].",
                entry.doc_name.as_str()
            ),
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

pub fn emit_generated_unit_enum(
    context: &CodegenContext,
    input: GeneratedUnitEnumInput<'_>,
) -> TokenStream {
    let GeneratedUnitEnumInput {
        ident,
        origin_ident,
        key_name,
        model,
        variants,
    } = input;

    let empty_generics = syn::Generics::default();
    let domain_override = model.domain();
    let label_key = model.label().map(|label| label.message_id().clone());
    let new_enum = generate_unit_enum_definition(ident, origin_ident, key_name, model, variants);
    let localize_with_match_arms = variants
        .iter()
        .map(|variant| variant.localize_with_match_arm(context, domain_override));
    let message_impl = generate_fluent_message_impl(
        context,
        ident,
        &empty_generics,
        quote! {
            match self {
                #(#localize_with_match_arms),*
            }
        },
    );
    let inventory_output = InventoryOutput::GeneratedEnum {
        messages: InventoryModuleInput {
            ident,
            module_name_prefix: "inventory",
            type_kind: TypeKind::Enum,
            entries: model.messages().to_vec(),
            namespace: model.namespace().cloned(),
        },
        label: model
            .label()
            .cloned()
            .map(|label_entry| InventoryModuleInput {
                ident,
                module_name_prefix: "label_inventory",
                type_kind: TypeKind::Enum,
                entries: vec![label_entry],
                namespace: model.namespace().cloned(),
            }),
    };
    let inventory_submit = emit_inventory_output(context, inventory_output);
    let label_impl = generate_localize_label_impl(
        context,
        ident,
        &empty_generics,
        label_key.as_ref(),
        domain_override,
    );
    quote! {
        #new_enum

        #message_impl

        #inventory_submit

        #label_impl
    }
}

pub fn emit_message_inventory_impls(
    context: &CodegenContext,
    ident: &syn::Ident,
    generics: &syn::Generics,
    fluent_message_body: TokenStream,
    inventory_output: InventoryOutput<'_>,
) -> TokenStream {
    let message_impl = generate_fluent_message_impl(context, ident, generics, fluent_message_body);
    let inventory_submit = emit_inventory_output(context, inventory_output);

    quote! {
        #message_impl

        #inventory_submit
    }
}

pub fn message_inventory_output<'a>(
    ident: &'a syn::Ident,
    module_name_prefix: &'a str,
    model: &MessageModel,
) -> InventoryOutput<'a> {
    InventoryOutput::MessageEntries(InventoryModuleInput {
        ident,
        module_name_prefix,
        type_kind: model.type_kind().clone(),
        entries: model.messages().to_vec(),
        namespace: model.namespace().cloned(),
    })
}

pub fn label_inventory_output<'a>(
    ident: &'a syn::Ident,
    type_kind: TypeKind,
    namespace: Option<NamespaceRule>,
    label_entry: MessageEntryModel,
) -> InventoryOutput<'a> {
    InventoryOutput::LabelEntry(InventoryModuleInput {
        ident,
        module_name_prefix: "label_inventory",
        type_kind,
        entries: vec![label_entry],
        namespace,
    })
}

pub fn emit_inventory_output(context: &CodegenContext, output: InventoryOutput<'_>) -> TokenStream {
    match output {
        InventoryOutput::None => quote! {},
        InventoryOutput::MessageEntries(input) | InventoryOutput::LabelEntry(input) => {
            generate_inventory_module(context, input)
        },
        InventoryOutput::GeneratedEnum { messages, label } => {
            let message_inventory = generate_inventory_module(context, messages);
            let label_inventory = label
                .map(|input| generate_inventory_module(context, input))
                .unwrap_or_else(|| quote! {});

            quote! {
                #message_inventory
                #label_inventory
            }
        },
    }
}

fn type_kind_tokens(context: &CodegenContext, type_kind: &TypeKind) -> TokenStream {
    let es_fluent = context.facade_path().tokens();
    match type_kind {
        TypeKind::Enum => quote! { #es_fluent::meta::TypeKind::Enum },
        TypeKind::Struct => quote! { #es_fluent::meta::TypeKind::Struct },
    }
}

fn generate_inventory_module(
    context: &CodegenContext,
    input: InventoryModuleInput<'_>,
) -> TokenStream {
    let InventoryModuleInput {
        ident,
        module_name_prefix,
        type_kind,
        entries,
        namespace,
    } = input;

    let type_name = namer::rust_ident_name(ident);
    let mod_name = format_ident!("__es_fluent_{}_{}", module_name_prefix, type_name);
    let es_fluent = context.facade_path().tokens();
    let type_kind = type_kind_tokens(context, &type_kind);
    let variants: Vec<_> = entries
        .iter()
        .map(|metadata| inventory_variant_tokens_for_model(context, metadata))
        .collect();
    let namespace_expr = namespace_rule_tokens(context, namespace.as_ref());

    quote! {
        #[doc(hidden)]
        #[allow(non_snake_case)]
        mod #mod_name {
            use super::*;

            static VARIANTS: &[#es_fluent::registry::FtlVariant] = &[
                #(#variants),*
            ];

            static TYPE_INFO: #es_fluent::registry::FtlTypeInfo =
                #es_fluent::registry::__macro::ftl_type_info(
                    #type_kind,
                    #type_name,
                    VARIANTS,
                    file!(),
                    module_path!(),
                    #namespace_expr,
                );

            #es_fluent::__inventory::submit!(#es_fluent::registry::RegisteredFtlType(&TYPE_INFO));
        }
    }
}

pub fn namespace_rule_tokens(
    context: &CodegenContext,
    namespace: Option<&NamespaceRule>,
) -> TokenStream {
    let es_fluent = context.facade_path().tokens();
    match namespace {
        Some(NamespaceRule::Literal(s)) => {
            quote! {
                Some(#es_fluent::registry::NamespaceRule::Literal(::std::borrow::Cow::Borrowed(#s)))
            }
        },
        Some(NamespaceRule::File) => {
            quote! { Some(#es_fluent::registry::NamespaceRule::File) }
        },
        Some(NamespaceRule::FileRelative) => {
            quote! { Some(#es_fluent::registry::NamespaceRule::FileRelative) }
        },
        Some(NamespaceRule::Folder) => {
            quote! { Some(#es_fluent::registry::NamespaceRule::Folder) }
        },
        Some(NamespaceRule::FolderRelative) => {
            quote! { Some(#es_fluent::registry::NamespaceRule::FolderRelative) }
        },
        None => quote! { None },
    }
}

pub fn core_error_to_compile_error(
    error: es_fluent_derive_core::error::EsFluentCoreError,
) -> TokenStream {
    macro_support::core_error_to_compile_error(error)
}

#[cfg(test)]
mod tests {
    use es_fluent_derive_core::error::AttrContext;
    use es_fluent_derive_core::semantic::{
        ArgumentValueStrategy, DerivePathList, GeneratedEnumModel, RustTypeName, ValueTransform,
    };
    use insta::assert_snapshot;
    use quote::quote;
    use syn::parse_quote;

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
        let context = super::CodegenContext::fallback();
        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::generate_localize_label_impl(
                &context,
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
        let context = super::CodegenContext::fallback();
        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::generate_localize_label_impl(
                &context,
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
            RustTypeName::new("StatusFtl", proc_macro2::Span::call_site()),
            RustTypeName::new("Status", proc_macro2::Span::call_site()),
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
        let context = super::CodegenContext::fallback();
        let borrowed = super::generate_field_value_expr(
            &context,
            &ArgumentValueStrategy::Borrowed {
                span: proc_macro2::Span::call_site(),
            },
            quote!(field),
            quote!(field),
        )
        .to_string();
        assert!(borrowed.contains("FluentBorrowedArgumentValue"));

        let optional = super::generate_field_value_expr(
            &context,
            &ArgumentValueStrategy::Optional {
                span: proc_macro2::Span::call_site(),
            },
            quote!(field),
            quote!(field),
        )
        .to_string();
        assert!(optional.contains("FluentOptionalArgumentValue"));

        let choice = super::generate_field_value_expr(
            &context,
            &ArgumentValueStrategy::Choice {
                span: proc_macro2::Span::call_site(),
            },
            quote!(field),
            quote!(field),
        )
        .to_string();
        assert!(choice.contains("as_fluent_choice"));

        let transform = super::generate_field_value_expr(
            &context,
            &ArgumentValueStrategy::Transform(Box::new(ValueTransform::new(
                parse_quote!(|value: &String| value.len()),
                proc_macro2::Span::call_site(),
            ))),
            quote!(field),
            quote!(field),
        )
        .to_string();
        assert!(transform.contains("value . len"));
    }
}

use es_fluent_derive_core::error::{AttrContext, EsFluentCoreError};
use es_fluent_derive_core::options::GeneratedVariantsOptions;
use es_fluent_derive_core::semantic::{
    ArgumentModel, ArgumentValueStrategy, DerivePathList, DomainName, FluentMessageId,
    GeneratedEnumModel, GeneratedKeyIdent, GeneratedKeyName, MessageEntryModel, MessageModel,
    RustSourceName, RustTypeName, SpannedValue,
};
use es_fluent_shared::meta::TypeKind;
use es_fluent_shared::{namer, namespace::NamespaceRule};
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};

use crate::macros::ir::inventory_variant_tokens_for_model;
use crate::macros::ir::{FluentArgument, GeneratedUnitEnumVariant, MessageEntrySpec};

pub use es_fluent_derive_core::validation::{
    NamespaceSource, SpannedNamespaceRuleRef, resolve_single_namespace_source,
};

#[derive(Clone)]
pub struct CodegenContext {
    facade_path: FacadePath,
}

impl CodegenContext {
    pub fn resolve() -> Self {
        Self {
            facade_path: FacadePath::resolve(),
        }
    }

    #[cfg(test)]
    pub fn fallback() -> Self {
        Self {
            facade_path: FacadePath::fallback(),
        }
    }

    pub fn facade_path(&self) -> &FacadePath {
        &self.facade_path
    }
}

#[derive(Clone)]
pub struct FacadePath {
    tokens: TokenStream,
}

impl FacadePath {
    fn resolve() -> Self {
        match crate_name("es-fluent") {
            Ok(FoundCrate::Itself) => Self {
                tokens: quote! { crate },
            },
            Ok(FoundCrate::Name(name)) => {
                let ident = format_ident!("{name}");
                Self {
                    tokens: quote! { ::#ident },
                }
            },
            Err(_) => Self::fallback(),
        }
    }

    fn fallback() -> Self {
        Self {
            tokens: quote! { ::es_fluent },
        }
    }

    pub fn tokens(&self) -> &TokenStream {
        &self.tokens
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
    let domain_expr = match domain_override {
        Some(domain) => {
            let domain = domain.as_str();
            quote! { #domain }
        },
        None => quote! { env!("CARGO_PKG_NAME") },
    };
    quote! {
        impl #impl_generics #es_fluent::FluentLabel for #ident #ty_generics #where_clause {
            fn localize_label<__EsFluentLocalizer: #es_fluent::FluentLocalizer + ?Sized>(
                localizer: &__EsFluentLocalizer,
            ) -> String {
                #es_fluent::__private::localize_label(localizer, #domain_expr, #ftl_key)
            }
        }
    }
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

pub fn emit_generated_unit_enum(
    context: &CodegenContext,
    input: GeneratedUnitEnumInput<'_>,
) -> TokenStream {
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
            RustSourceName::from_ident(ident),
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
        RustTypeName::from_ident(ident),
        RustTypeName::from_ident(origin_ident),
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
            entries: generated_model.messages().to_vec(),
            namespace: generated_model.namespace().cloned(),
        },
        label: generated_model
            .label()
            .cloned()
            .map(|label_entry| InventoryModuleInput {
                ident,
                module_name_prefix: "label_inventory",
                type_kind: TypeKind::Enum,
                entries: vec![label_entry],
                namespace: generated_model.namespace().cloned(),
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
                #es_fluent::registry::FtlTypeInfo {
                    type_kind: #type_kind,
                    type_name: #type_name,
                    variants: VARIANTS,
                    file_path: file!(),
                    module_path: module_path!(),
                    namespace: #namespace_expr,
                };

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
            super::RustTypeName::new("StatusFtl", proc_macro2::Span::call_site()),
            super::RustTypeName::new("Status", proc_macro2::Span::call_site()),
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
}

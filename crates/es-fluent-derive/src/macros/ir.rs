use es_fluent_derive_core::semantic::{
    ArgName, ArgumentModel, DomainName, FluentMessageId, MessageEntryModel, RustSourceName,
    SourceLocation, SpannedValue,
};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::Ident;

use crate::macros::utils::CodegenContext;

#[derive(Clone)]
pub(crate) struct FluentArgument {
    pub(crate) metadata: ArgumentModel,
    pub(crate) value_expr: TokenStream,
}

impl FluentArgument {
    pub(crate) fn name(&self) -> &str {
        self.metadata.name().as_str()
    }

    fn context_bound_insert_statement(&self, context: &CodegenContext) -> TokenStream {
        let key = self.name();
        let value_expr = &self.value_expr;
        let es_fluent = context.facade_path().tokens();
        quote! {
            {
                use #es_fluent::__private::IntoFluentArgumentValue as _;
                args.insert(
                    #es_fluent::registry::StaticFluentArgumentName::new_unchecked(#key),
                    (#value_expr).into_fluent_argument_value(localize),
                );
            }
        }
    }
}

pub(crate) struct MessageEntrySpec {
    pub(crate) metadata: MessageEntryModel,
    pub(crate) runtime_arguments: Vec<FluentArgument>,
}

impl MessageEntrySpec {
    pub(crate) fn new(
        source_name: RustSourceName,
        message_id: SpannedValue<FluentMessageId>,
        runtime_arguments: Vec<FluentArgument>,
    ) -> Self {
        let arguments = runtime_arguments
            .iter()
            .map(|argument| argument.metadata.clone())
            .collect();
        let source_location = SourceLocation::new(message_id.span());

        Self {
            metadata: MessageEntryModel::new(source_name, message_id, arguments, source_location),
            runtime_arguments,
        }
    }

    pub(crate) fn from_metadata(
        metadata: MessageEntryModel,
        runtime_arguments: Vec<FluentArgument>,
    ) -> Self {
        Self {
            metadata,
            runtime_arguments,
        }
    }

    pub(crate) fn localize_with_expr(
        &self,
        context: &CodegenContext,
        domain_override: Option<&DomainName>,
    ) -> TokenStream {
        LocalizeCallSpec {
            domain_override: domain_override.cloned(),
            ftl_key: self.metadata.message_id().clone(),
            arguments: self.runtime_arguments.clone(),
        }
        .localize_with_expr(context)
    }
}

pub(crate) fn inventory_variant_tokens_for_model(
    context: &CodegenContext,
    metadata: &MessageEntryModel,
) -> TokenStream {
    InventoryVariantSpec {
        name: metadata.source_name().to_string(),
        ftl_key: metadata.message_id().clone(),
        arg_names: metadata.argument_names(),
        source_location: metadata.source_location().clone(),
    }
    .tokens(context)
}

pub(crate) struct LocalizeCallSpec {
    pub(crate) domain_override: Option<DomainName>,
    pub(crate) ftl_key: FluentMessageId,
    pub(crate) arguments: Vec<FluentArgument>,
}

impl LocalizeCallSpec {
    pub(crate) fn localize_with_expr(&self, context: &CodegenContext) -> TokenStream {
        let es_fluent = context.facade_path().tokens();
        let domain_expr = match self.domain_override.as_ref() {
            Some(domain) => {
                let domain = domain.as_str();
                quote! { #es_fluent::registry::StaticFluentDomain::new_unchecked(#domain) }
            },
            None => {
                quote! { #es_fluent::registry::StaticFluentDomain::new_unchecked(env!("CARGO_PKG_NAME")) }
            },
        };
        let ftl_key = self.ftl_key.as_str();
        let ftl_key_expr = quote! {
            #es_fluent::registry::StaticFluentEntryId::new_unchecked(#ftl_key)
        };

        if self.arguments.is_empty() {
            quote! {
                localize(#domain_expr, #ftl_key_expr, None)
            }
        } else {
            let inserts: Vec<_> = self
                .arguments
                .iter()
                .map(|argument| argument.context_bound_insert_statement(context))
                .collect();

            quote! {
                {
                    let mut args = #es_fluent::FluentArgs::new();
                    #(#inserts)*
                    localize(#domain_expr, #ftl_key_expr, Some(&args))
                }
            }
        }
    }
}

pub(crate) struct InventoryVariantSpec {
    pub(crate) name: String,
    pub(crate) ftl_key: FluentMessageId,
    pub(crate) arg_names: Vec<ArgName>,
    pub(crate) source_location: SourceLocation,
}

impl InventoryVariantSpec {
    pub(crate) fn tokens(&self, context: &CodegenContext) -> TokenStream {
        let name = &self.name;
        let ftl_key = self.ftl_key.as_str();
        let es_fluent = context.facade_path().tokens();
        let args_tokens: Vec<_> = self
            .arg_names
            .iter()
            .map(|arg| {
                let arg = arg.as_str();
                quote! { #es_fluent::registry::StaticFluentArgumentName::new_unchecked(#arg) }
            })
            .collect();
        let source_span = self.source_location.span();
        let source_line = quote_spanned! { source_span=> line!() };

        quote! {
            #es_fluent::registry::FtlVariant {
                name: #name,
                ftl_key: #es_fluent::registry::StaticFluentEntryId::new_unchecked(#ftl_key),
                args: &[#(#args_tokens),*],
                module_path: module_path!(),
                line: #source_line,
            }
        }
    }
}

pub(crate) struct GeneratedUnitEnumVariant {
    pub(crate) ident: Ident,
    pub(crate) doc_name: String,
    pub(crate) message_entry: MessageEntrySpec,
}

impl GeneratedUnitEnumVariant {
    pub(crate) fn localize_with_match_arm(
        &self,
        context: &CodegenContext,
        domain_override: Option<&DomainName>,
    ) -> TokenStream {
        let variant_ident = &self.ident;
        let body = self
            .message_entry
            .localize_with_expr(context, domain_override);

        quote! {
            Self::#variant_ident => #body
        }
    }
}

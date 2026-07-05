use es_fluent_derive_core::semantic::{
    ArgName, ArgumentModel, DomainName, FluentMessageId, GeneratedDocName, MessageEntryModel,
    RustSourceName, SourceLocation,
};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::Ident;

use crate::macros::utils::{
    CodegenContext, static_argument_name_tokens, static_domain_tokens, static_entry_id_tokens,
};

#[derive(Clone)]
pub(crate) struct FluentArgument {
    pub(crate) metadata: ArgumentModel,
    pub(crate) value_expr: TokenStream,
}

impl FluentArgument {
    fn context_bound_insert_statement(&self, context: &CodegenContext) -> TokenStream {
        let value_expr = &self.value_expr;
        let es_fluent = context.facade_path().tokens();
        let key = static_argument_name_tokens(context, self.metadata.name());
        quote! {
            {
                use #es_fluent::__private::IntoFluentArgumentValue as _;
                args.insert(
                    #key,
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
        name: metadata.rust_source_name().clone(),
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
        let domain_expr = static_domain_tokens(context, self.domain_override.as_ref());
        let ftl_key_expr = static_entry_id_tokens(context, &self.ftl_key);

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
    pub(crate) name: RustSourceName,
    pub(crate) ftl_key: FluentMessageId,
    pub(crate) arg_names: Vec<ArgName>,
    pub(crate) source_location: SourceLocation,
}

impl InventoryVariantSpec {
    pub(crate) fn tokens(&self, context: &CodegenContext) -> TokenStream {
        let name = self.name.as_str();
        let es_fluent = context.facade_path().tokens();
        let args_tokens: Vec<_> = self
            .arg_names
            .iter()
            .map(|arg| static_argument_name_tokens(context, arg))
            .collect();
        let entry_id = static_entry_id_tokens(context, &self.ftl_key);
        let source_span = self.source_location.span();
        let source_line = quote_spanned! { source_span=> line!() };

        quote! {
            #es_fluent::registry::__macro::ftl_variant(
                #name,
                #entry_id,
                &[#(#args_tokens),*],
                module_path!(),
                #source_line,
            )
        }
    }
}

pub(crate) struct GeneratedUnitEnumVariant {
    pub(crate) ident: Ident,
    pub(crate) doc_name: GeneratedDocName,
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

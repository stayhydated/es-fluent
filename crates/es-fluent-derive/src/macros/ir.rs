use es_fluent_derive_core::error::AttrContext;
use es_fluent_derive_core::semantic::{
    ArgumentModel, MessageEntryModel, SourceLocation, SpannedValue,
    parse_fluent_message_id_in_context,
};
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::Ident;

#[derive(Clone)]
pub(crate) struct FluentArgument {
    pub(crate) metadata: ArgumentModel,
    pub(crate) value_expr: TokenStream,
}

impl FluentArgument {
    pub(crate) fn name(&self) -> &str {
        self.metadata.name().as_str()
    }

    fn context_bound_insert_statement(&self) -> TokenStream {
        let key = self.name();
        let value_expr = &self.value_expr;
        quote! {
            {
                use ::es_fluent::__private::IntoFluentArgumentValue as _;
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
    pub(crate) fn new(
        source_name: String,
        ftl_key: String,
        source_span: Span,
        runtime_arguments: Vec<FluentArgument>,
    ) -> Self {
        let message_id =
            parse_fluent_message_id_in_context(ftl_key, source_span, AttrContext::MessageContainer)
                .unwrap_or_else(|error| error.abort());
        let arguments = runtime_arguments
            .iter()
            .map(|argument| argument.metadata.clone())
            .collect();

        Self {
            metadata: MessageEntryModel::new(
                source_name,
                SpannedValue::new(message_id, source_span),
                arguments,
                SourceLocation::new(source_span),
            ),
            runtime_arguments,
        }
    }

    pub(crate) fn ftl_key(&self) -> &str {
        self.metadata.message_id().as_str()
    }

    pub(crate) fn localize_with_expr(&self, domain_override: Option<String>) -> TokenStream {
        LocalizeCallSpec {
            domain_override,
            ftl_key: self.ftl_key().to_string(),
            arguments: self.runtime_arguments.clone(),
        }
        .localize_with_expr()
    }
}

pub(crate) fn inventory_variant_tokens_for_model(metadata: &MessageEntryModel) -> TokenStream {
    InventoryVariantSpec {
        name: metadata.source_name().to_string(),
        ftl_key: metadata.message_id().to_string(),
        arg_names: metadata.argument_names(),
        source_location: metadata.source_location().clone(),
    }
    .tokens()
}

pub(crate) struct LocalizeCallSpec {
    pub(crate) domain_override: Option<String>,
    pub(crate) ftl_key: String,
    pub(crate) arguments: Vec<FluentArgument>,
}

impl LocalizeCallSpec {
    pub(crate) fn localize_with_expr(&self) -> TokenStream {
        let domain_expr = match self.domain_override.as_deref() {
            Some(domain) => quote! { #domain },
            None => quote! { env!("CARGO_PKG_NAME") },
        };
        let ftl_key = &self.ftl_key;

        if self.arguments.is_empty() {
            quote! {
                localize(#domain_expr, #ftl_key, None)
            }
        } else {
            let inserts: Vec<_> = self
                .arguments
                .iter()
                .map(FluentArgument::context_bound_insert_statement)
                .collect();

            quote! {
                {
                    let mut args = ::std::collections::HashMap::new();
                    #(#inserts)*
                    localize(#domain_expr, #ftl_key, Some(&args))
                }
            }
        }
    }
}

pub(crate) struct InventoryVariantSpec {
    pub(crate) name: String,
    pub(crate) ftl_key: String,
    pub(crate) arg_names: Vec<String>,
    pub(crate) source_location: SourceLocation,
}

impl InventoryVariantSpec {
    pub(crate) fn tokens(&self) -> TokenStream {
        let name = &self.name;
        let ftl_key = &self.ftl_key;
        let args_tokens: Vec<_> = self.arg_names.iter().map(|arg| quote! { #arg }).collect();
        let source_span = self.source_location.span();
        let source_line = quote_spanned! { source_span=> line!() };

        quote! {
            ::es_fluent::registry::FtlVariant {
                name: #name,
                ftl_key: #ftl_key,
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
    pub(crate) fn localize_with_match_arm(&self, domain_override: Option<&str>) -> TokenStream {
        let variant_ident = &self.ident;
        let body = self
            .message_entry
            .localize_with_expr(domain_override.map(str::to_string));

        quote! {
            Self::#variant_ident => #body
        }
    }
}

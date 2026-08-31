use crate::error::{AttrContext, EsFluentCoreResult};
use es_fluent_shared::{namer, namespace::NamespaceRule};

use super::{
    DerivePathList, DomainName, GeneratedDocName, MessageEntryModel, RustSourceName, RustTypeName,
    SourceLocation, SpannedValue, VariantKey, generated_variant_message_id,
    parse_variant_key_in_context,
};

/// Semantic seed for one generated unit-enum variant before the target enum key is known.
#[derive(Clone, Debug)]
pub struct GeneratedVariantMessageSeed {
    ident: syn::Ident,
    doc_name: GeneratedDocName,
    key_fragment: SpannedValue<VariantKey>,
}

impl GeneratedVariantMessageSeed {
    pub fn new(
        ident: syn::Ident,
        doc_name: impl Into<String>,
        key_fragment: impl Into<String>,
        span: proc_macro2::Span,
        context: AttrContext,
    ) -> EsFluentCoreResult<Self> {
        let key_fragment = parse_variant_key_in_context(key_fragment, span, context)?;
        Ok(Self {
            ident,
            doc_name: GeneratedDocName::new(doc_name),
            key_fragment: SpannedValue::new(key_fragment, span),
        })
    }

    pub fn ident(&self) -> &syn::Ident {
        &self.ident
    }

    pub fn doc_name(&self) -> &GeneratedDocName {
        &self.doc_name
    }

    pub fn materialize_message(
        &self,
        base_key: &namer::FluentKey,
        context: AttrContext,
    ) -> EsFluentCoreResult<MessageEntryModel> {
        let message_id = generated_variant_message_id(
            base_key,
            self.key_fragment.value().as_str(),
            self.key_fragment.span(),
            context,
        )?;
        Ok(MessageEntryModel::new(
            RustSourceName::from_ident(&self.ident),
            message_id.clone(),
            Vec::new(),
            SourceLocation::new(message_id.span()),
        ))
    }
}

/// Semantic model for a generated unit enum.
#[derive(Clone, Debug)]
pub struct GeneratedEnumModel {
    ident: RustTypeName,
    origin_ident: RustTypeName,
    derives: DerivePathList,
    messages: Vec<MessageEntryModel>,
    label: Option<MessageEntryModel>,
    domain: Option<DomainName>,
    namespace: Option<NamespaceRule>,
}

impl GeneratedEnumModel {
    pub fn new(
        ident: RustTypeName,
        origin_ident: RustTypeName,
        derives: DerivePathList,
        messages: Vec<MessageEntryModel>,
        label: Option<MessageEntryModel>,
        domain: Option<DomainName>,
        namespace: Option<NamespaceRule>,
    ) -> Self {
        Self {
            ident,
            origin_ident,
            derives,
            messages,
            label,
            domain,
            namespace,
        }
    }

    pub fn ident(&self) -> &str {
        self.ident.as_str()
    }

    pub fn origin_ident(&self) -> &str {
        self.origin_ident.as_str()
    }

    pub fn derives(&self) -> &DerivePathList {
        &self.derives
    }

    pub fn messages(&self) -> &[MessageEntryModel] {
        &self.messages
    }

    pub fn label(&self) -> Option<&MessageEntryModel> {
        self.label.as_ref()
    }

    pub fn domain(&self) -> Option<&DomainName> {
        self.domain.as_ref()
    }

    pub fn namespace(&self) -> Option<&NamespaceRule> {
        self.namespace.as_ref()
    }
}

use es_fluent_shared::{meta::TypeKind, namespace::NamespaceRule};
use proc_macro2::Span;

use super::{
    ArgName, ArgumentModel, DomainName, FluentMessageId, RustSourceName, RustTypeName,
    SourceLocation, SpannedValue,
};

/// Semantic metadata for one generated Fluent message entry.
#[derive(Clone, Debug)]
pub struct MessageEntryModel {
    source_name: RustSourceName,
    message_id: SpannedValue<FluentMessageId>,
    arguments: Vec<ArgumentModel>,
    source_location: SourceLocation,
}

impl MessageEntryModel {
    pub fn new(
        source_name: RustSourceName,
        message_id: SpannedValue<FluentMessageId>,
        arguments: Vec<ArgumentModel>,
        source_location: SourceLocation,
    ) -> Self {
        Self {
            source_name,
            message_id,
            arguments,
            source_location,
        }
    }

    pub fn source_name(&self) -> &str {
        self.source_name.as_str()
    }

    pub fn rust_source_name(&self) -> &RustSourceName {
        &self.source_name
    }

    pub fn message_id(&self) -> &FluentMessageId {
        self.message_id.value()
    }

    pub fn span(&self) -> Span {
        self.source_location.span()
    }

    pub fn source_location(&self) -> &SourceLocation {
        &self.source_location
    }

    pub fn arguments(&self) -> &[ArgumentModel] {
        &self.arguments
    }

    pub fn argument_names(&self) -> Vec<ArgName> {
        self.arguments
            .iter()
            .map(|argument| argument.name().clone())
            .collect()
    }
}

/// Semantic model for messages generated from one source type.
#[derive(Clone, Debug)]
pub struct MessageModel {
    source_type: RustTypeName,
    type_kind: TypeKind,
    domain: Option<DomainName>,
    namespace: Option<NamespaceRule>,
    messages: Vec<MessageEntryModel>,
    label: Option<MessageEntryModel>,
}

impl MessageModel {
    pub fn new(
        source_type: RustTypeName,
        type_kind: TypeKind,
        domain: Option<DomainName>,
        namespace: Option<NamespaceRule>,
        messages: Vec<MessageEntryModel>,
        label: Option<MessageEntryModel>,
    ) -> Self {
        Self {
            source_type,
            type_kind,
            domain,
            namespace,
            messages,
            label,
        }
    }

    pub fn source_type(&self) -> &str {
        self.source_type.as_str()
    }

    pub fn type_kind(&self) -> &TypeKind {
        &self.type_kind
    }

    pub fn domain(&self) -> Option<&DomainName> {
        self.domain.as_ref()
    }

    pub fn namespace(&self) -> Option<&NamespaceRule> {
        self.namespace.as_ref()
    }

    pub fn messages(&self) -> &[MessageEntryModel] {
        &self.messages
    }

    pub fn label(&self) -> Option<&MessageEntryModel> {
        self.label.as_ref()
    }
}

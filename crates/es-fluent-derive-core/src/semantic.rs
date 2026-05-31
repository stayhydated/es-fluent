//! Typed semantic values built from parsed derive attributes.

use crate::{
    error::{AttrContext, AttrError, ErrorExt as _, EsFluentCoreError, EsFluentCoreResult},
    options::choice::CaseStyle,
};
use es_fluent_shared::{
    fluent::{
        FluentArgumentName, FluentDomain, FluentIdentifierError,
        FluentMessageId as SharedMessageId, FluentVariantKey,
    },
    meta::TypeKind,
    namespace::NamespaceRule,
};
use proc_macro2::Span;
use quote::ToTokens as _;
use strum::IntoEnumIterator as _;
use syn::spanned::Spanned as _;

pub use es_fluent_shared::fluent::{
    FluentArgumentName as ArgName, FluentDomain as DomainName, FluentMessageId,
    FluentVariantKey as VariantKey,
};

/// A value paired with the best source span available for diagnostics or code emission.
#[derive(Clone, Debug)]
pub struct SpannedValue<T> {
    value: T,
    span: Span,
}

impl<T> SpannedValue<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn into_value(self) -> T {
        self.value
    }
}

pub fn parse_arg_name(value: impl Into<String>, span: Span) -> EsFluentCoreResult<ArgName> {
    parse_arg_name_in_context(value, span, AttrContext::MessageContainer)
}

pub fn parse_arg_name_in_context(
    value: impl Into<String>,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreResult<ArgName> {
    FluentArgumentName::try_new(value).map_err(|error| semantic_error(error, span, context))
}

pub fn parse_variant_key_in_context(
    value: impl Into<String>,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreResult<VariantKey> {
    FluentVariantKey::try_new(value).map_err(|error| semantic_error(error, span, context))
}

pub fn parse_domain_name_in_context(
    value: impl Into<String>,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreResult<DomainName> {
    FluentDomain::try_new(value).map_err(|error| semantic_error(error, span, context))
}

pub fn parse_fluent_message_id_in_context(
    value: impl Into<String>,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreResult<FluentMessageId> {
    SharedMessageId::try_new(value).map_err(|error| semantic_error(error, span, context))
}

/// Source location metadata for a generated semantic model entry.
#[derive(Clone, Debug)]
pub struct SourceLocation {
    span: Span,
}

impl SourceLocation {
    pub fn new(span: Span) -> Self {
        Self { span }
    }

    pub fn span(&self) -> Span {
        self.span
    }
}

/// Semantic metadata for one generated Fluent argument.
#[derive(Clone, Debug)]
pub struct ArgumentModel {
    name: SpannedValue<ArgName>,
    value_strategy: ArgumentValueStrategy,
}

impl ArgumentModel {
    pub fn new(name: SpannedValue<ArgName>) -> Self {
        Self::new_with_value_strategy(name, ArgumentValueStrategy::Borrowed)
    }

    pub fn new_with_value_strategy(
        name: SpannedValue<ArgName>,
        value_strategy: ArgumentValueStrategy,
    ) -> Self {
        Self {
            name,
            value_strategy,
        }
    }

    pub fn name(&self) -> &ArgName {
        self.name.value()
    }

    pub fn span(&self) -> Span {
        self.name.span()
    }

    pub fn value_strategy(&self) -> &ArgumentValueStrategy {
        &self.value_strategy
    }
}

/// Runtime value strategy for one generated Fluent argument.
#[derive(Clone, Debug)]
pub enum ArgumentValueStrategy {
    /// Borrow the field value and let runtime autoref dispatch choose the final value form.
    Borrowed,
    /// Treat the field value as an `Option<T>`.
    Optional,
    /// Convert the field value through `EsFluentChoice`.
    Choice,
    /// Apply an explicit field-level transform expression.
    Transform(ValueTransform),
}

/// Explicit field-level value transform expression.
#[derive(Clone, Debug)]
pub struct ValueTransform {
    expr: syn::Expr,
    span: Span,
}

impl ValueTransform {
    pub fn new(expr: syn::Expr, span: Span) -> Self {
        Self { expr, span }
    }

    pub fn expr(&self) -> &syn::Expr {
        &self.expr
    }

    pub fn span(&self) -> Span {
        self.span
    }
}

/// Semantic metadata for one generated Fluent message entry.
#[derive(Clone, Debug)]
pub struct MessageEntryModel {
    source_name: String,
    message_id: SpannedValue<FluentMessageId>,
    arguments: Vec<ArgumentModel>,
    source_location: SourceLocation,
}

impl MessageEntryModel {
    pub fn new(
        source_name: impl Into<String>,
        message_id: SpannedValue<FluentMessageId>,
        arguments: Vec<ArgumentModel>,
        source_location: SourceLocation,
    ) -> Self {
        Self {
            source_name: source_name.into(),
            message_id,
            arguments,
            source_location,
        }
    }

    pub fn source_name(&self) -> &str {
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

    pub fn argument_names(&self) -> Vec<String> {
        self.arguments
            .iter()
            .map(|argument| argument.name().to_string())
            .collect()
    }
}

/// Inventory behavior for a semantic message model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InventoryPolicy {
    /// Emit inventory metadata for the model.
    Emit,
    /// Skip inventory metadata for the model.
    Skip,
}

impl InventoryPolicy {
    pub fn should_emit(self) -> bool {
        matches!(self, Self::Emit)
    }
}

/// Semantic model for messages generated from one source type.
#[derive(Clone, Debug)]
pub struct MessageModel {
    source_type: String,
    type_kind: TypeKind,
    domain: Option<DomainName>,
    namespace: Option<NamespaceRule>,
    messages: Vec<MessageEntryModel>,
    label: Option<MessageEntryModel>,
    inventory_policy: InventoryPolicy,
}

impl MessageModel {
    pub fn new(
        source_type: impl Into<String>,
        type_kind: TypeKind,
        domain: Option<DomainName>,
        namespace: Option<NamespaceRule>,
        messages: Vec<MessageEntryModel>,
        label: Option<MessageEntryModel>,
        inventory_policy: InventoryPolicy,
    ) -> Self {
        Self {
            source_type: source_type.into(),
            type_kind,
            domain,
            namespace,
            messages,
            label,
            inventory_policy,
        }
    }

    pub fn source_type(&self) -> &str {
        &self.source_type
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

    pub fn inventory_policy(&self) -> InventoryPolicy {
        self.inventory_policy
    }
}

/// A validated derive path for a generated enum.
#[derive(Clone, Debug)]
pub struct DerivePath {
    path: syn::Path,
    span: Span,
}

impl DerivePath {
    pub fn new(path: syn::Path, context: AttrContext) -> EsFluentCoreResult<Self> {
        let span = path.span();
        if path.segments.is_empty() {
            return Err(EsFluentCoreError::StructuredAttributeError(AttrError::new(
                context,
                "derive path must not be empty",
                Some(span),
            )));
        }

        Ok(Self { path, span })
    }

    pub fn path(&self) -> &syn::Path {
        &self.path
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn to_token_string(&self) -> String {
        self.path.to_token_stream().to_string()
    }
}

/// Validated derive paths for a generated enum.
#[derive(Clone, Debug, Default)]
pub struct DerivePathList {
    paths: Vec<DerivePath>,
}

impl DerivePathList {
    pub fn from_paths(
        paths: impl IntoIterator<Item = syn::Path>,
        context: AttrContext,
    ) -> EsFluentCoreResult<Self> {
        let paths = paths
            .into_iter()
            .map(|path| DerivePath::new(path, context))
            .collect::<EsFluentCoreResult<Vec<_>>>()?;
        Ok(Self { paths })
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn paths(&self) -> &[DerivePath] {
        &self.paths
    }

    pub fn token_strings(&self) -> Vec<String> {
        self.paths.iter().map(DerivePath::to_token_string).collect()
    }
}

/// Semantic model for a generated unit enum.
#[derive(Clone, Debug)]
pub struct GeneratedEnumModel {
    ident: String,
    origin_ident: String,
    derives: DerivePathList,
    messages: Vec<MessageEntryModel>,
    label: Option<MessageEntryModel>,
    domain: Option<DomainName>,
    namespace: Option<NamespaceRule>,
}

impl GeneratedEnumModel {
    pub fn new(
        ident: impl Into<String>,
        origin_ident: impl Into<String>,
        derives: DerivePathList,
        messages: Vec<MessageEntryModel>,
        label: Option<MessageEntryModel>,
        domain: Option<DomainName>,
        namespace: Option<NamespaceRule>,
    ) -> Self {
        Self {
            ident: ident.into(),
            origin_ident: origin_ident.into(),
            derives,
            messages,
            label,
            domain,
            namespace,
        }
    }

    pub fn ident(&self) -> &str {
        &self.ident
    }

    pub fn origin_ident(&self) -> &str {
        &self.origin_ident
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

/// Semantic mapping for one `EsFluentChoice` enum variant.
#[derive(Clone, Debug)]
pub struct ChoiceVariantModel {
    ident: String,
    value: SpannedValue<String>,
}

impl ChoiceVariantModel {
    pub fn new(ident: impl Into<String>, value: SpannedValue<String>) -> Self {
        Self {
            ident: ident.into(),
            value,
        }
    }

    pub fn ident(&self) -> &str {
        &self.ident
    }

    pub fn value(&self) -> &str {
        self.value.value()
    }

    pub fn span(&self) -> Span {
        self.value.span()
    }
}

/// Semantic model for an `EsFluentChoice` implementation.
#[derive(Clone, Debug)]
pub struct ChoiceModel {
    ident: String,
    variants: Vec<ChoiceVariantModel>,
}

impl ChoiceModel {
    pub fn from_variant_idents<'a>(
        ident: &syn::Ident,
        variant_idents: impl IntoIterator<Item = &'a syn::Ident>,
        rename_all: Option<&str>,
    ) -> EsFluentCoreResult<Self> {
        let case_style = parse_choice_case_style(rename_all, ident.span())?;
        let variants = variant_idents
            .into_iter()
            .map(|variant_ident| {
                let variant_name = es_fluent_shared::namer::rust_ident_name(variant_ident);
                let value = case_style
                    .map_or_else(|| variant_name.clone(), |style| style.apply(&variant_name));
                ChoiceVariantModel::new(
                    variant_name,
                    SpannedValue::new(value, variant_ident.span()),
                )
            })
            .collect();

        Ok(Self {
            ident: es_fluent_shared::namer::rust_ident_name(ident),
            variants,
        })
    }

    pub fn ident(&self) -> &str {
        &self.ident
    }

    pub fn variants(&self) -> &[ChoiceVariantModel] {
        &self.variants
    }
}

fn parse_choice_case_style(
    rename_all: Option<&str>,
    span: Span,
) -> EsFluentCoreResult<Option<CaseStyle>> {
    let Some(rename_all) = rename_all else {
        return Ok(None);
    };

    rename_all
        .parse::<CaseStyle>()
        .map(Some)
        .map_err(|message| {
            let supported = CaseStyle::iter()
                .map(|style| style.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            EsFluentCoreError::StructuredAttributeError(AttrError::new(
                AttrContext::ChoiceContainer,
                message.to_string(),
                Some(span),
            ))
            .with_help(format!("supported values are: {supported}"))
        })
}

fn semantic_error(
    error: FluentIdentifierError,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreError {
    EsFluentCoreError::StructuredAttributeError(AttrError::new(
        context,
        error.to_string(),
        Some(span),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_names_accept_current_generated_shapes() {
        let span = Span::call_site();

        assert_eq!(
            parse_fluent_message_id_in_context("status-Ready", span, AttrContext::MessageContainer)
                .expect("message id")
                .as_str(),
            "status-Ready"
        );
        assert_eq!(
            parse_arg_name("display_name", span)
                .expect("argument")
                .as_str(),
            "display_name"
        );
        assert_eq!(
            parse_variant_key_in_context("custom-key", span, AttrContext::EnumVariant)
                .expect("variant key")
                .as_str(),
            "custom-key"
        );
        assert_eq!(
            parse_domain_name_in_context("es-fluent-lang", span, AttrContext::MessageContainer)
                .expect("domain")
                .as_str(),
            "es-fluent-lang"
        );
    }

    #[test]
    fn typed_names_reject_empty_leading_digit_and_invalid_characters() {
        let span = Span::call_site();

        let err = parse_arg_name_in_context("", span, AttrContext::MessageField)
            .expect_err("empty arg should fail");
        assert_eq!(
            err.to_string(),
            "Attribute error in message field: Fluent argument name must not be empty"
        );

        assert!(parse_arg_name("1value", span).is_err());
        assert!(parse_arg_name("display name", span).is_err());
        assert!(
            parse_fluent_message_id_in_context("_message", span, AttrContext::MessageContainer)
                .is_err()
        );
    }

    #[test]
    fn message_entry_model_returns_inventory_argument_names_from_arguments() {
        let span = Span::call_site();
        let entry = MessageEntryModel::new(
            "Ready",
            SpannedValue::new(
                parse_fluent_message_id_in_context(
                    "status-Ready",
                    span,
                    AttrContext::MessageContainer,
                )
                .expect("message id"),
                span,
            ),
            vec![
                ArgumentModel::new(SpannedValue::new(
                    parse_arg_name("first", span).expect("arg"),
                    span,
                )),
                ArgumentModel::new_with_value_strategy(
                    SpannedValue::new(parse_arg_name("second", span).expect("arg"), span),
                    ArgumentValueStrategy::Choice,
                ),
            ],
            SourceLocation::new(span),
        );

        assert_eq!(entry.source_name(), "Ready");
        assert_eq!(entry.message_id().as_str(), "status-Ready");
        let _span = entry.source_location().span();
        assert_eq!(
            entry.argument_names(),
            vec!["first".to_string(), "second".to_string()]
        );
        assert!(matches!(
            entry.arguments()[1].value_strategy(),
            ArgumentValueStrategy::Choice
        ));
    }

    #[test]
    fn message_model_groups_entries_with_inventory_policy() {
        let span = Span::call_site();
        let entry = MessageEntryModel::new(
            "Ready",
            SpannedValue::new(
                parse_fluent_message_id_in_context(
                    "status-Ready",
                    span,
                    AttrContext::MessageContainer,
                )
                .expect("message id"),
                span,
            ),
            Vec::new(),
            SourceLocation::new(span),
        );
        let model = MessageModel::new(
            "Status",
            TypeKind::Enum,
            None,
            None,
            vec![entry.clone()],
            None,
            InventoryPolicy::Emit,
        );

        assert_eq!(model.source_type(), "Status");
        assert!(matches!(model.type_kind(), TypeKind::Enum));
        assert!(model.inventory_policy().should_emit());
        assert_eq!(model.messages()[0].message_id().as_str(), "status-Ready");

        let generated = GeneratedEnumModel::new(
            "StatusFtl",
            "Status",
            DerivePathList::from_paths(
                vec![syn::parse_quote!(Debug)],
                AttrContext::VariantsContainer,
            )
            .expect("derive paths"),
            vec![entry],
            None,
            None,
            None,
        );

        assert_eq!(generated.ident(), "StatusFtl");
        assert_eq!(generated.origin_ident(), "Status");
        assert_eq!(
            generated.derives().token_strings(),
            vec!["Debug".to_string()]
        );
        assert_eq!(generated.messages()[0].source_name(), "Ready");
    }

    #[test]
    fn choice_model_applies_rename_all_once() {
        let choice_ident: syn::Ident = syn::parse_quote!(SeverityChoice);
        let high_ident: syn::Ident = syn::parse_quote!(VeryHigh);
        let low_ident: syn::Ident = syn::parse_quote!(Low);

        let model = ChoiceModel::from_variant_idents(
            &choice_ident,
            [&high_ident, &low_ident],
            Some("snake_case"),
        )
        .expect("choice model");

        assert_eq!(model.ident(), "SeverityChoice");
        assert_eq!(model.variants()[0].ident(), "VeryHigh");
        assert_eq!(model.variants()[0].value(), "very_high");
        assert_eq!(model.variants()[1].value(), "low");
    }

    #[test]
    fn choice_model_rejects_invalid_rename_all_in_choice_context() {
        let choice_ident: syn::Ident = syn::parse_quote!(SeverityChoice);
        let variant_ident: syn::Ident = syn::parse_quote!(VeryHigh);

        let err =
            ChoiceModel::from_variant_idents(&choice_ident, [&variant_ident], Some("not_a_style"))
                .expect_err("invalid rename_all should fail");

        assert!(
            err.to_string()
                .contains("Attribute error in choice container")
        );
        assert!(err.to_string().contains("supported values are"));
    }
}

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
    namer,
    namespace::NamespaceRule,
};
use heck::{ToPascalCase as _, ToSnakeCase as _};
use proc_macro2::Span;
use quote::ToTokens as _;
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

pub fn spanned_message_id_from_value(
    value: impl Into<String>,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreResult<SpannedValue<FluentMessageId>> {
    let value = parse_fluent_message_id_in_context(value, span, context)?;
    Ok(SpannedValue::new(value, span))
}

pub fn message_id_from_fluent_key(
    key: namer::FluentKey,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreResult<SpannedValue<FluentMessageId>> {
    spanned_message_id_from_value(key.to_string(), span, context)
}

pub fn message_id_for_ident(
    ident: &syn::Ident,
    context: AttrContext,
) -> EsFluentCoreResult<SpannedValue<FluentMessageId>> {
    message_id_from_fluent_key(namer::FluentKey::from(ident), ident.span(), context)
}

pub fn label_message_id_for_ident(
    ident: &syn::Ident,
    context: AttrContext,
) -> EsFluentCoreResult<SpannedValue<FluentMessageId>> {
    message_id_from_fluent_key(namer::FluentKey::new_label(ident), ident.span(), context)
}

pub fn variant_message_id(
    base_key: &FluentMessageId,
    variant_ident: &syn::Ident,
    override_key: Option<&VariantKey>,
    context: AttrContext,
) -> EsFluentCoreResult<SpannedValue<FluentMessageId>> {
    let variant_key_suffix = override_key
        .map(VariantKey::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| namer::rust_ident_name(variant_ident));
    message_id_from_fluent_key(
        namer::FluentKey::from(base_key.as_str()).join(&variant_key_suffix),
        variant_ident.span(),
        context,
    )
}

pub fn generated_variant_message_id(
    base_key: &namer::FluentKey,
    key_fragment: &str,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreResult<SpannedValue<FluentMessageId>> {
    message_id_from_fluent_key(base_key.join(key_fragment), span, context)
}

pub fn generated_label_message_id(
    base_key: &namer::FluentKey,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreResult<SpannedValue<FluentMessageId>> {
    spanned_message_id_from_value(
        format!("{}{}", base_key, namer::FluentKey::LABEL_SUFFIX),
        span,
        context,
    )
}

pub fn generated_label_message_value(
    base_key: &namer::FluentKey,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreResult<FluentMessageId> {
    generated_label_message_id(base_key, span, context).map(SpannedValue::into_value)
}

/// Rust type identifier metadata preserved with its source span.
#[derive(Clone, Debug)]
pub struct RustTypeName {
    value: String,
    span: Span,
}

impl RustTypeName {
    pub fn from_ident(ident: &syn::Ident) -> Self {
        Self {
            value: namer::rust_ident_name(ident),
            span: ident.span(),
        }
    }

    pub fn new(value: impl Into<String>, span: Span) -> Self {
        Self {
            value: value.into(),
            span,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn span(&self) -> Span {
        self.span
    }
}

/// Rust item name metadata preserved with its source span.
#[derive(Clone, Debug)]
pub struct RustSourceName {
    value: String,
    span: Span,
}

impl RustSourceName {
    pub fn from_ident(ident: &syn::Ident) -> Self {
        Self {
            value: namer::rust_ident_name(ident),
            span: ident.span(),
        }
    }

    pub fn new(value: impl Into<String>, span: Span) -> Self {
        Self {
            value: value.into(),
            span,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn span(&self) -> Span {
        self.span
    }
}

/// A typed generated variant key from `#[fluent_variants(keys = [...])]`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GeneratedKeyName {
    value: String,
}

impl GeneratedKeyName {
    pub fn try_new(
        value: impl Into<String>,
        span: Span,
        context: AttrContext,
    ) -> EsFluentCoreResult<Self> {
        let value = value.into();
        let snake_cased = value.to_snake_case();
        let is_lower_snake =
            !value.is_empty() && value == snake_cased && value == value.to_ascii_lowercase();

        if !is_lower_snake {
            return Err(EsFluentCoreError::StructuredAttributeError(AttrError::new(
                context,
                format!(
                    "keys in #[fluent_variants] must be lowercase snake_case; found \"{}\"",
                    value
                ),
                Some(span),
            ))
            .with_help("use values like \"description\" or \"label\"".to_string()));
        }

        Ok(Self { value })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn to_pascal_case(&self) -> String {
        self.value.to_pascal_case()
    }
}

/// A generated Rust identifier derived from a typed generated variant key.
#[derive(Clone, Debug)]
pub struct GeneratedKeyIdent {
    ident: syn::Ident,
}

impl GeneratedKeyIdent {
    pub fn variants(
        source_ident: &syn::Ident,
        key: &SpannedValue<GeneratedKeyName>,
        suffix: &str,
    ) -> Self {
        Self::from_parts(source_ident, key, suffix)
    }

    pub fn base(source_ident: &syn::Ident, key: &SpannedValue<GeneratedKeyName>) -> Self {
        Self::from_parts(source_ident, key, "")
    }

    fn from_parts(
        source_ident: &syn::Ident,
        key: &SpannedValue<GeneratedKeyName>,
        suffix: &str,
    ) -> Self {
        let ident = syn::Ident::new(
            &format!(
                "{}{}{}",
                namer::rust_ident_name(source_ident),
                key.value().to_pascal_case(),
                suffix
            ),
            key.span(),
        );
        Self { ident }
    }

    pub fn into_ident(self) -> syn::Ident {
        self.ident
    }
}

/// Display/source name used in generated enum documentation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedDocName(String);

impl GeneratedDocName {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for GeneratedDocName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

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
        span: Span,
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
        let span = name.span();
        Self::new_with_value_strategy(name, ArgumentValueStrategy::Borrowed { span })
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
    Borrowed { span: Span },
    /// Treat the field value as an `Option<T>`.
    Optional { span: Span },
    /// Convert the field value through `EsFluentChoice`.
    Choice { span: Span },
    /// Apply an explicit field-level transform expression.
    Transform(Box<ValueTransform>),
}

impl ArgumentValueStrategy {
    pub fn span(&self) -> Span {
        match self {
            Self::Borrowed { span } | Self::Optional { span } | Self::Choice { span } => *span,
            Self::Transform(transform) => transform.span(),
        }
    }
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

/// Semantic mapping for one `EsFluentChoice` enum variant.
#[derive(Clone, Debug)]
pub struct ChoiceVariantModel {
    ident: syn::Ident,
    value: SpannedValue<String>,
}

impl ChoiceVariantModel {
    pub fn new(ident: syn::Ident, value: SpannedValue<String>) -> Self {
        Self { ident, value }
    }

    pub fn ident(&self) -> &syn::Ident {
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
    ident: syn::Ident,
    variants: Vec<ChoiceVariantModel>,
}

impl ChoiceModel {
    pub fn from_variant_idents<'a>(
        ident: &syn::Ident,
        variant_idents: impl IntoIterator<Item = &'a syn::Ident>,
        rename_all: Option<CaseStyle>,
    ) -> EsFluentCoreResult<Self> {
        let variants = variant_idents
            .into_iter()
            .map(|variant_ident| {
                let variant_name = es_fluent_shared::namer::rust_ident_name(variant_ident);
                let value = rename_all
                    .map_or_else(|| variant_name.clone(), |style| style.apply(&variant_name));
                ChoiceVariantModel::new(
                    variant_ident.clone(),
                    SpannedValue::new(value, variant_ident.span()),
                )
            })
            .collect();

        Ok(Self {
            ident: ident.clone(),
            variants,
        })
    }

    pub fn ident(&self) -> &syn::Ident {
        &self.ident
    }

    pub fn variants(&self) -> &[ChoiceVariantModel] {
        &self.variants
    }
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
    fn generated_message_id_helpers_return_typed_spanned_values() {
        let span = Span::call_site();
        let login_form: syn::Ident = syn::parse_quote!(LoginForm);
        let login_error: syn::Ident = syn::parse_quote!(LoginError);
        let failed: syn::Ident = syn::parse_quote!(Failed);
        let username: syn::Ident = syn::parse_quote!(Username);

        assert_eq!(
            message_id_for_ident(&login_form, AttrContext::MessageContainer)
                .expect("struct message id")
                .value()
                .as_str(),
            "login_form"
        );
        assert_eq!(
            label_message_id_for_ident(&login_form, AttrContext::LabelContainer)
                .expect("label message id")
                .value()
                .as_str(),
            "login_form_label"
        );

        let base = message_id_for_ident(&login_error, AttrContext::MessageContainer)
            .expect("enum base")
            .into_value();
        assert_eq!(
            variant_message_id(&base, &failed, None, AttrContext::EnumVariant)
                .expect("variant message id")
                .value()
                .as_str(),
            "login_error-Failed"
        );

        let override_key =
            parse_variant_key_in_context("custom-key", span, AttrContext::EnumVariant)
                .expect("override key");
        assert_eq!(
            variant_message_id(
                &base,
                &failed,
                Some(&override_key),
                AttrContext::EnumVariant
            )
            .expect("overridden variant message id")
            .value()
            .as_str(),
            "login_error-custom-key"
        );

        let generated_base = namer::FluentKey::from("login_form_label_variants");
        assert_eq!(
            generated_variant_message_id(
                &generated_base,
                "username",
                username.span(),
                AttrContext::VariantsContainer,
            )
            .expect("generated variant id")
            .value()
            .as_str(),
            "login_form_label_variants-username"
        );
        assert_eq!(
            generated_label_message_id(&generated_base, span, AttrContext::VariantsContainer,)
                .expect("generated label id")
                .value()
                .as_str(),
            "login_form_label_variants_label"
        );
    }

    #[test]
    fn message_entry_model_returns_inventory_argument_names_from_arguments() {
        let span = Span::call_site();
        let entry = MessageEntryModel::new(
            RustSourceName::new("Ready", span),
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
                    ArgumentValueStrategy::Choice { span },
                ),
            ],
            SourceLocation::new(span),
        );

        assert_eq!(entry.source_name(), "Ready");
        assert_eq!(entry.message_id().as_str(), "status-Ready");
        let _span = entry.source_location().span();
        assert_eq!(
            entry
                .argument_names()
                .iter()
                .map(ArgName::as_str)
                .collect::<Vec<_>>(),
            vec!["first", "second"]
        );
        assert!(matches!(
            entry.arguments()[1].value_strategy(),
            ArgumentValueStrategy::Choice { .. }
        ));
    }

    #[test]
    fn message_model_groups_entries() {
        let span = Span::call_site();
        let entry = MessageEntryModel::new(
            RustSourceName::new("Ready", span),
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
            RustTypeName::new("Status", proc_macro2::Span::call_site()),
            TypeKind::Enum,
            None,
            None,
            vec![entry.clone()],
            None,
        );

        assert_eq!(model.source_type(), "Status");
        assert!(matches!(model.type_kind(), TypeKind::Enum));
        assert_eq!(model.messages()[0].message_id().as_str(), "status-Ready");

        let generated = GeneratedEnumModel::new(
            RustTypeName::new("StatusFtl", proc_macro2::Span::call_site()),
            RustTypeName::new("Status", proc_macro2::Span::call_site()),
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
            Some(CaseStyle::SnakeCase),
        )
        .expect("choice model");

        assert_eq!(model.ident().to_string(), "SeverityChoice");
        assert_eq!(model.variants()[0].ident().to_string(), "VeryHigh");
        assert_eq!(model.variants()[0].value(), "very_high");
        assert_eq!(model.variants()[1].value(), "low");
    }
}

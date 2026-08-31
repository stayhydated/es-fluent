use super::*;

/// Validated data needed to emit an `EsFluentLabel` implementation and inventory entry.
#[derive(Clone, Debug)]
pub struct EsFluentLabelExpansion {
    ident: syn::Ident,
    generics: syn::Generics,
    ftl_key: SpannedValue<FluentMessageId>,
    domain: Option<crate::semantic::DomainName>,
    label_inventory: MessageModel,
}

impl EsFluentLabelExpansion {
    /// Builds a validated expansion model from the user's derive input.
    pub fn from_derive_input(input: &syn::DeriveInput) -> ExpansionResult<Self> {
        let validated = ValidatedDeriveInput::for_es_fluent_label(input)?;
        let opts = LabelOpts::from_derive_input(validated.input())?;
        let container_context = ContainerContext::from_envelope(validated.required_envelope()?);
        validate_container_domain(&container_context, input.ident.span())?;
        let model = lowered::LabelModel::from_options(&opts)?;

        let original_ident = model.ident();
        let ftl_key = model.message_id().clone();
        let label_inventory = label_inventory_model(
            original_ident,
            *model.type_kind(),
            ftl_key.clone(),
            &opts,
            &container_context,
        )?;

        Ok(Self {
            ident: original_ident.clone(),
            generics: opts.generics().clone(),
            ftl_key,
            domain: container_context.fluent_domain().cloned(),
            label_inventory,
        })
    }

    /// The source type identifier receiving the generated implementation.
    pub fn ident(&self) -> &syn::Ident {
        &self.ident
    }

    /// The source type generics preserved from the user-authored type.
    pub fn generics(&self) -> &syn::Generics {
        &self.generics
    }

    /// The generated label message id.
    pub fn ftl_key(&self) -> &FluentMessageId {
        self.ftl_key.value()
    }

    /// The optional explicit Fluent domain inherited from the parent `#[fluent(...)]`.
    pub fn domain(&self) -> Option<&crate::semantic::DomainName> {
        self.domain.as_ref()
    }

    /// The generated label inventory model.
    pub fn label_inventory(&self) -> &MessageModel {
        &self.label_inventory
    }
}

fn label_inventory_model(
    original_ident: &syn::Ident,
    type_kind: TypeKind,
    ftl_key: SpannedValue<FluentMessageId>,
    opts: &LabelOpts,
    container_context: &ContainerContext,
) -> Result<MessageModel, EsFluentCoreError> {
    let namespace = label_namespace(original_ident, opts, container_context)?;
    let label_entry = MessageEntryModel::new(
        RustSourceName::from_ident(original_ident),
        ftl_key,
        Vec::new(),
        crate::semantic::SourceLocation::new(original_ident.span()),
    );

    Ok(MessageModel::new(
        RustTypeName::from_ident(original_ident),
        type_kind,
        None,
        namespace,
        Vec::new(),
        Some(label_entry),
    ))
}

fn label_namespace(
    original_ident: &syn::Ident,
    opts: &LabelOpts,
    container_context: &ContainerContext,
) -> Result<Option<NamespaceRule>, EsFluentCoreError> {
    let label_namespace = opts.attr_args().namespace().map(|namespace| {
        SpannedNamespaceRuleRef::new(
            namespace,
            opts.attr_args()
                .namespace_span()
                .unwrap_or_else(|| original_ident.span()),
        )
    });
    let namespace = resolve_single_namespace_source([
        NamespaceSource::new(
            "#[fluent(namespace = ...)]",
            AttrContext::MessageContainer,
            container_context
                .fluent_namespace()
                .map(SpannedNamespaceRule::as_ref),
        ),
        NamespaceSource::new(
            "#[fluent_label(namespace = ...)]",
            AttrContext::LabelContainer,
            label_namespace,
        ),
    ])?;

    validate_namespace(
        namespace.map(SpannedNamespaceRuleRef::rule),
        namespace
            .map(SpannedNamespaceRuleRef::span)
            .unwrap_or_else(|| original_ident.span()),
    )?;

    Ok(namespace.map(|namespace| namespace.rule().clone()))
}

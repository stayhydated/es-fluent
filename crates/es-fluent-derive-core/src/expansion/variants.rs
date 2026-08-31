use super::*;

/// One generated variant entry in an `EsFluentVariants` target enum.
#[derive(Clone, Debug)]
pub struct EsFluentGeneratedVariant {
    ident: syn::Ident,
    doc_name: GeneratedDocName,
    message_entry: MessageEntryModel,
}

impl EsFluentGeneratedVariant {
    /// The generated unit variant identifier.
    pub fn ident(&self) -> &syn::Ident {
        &self.ident
    }

    /// The source name used for documentation and FTL default values.
    pub fn doc_name(&self) -> &GeneratedDocName {
        &self.doc_name
    }

    /// The final message metadata for inventory and runtime localization.
    pub fn message_entry(&self) -> &MessageEntryModel {
        &self.message_entry
    }
}

/// One generated enum target from `EsFluentVariants`.
#[derive(Clone, Debug)]
pub struct EsFluentVariantsTarget {
    ident: syn::Ident,
    key_name: Option<GeneratedKeyName>,
    variants: Vec<EsFluentGeneratedVariant>,
    choice: ChoiceModel,
    label_entry: MessageEntryModel,
    generated_model: GeneratedEnumModel,
}

impl EsFluentVariantsTarget {
    /// The generated enum identifier.
    pub fn ident(&self) -> &syn::Ident {
        &self.ident
    }

    /// The optional key name used when `#[fluent_variants(keys = [...])]` is present.
    pub fn key_name(&self) -> Option<&GeneratedKeyName> {
        self.key_name.as_ref()
    }

    /// Generated unit variants and metadata.
    pub fn variants(&self) -> &[EsFluentGeneratedVariant] {
        &self.variants
    }

    /// Inferred selector mapping for the generated unit enum.
    pub fn choice(&self) -> &ChoiceModel {
        &self.choice
    }

    /// Generated label key for the generated enum target.
    pub fn label_key(&self) -> &FluentMessageId {
        self.label_entry.message_id()
    }

    /// Generated label metadata for the generated enum target.
    pub fn label_entry(&self) -> &MessageEntryModel {
        &self.label_entry
    }

    /// Validated semantic model for the generated enum target.
    pub fn generated_model(&self) -> &GeneratedEnumModel {
        &self.generated_model
    }
}

/// Validated data needed to emit generated enums for `EsFluentVariants`.
#[derive(Clone, Debug)]
pub struct EsFluentVariantsExpansion {
    origin_ident: syn::Ident,
    generics: syn::Generics,
    domain: Option<crate::semantic::DomainName>,
    namespace: Option<NamespaceRule>,
    targets: Vec<EsFluentVariantsTarget>,
}

impl EsFluentVariantsExpansion {
    /// Builds a validated expansion model from the user's derive input.
    pub fn from_derive_input(input: &syn::DeriveInput) -> ExpansionResult<Self> {
        let validated = ValidatedDeriveInput::for_es_fluent_variants(input)?;
        let input = validated.input();
        let label_opts = LabelOpts::from_derive_input(input)?;
        let container_context = ContainerContext::from_envelope(validated.required_envelope()?);
        validate_container_domain(&container_context, input.ident.span())?;

        match &input.data {
            Data::Struct(_) => {
                let opts = StructVariantsOpts::from_derive_input(input)?;
                Self::from_struct_options(&container_context, &opts, Some(&label_opts))
            },
            Data::Enum(_) => {
                let opts = EnumVariantsOpts::from_derive_input(input)?;
                Self::from_enum_options(&container_context, &opts, Some(&label_opts))
            },
            Data::Union(_) => unreachable!("union input was rejected above"),
        }
    }

    /// Builds a validated expansion model from parsed struct options.
    pub fn from_struct_options(
        container_context: &ContainerContext,
        opts: &StructVariantsOpts,
        label_opts: Option<&LabelOpts>,
    ) -> ExpansionResult<Self> {
        let model = lowered::GeneratedVariantsStructModel::from_options(opts)?;
        derive_validation::validate_generated_variants_struct_model(&model)?;
        let variant_seeds = build_struct_variant_seeds(&model)?;
        build_variants_expansion(container_context, opts, label_opts, &variant_seeds)
    }

    /// Builds a validated expansion model from parsed enum options.
    pub fn from_enum_options(
        container_context: &ContainerContext,
        opts: &EnumVariantsOpts,
        label_opts: Option<&LabelOpts>,
    ) -> ExpansionResult<Self> {
        let model = lowered::GeneratedVariantsEnumModel::from_options(opts)?;
        derive_validation::validate_generated_variants_enum_model(&model)?;
        let variant_seeds = build_enum_variant_seeds(&model)?;
        build_variants_expansion(container_context, opts, label_opts, &variant_seeds)
    }

    /// The source type identifier.
    pub fn origin_ident(&self) -> &syn::Ident {
        &self.origin_ident
    }

    /// The source type generics.
    pub fn generics(&self) -> &syn::Generics {
        &self.generics
    }

    /// The optional explicit Fluent domain inherited from parent `#[fluent(...)]`.
    pub fn domain(&self) -> Option<&crate::semantic::DomainName> {
        self.domain.as_ref()
    }

    /// The resolved namespace rule for all generated targets.
    pub fn namespace(&self) -> Option<&NamespaceRule> {
        self.namespace.as_ref()
    }

    /// The generated enum targets.
    pub fn targets(&self) -> &[EsFluentVariantsTarget] {
        &self.targets
    }
}

fn build_variants_expansion(
    container_context: &ContainerContext,
    opts: &impl GeneratedVariantsOptions,
    label_opts: Option<&LabelOpts>,
    variant_seeds: &[GeneratedVariantMessageSeed],
) -> ExpansionResult<EsFluentVariantsExpansion> {
    if variant_seeds.is_empty() {
        validate_requested_generated_variants_have_targets(opts)?;
        return Ok(EsFluentVariantsExpansion {
            origin_ident: opts.variants_ident().clone(),
            generics: container_context.generics().clone(),
            domain: container_context.fluent_domain().cloned(),
            namespace: None,
            targets: Vec::new(),
        });
    }

    let namespace = resolved_variants_namespace(
        opts,
        label_opts,
        container_context
            .fluent_namespace()
            .map(SpannedNamespaceRule::as_ref),
    )?;
    validate_namespace(
        namespace.map(SpannedNamespaceRuleRef::rule),
        namespace
            .map(SpannedNamespaceRuleRef::span)
            .unwrap_or_else(|| opts.variants_ident().span()),
    )?;
    let namespace = namespace.map(|namespace| namespace.rule().clone());
    let derives = DerivePathList::for_generated_variants(
        opts.variants_attr_args().derive().iter().cloned(),
        AttrContext::VariantsContainer,
    )?;
    let targets = generated_variants_targets(opts)
        .into_iter()
        .map(|target| {
            let base_key = es_fluent_shared::namer::FluentKey::from(&target.ident);
            let variants = variant_seeds
                .iter()
                .map(|seed| materialize_generated_variant(seed, &base_key))
                .collect::<Result<Vec<_>, _>>()?;
            let choice = ChoiceModel::from_variant_idents(
                &target.ident,
                variants.iter().map(|variant| variant.ident()),
                None,
            )?;
            let label_key = variants_label_key(&base_key, opts.variants_ident().span())?;
            let label_model = MessageEntryModel::new(
                RustSourceName::from_ident(&target.ident),
                SpannedValue::new(label_key, opts.variants_ident().span()),
                Vec::new(),
                crate::semantic::SourceLocation::new(opts.variants_ident().span()),
            );
            let generated_model = GeneratedEnumModel::new(
                RustTypeName::from_ident(&target.ident),
                RustTypeName::from_ident(opts.variants_ident()),
                derives.clone(),
                variants
                    .iter()
                    .map(|variant| variant.message_entry().clone())
                    .collect(),
                Some(label_model.clone()),
                container_context.fluent_domain().cloned(),
                namespace.clone(),
            );

            Ok(EsFluentVariantsTarget {
                ident: target.ident,
                key_name: target.key_name,
                variants,
                choice,
                label_entry: label_model,
                generated_model,
            })
        })
        .collect::<Result<Vec<_>, EsFluentCoreError>>()?;

    Ok(EsFluentVariantsExpansion {
        origin_ident: opts.variants_ident().clone(),
        generics: container_context.generics().clone(),
        domain: container_context.fluent_domain().cloned(),
        namespace,
        targets,
    })
}

fn validate_requested_generated_variants_have_targets(
    opts: &impl GeneratedVariantsOptions,
) -> Result<(), EsFluentCoreError> {
    let mut errors = Vec::new();

    if opts.variants_attr_args().keys().is_some() {
        let mut error = AttrError::new(
            AttrContext::VariantsContainer,
            "`#[fluent_variants(keys = ...)]` requires at least one unskipped field or variant for generated variant enums",
            opts.variants_attr_args()
                .keys_span()
                .or_else(|| Some(opts.variants_ident().span())),
        );
        error.help = Some(
            "remove `keys = [...]`, or leave at least one field or variant without `#[fluent_variants(skip)]`"
                .to_string(),
        );
        errors.push(error);
    }

    match errors.len() {
        0 => Ok(()),
        1 => Err(EsFluentCoreError::StructuredAttributeError(
            errors.into_iter().next().expect("one error"),
        )),
        _ => Err(EsFluentCoreError::StructuredAttributeErrors(errors)),
    }
}

struct GeneratedVariantsTargetSeed {
    ident: syn::Ident,
    key_name: Option<GeneratedKeyName>,
}

fn generated_variants_targets(
    opts: &impl GeneratedVariantsOptions,
) -> Vec<GeneratedVariantsTargetSeed> {
    let Some(keys) = opts.variants_attr_args().keys() else {
        return vec![GeneratedVariantsTargetSeed {
            ident: opts.ftl_enum_ident(),
            key_name: None,
        }];
    };

    keys.iter()
        .map(|key| GeneratedVariantsTargetSeed {
            ident: GeneratedKeyIdent::variants(opts.variants_ident(), key, "Variants").into_ident(),
            key_name: Some(key.value().clone()),
        })
        .collect()
}

fn materialize_generated_variant(
    seed: &GeneratedVariantMessageSeed,
    base_key: &es_fluent_shared::namer::FluentKey,
) -> Result<EsFluentGeneratedVariant, EsFluentCoreError> {
    let message = seed.materialize_message(base_key, AttrContext::VariantsContainer)?;

    Ok(EsFluentGeneratedVariant {
        ident: seed.ident().clone(),
        doc_name: seed.doc_name().clone(),
        message_entry: message,
    })
}

fn variants_label_key(
    base_key: &es_fluent_shared::namer::FluentKey,
    span: proc_macro2::Span,
) -> Result<FluentMessageId, EsFluentCoreError> {
    generated_label_message_value(base_key, span, AttrContext::VariantsContainer)
}

fn build_struct_variant_seeds(
    model: &lowered::GeneratedVariantsStructModel<'_>,
) -> Result<Vec<GeneratedVariantMessageSeed>, EsFluentCoreError> {
    model
        .fields()
        .iter()
        .map(|field| {
            let field_ident = field.ident();
            let original_field_name = es_fluent_shared::namer::rust_ident_name(field_ident);
            let pascal_case_name = original_field_name.to_pascal_case();
            let variant_ident = syn::Ident::new(&pascal_case_name, field_ident.span());
            GeneratedVariantMessageSeed::new(
                variant_ident,
                original_field_name,
                es_fluent_shared::namer::rust_ident_name(field_ident),
                field_ident.span(),
                AttrContext::VariantsField,
            )
        })
        .collect()
}

fn build_enum_variant_seeds(
    model: &lowered::GeneratedVariantsEnumModel<'_>,
) -> Result<Vec<GeneratedVariantMessageSeed>, EsFluentCoreError> {
    model
        .variants()
        .iter()
        .map(|variant| {
            let variant_ident = variant.ident();
            let variant_key = es_fluent_shared::namer::rust_ident_name(variant_ident);
            GeneratedVariantMessageSeed::new(
                variant_ident.clone(),
                variant_key.clone(),
                variant_key,
                variant_ident.span(),
                AttrContext::VariantsVariant,
            )
        })
        .collect()
}

fn resolved_variants_namespace<'a>(
    opts: &'a impl GeneratedVariantsOptions,
    label_opts: Option<&'a LabelOpts>,
    fluent_namespace: Option<SpannedNamespaceRuleRef<'a>>,
) -> Result<Option<SpannedNamespaceRuleRef<'a>>, EsFluentCoreError> {
    let variants_namespace = opts.variants_attr_args().namespace().map(|namespace| {
        SpannedNamespaceRuleRef::new(
            namespace,
            opts.variants_attr_args()
                .namespace_span()
                .unwrap_or_else(|| opts.variants_ident().span()),
        )
    });
    let label_namespace = label_opts.and_then(|opts| {
        opts.attr_args().namespace().map(|namespace| {
            SpannedNamespaceRuleRef::new(
                namespace,
                opts.attr_args()
                    .namespace_span()
                    .unwrap_or_else(|| opts.ident().span()),
            )
        })
    });

    resolve_single_namespace_source([
        NamespaceSource::new(
            "#[fluent(namespace = ...)]",
            AttrContext::MessageContainer,
            fluent_namespace,
        ),
        NamespaceSource::new(
            "#[fluent_variants(namespace = ...)]",
            AttrContext::VariantsContainer,
            variants_namespace,
        ),
        NamespaceSource::new(
            "#[fluent_label(namespace = ...)]",
            AttrContext::LabelContainer,
            label_namespace,
        ),
    ])
}

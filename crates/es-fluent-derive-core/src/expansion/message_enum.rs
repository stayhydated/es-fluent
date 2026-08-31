use super::*;

#[derive(Clone, Copy, Debug)]
pub(super) struct InferredChoiceConfig {
    rename_all: Option<CaseStyle>,
}

impl InferredChoiceConfig {
    fn rename_all(self) -> Option<CaseStyle> {
        self.rename_all
    }
}

pub(super) fn inferred_choice_config(
    input: &syn::DeriveInput,
) -> ExpansionResult<Option<InferredChoiceConfig>> {
    let Data::Enum(data) = &input.data else {
        return Ok(None);
    };

    let has_choice_attr = input
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("fluent_choice"));
    let is_unit_enum = data
        .variants
        .iter()
        .all(|variant| matches!(variant.fields, syn::Fields::Unit));

    if !is_unit_enum {
        if has_choice_attr {
            return Err(syn::Error::new(
                input.ident.span(),
                "#[fluent_choice(...)] can only be used with #[derive(EsFluent)] on unit-only enums",
            )
            .into());
        }

        return Ok(None);
    }

    if data.variants.is_empty() {
        if has_choice_attr {
            return Err(syn::Error::new(
                input.ident.span(),
                "#[fluent_choice(...)] cannot be used with #[derive(EsFluent)] on empty enums",
            )
            .into());
        }

        return Ok(None);
    }

    let choice_opts = ChoiceOpts::from_derive_input(input)?;
    Ok(Some(InferredChoiceConfig {
        rename_all: *choice_opts.attr_args().rename_all(),
    }))
}

/// Validated data needed to emit an `EsFluent` enum implementation.
#[derive(Clone, Debug)]
pub struct EsFluentEnumExpansion {
    ident: syn::Ident,
    generics: syn::Generics,
    domain: Option<crate::semantic::DomainName>,
    is_empty: bool,
    variants: Vec<EsFluentMessageVariant>,
    message_model: MessageModel,
    inferred_choice: Option<ChoiceModel>,
}

impl EsFluentEnumExpansion {
    /// Builds a validated enum expansion model from parsed options.
    pub fn from_options(opts: &EnumOpts) -> ExpansionResult<Self> {
        Self::from_options_with_choice(opts, default_inferred_choice_config(opts))
    }

    /// Builds a validated enum expansion model from parsed options and an
    /// optional inferred selector configuration.
    pub(super) fn from_options_with_choice(
        opts: &EnumOpts,
        inferred_choice: Option<InferredChoiceConfig>,
    ) -> ExpansionResult<Self> {
        let container_context = ContainerContext::from_enum_options(opts);
        derive_validation::validate_enum(opts)?;
        validate_container_namespace(&container_context, opts.ident().span())?;
        validate_container_domain(&container_context, opts.ident().span())?;

        let model = lowered::MessageEnumModel::from_options(opts)?;
        let domain = container_context.fluent_domain().cloned();
        let variants = model
            .variants()
            .iter()
            .map(enum_variant_expansion)
            .collect::<Result<Vec<_>, EsFluentCoreError>>()?;
        let messages = variants
            .iter()
            .filter_map(EsFluentMessageVariant::message_entry)
            .cloned()
            .collect();
        let message_model = MessageModel::new(
            RustTypeName::from_ident(container_context.source_ident()),
            TypeKind::Enum,
            domain.clone(),
            container_context
                .fluent_namespace()
                .map(SpannedNamespaceRule::rule)
                .cloned(),
            messages,
            None,
        );

        Ok(Self {
            ident: container_context.source_ident().clone(),
            generics: container_context.generics().clone(),
            domain,
            is_empty: model.is_empty(),
            variants,
            message_model,
            inferred_choice: inferred_choice_from_options(opts, inferred_choice)?,
        })
    }

    /// The source enum identifier.
    pub fn ident(&self) -> &syn::Ident {
        &self.ident
    }

    /// The source enum generics.
    pub fn generics(&self) -> &syn::Generics {
        &self.generics
    }

    /// Optional explicit Fluent domain.
    pub fn domain(&self) -> Option<&crate::semantic::DomainName> {
        self.domain.as_ref()
    }

    /// Whether the enum has no variants.
    pub fn is_empty(&self) -> bool {
        self.is_empty
    }

    /// Per-variant runtime and message metadata.
    pub fn variants(&self) -> &[EsFluentMessageVariant] {
        &self.variants
    }

    /// The inferred `EsFluentChoice` model for unit enums.
    pub fn inferred_choice(&self) -> Option<&ChoiceModel> {
        self.inferred_choice.as_ref()
    }

    /// The final inventory model.
    pub fn message_model(&self) -> &MessageModel {
        &self.message_model
    }
}

fn inferred_choice_from_options(
    opts: &EnumOpts,
    config: Option<InferredChoiceConfig>,
) -> ExpansionResult<Option<ChoiceModel>> {
    let Some(config) = config else {
        return Ok(None);
    };

    let variants = opts
        .variants()
        .into_iter()
        .map(|variant| ChoiceVariantSource::new(variant.ident(), variant.directive().key()))
        .collect::<Vec<_>>();
    let choice = ChoiceModel::from_variant_sources(opts.ident(), variants, config.rename_all())?;

    Ok(Some(choice))
}

fn default_inferred_choice_config(opts: &EnumOpts) -> Option<InferredChoiceConfig> {
    let variants = opts.variants();
    if variants.is_empty() {
        return None;
    }

    variants
        .iter()
        .all(|variant| matches!(variant.style(), darling::ast::Style::Unit))
        .then_some(InferredChoiceConfig { rename_all: None })
}

/// Runtime and inventory model for one enum variant.
#[derive(Clone, Debug)]
pub enum EsFluentMessageVariant {
    /// Variant delegates to fallback behavior instead of localizing through an FTL key.
    Skipped(EsFluentSkippedVariant),
    /// Variant localizes through a generated FTL key.
    Localized(EsFluentLocalizedVariant),
}

impl EsFluentMessageVariant {
    /// Returns final message metadata for localized variants.
    pub fn message_entry(&self) -> Option<&MessageEntryModel> {
        match self {
            Self::Skipped(_) => None,
            Self::Localized(variant) => Some(variant.message_entry()),
        }
    }
}

/// Fallback data for one skipped enum variant.
#[derive(Clone, Debug)]
pub struct EsFluentSkippedVariant {
    ident: syn::Ident,
    shape: EsFluentEnumVariantShape,
}

impl EsFluentSkippedVariant {
    /// Variant identifier.
    pub fn ident(&self) -> &syn::Ident {
        &self.ident
    }

    /// Variant shape for fallback match-arm emission.
    pub fn shape(&self) -> &EsFluentEnumVariantShape {
        &self.shape
    }
}

/// Localization data for one enum variant.
#[derive(Clone, Debug)]
pub struct EsFluentLocalizedVariant {
    ident: syn::Ident,
    shape: EsFluentEnumVariantShape,
    message_entry: MessageEntryModel,
}

impl EsFluentLocalizedVariant {
    /// Variant identifier.
    pub fn ident(&self) -> &syn::Ident {
        &self.ident
    }

    /// Variant shape for match-arm emission.
    pub fn shape(&self) -> &EsFluentEnumVariantShape {
        &self.shape
    }

    /// Final message metadata.
    pub fn message_entry(&self) -> &MessageEntryModel {
        &self.message_entry
    }
}

/// Enum variant shape needed by token emission.
#[derive(Clone, Debug)]
pub enum EsFluentEnumVariantShape {
    /// Unit variant.
    Unit,
    /// Tuple variant.
    Tuple { fields: Vec<EsFluentTupleField> },
    /// Struct variant.
    Struct {
        fields: Vec<EsFluentNamedField>,
        has_skipped_fields: bool,
    },
}

/// Tuple variant field binding and argument metadata.
#[derive(Clone, Debug)]
pub enum EsFluentTupleField {
    /// A tuple field ignored by generated Fluent arguments.
    Skipped { index: lowered::TupleFieldIndex },
    /// A tuple field that contributes one generated Fluent argument.
    Argument {
        index: lowered::TupleFieldIndex,
        argument: Box<ArgumentModel>,
    },
}

impl EsFluentTupleField {
    /// Original declaration index in the tuple variant.
    pub fn index(&self) -> lowered::TupleFieldIndex {
        match self {
            Self::Skipped { index } | Self::Argument { index, .. } => *index,
        }
    }

    /// Final argument metadata when the field contributes to localization.
    pub fn argument(&self) -> Option<&ArgumentModel> {
        match self {
            Self::Skipped { .. } => None,
            Self::Argument { argument, .. } => Some(argument.as_ref()),
        }
    }
}

/// Struct variant field binding and argument metadata.
#[derive(Clone, Debug)]
pub struct EsFluentNamedField {
    binding: syn::Ident,
    argument: ArgumentModel,
}

impl EsFluentNamedField {
    /// Field binding identifier.
    pub fn binding(&self) -> &syn::Ident {
        &self.binding
    }

    /// Final argument metadata.
    pub fn argument(&self) -> &ArgumentModel {
        &self.argument
    }
}

fn enum_variant_expansion(
    variant: &lowered::MessageEnumVariant<'_>,
) -> Result<EsFluentMessageVariant, EsFluentCoreError> {
    let ident = variant.ident().clone();
    let shape = enum_variant_shape(variant)?;

    if variant.is_skipped() {
        return Ok(EsFluentMessageVariant::Skipped(EsFluentSkippedVariant {
            ident,
            shape,
        }));
    }

    let message_entry = MessageEntryModel::new(
        RustSourceName::from_ident(variant.ident()),
        variant.message_id().clone(),
        enum_variant_arguments(&shape),
        crate::semantic::SourceLocation::new(variant.message_id().span()),
    );

    Ok(EsFluentMessageVariant::Localized(
        EsFluentLocalizedVariant {
            ident,
            shape,
            message_entry,
        },
    ))
}

fn enum_variant_shape(
    variant: &lowered::MessageEnumVariant<'_>,
) -> Result<EsFluentEnumVariantShape, EsFluentCoreError> {
    match variant {
        lowered::MessageEnumVariant::Unit { .. } => Ok(EsFluentEnumVariantShape::Unit),
        lowered::MessageEnumVariant::Tuple { all_fields, .. } => all_fields
            .iter()
            .map(|field| {
                if FluentField::is_skipped(field.field()) {
                    Ok(EsFluentTupleField::Skipped {
                        index: field.original_index(),
                    })
                } else {
                    Ok(EsFluentTupleField::Argument {
                        index: field.original_index(),
                        argument: Box::new(field.argument_model()?),
                    })
                }
            })
            .collect::<Result<Vec<_>, EsFluentCoreError>>()
            .map(|fields| EsFluentEnumVariantShape::Tuple { fields }),
        lowered::MessageEnumVariant::Struct {
            fields,
            has_skipped_fields,
            ..
        } => fields
            .iter()
            .map(|field| {
                Ok(EsFluentNamedField {
                    binding: field.binding().clone(),
                    argument: field.argument_model()?,
                })
            })
            .collect::<Result<Vec<_>, EsFluentCoreError>>()
            .map(|fields| EsFluentEnumVariantShape::Struct {
                fields,
                has_skipped_fields: *has_skipped_fields,
            }),
    }
}

fn enum_variant_arguments(shape: &EsFluentEnumVariantShape) -> Vec<ArgumentModel> {
    match shape {
        EsFluentEnumVariantShape::Unit => Vec::new(),
        EsFluentEnumVariantShape::Tuple { fields } => fields
            .iter()
            .filter_map(|field| field.argument().cloned())
            .collect(),
        EsFluentEnumVariantShape::Struct { fields, .. } => fields
            .iter()
            .map(|field| field.argument().clone())
            .collect(),
    }
}

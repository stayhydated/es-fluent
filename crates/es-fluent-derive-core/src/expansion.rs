//! Validated expansion models consumed by proc-macro token emission.

use darling::FromDeriveInput as _;
use es_fluent_shared::{fluent::FluentMessageId, meta::TypeKind, namespace::NamespaceRule};
use heck::ToPascalCase as _;
use syn::Data;

use crate::{
    context::{ContainerContext, ContainerEnvelope},
    error::{AttrContext, EsFluentCoreError},
    lowered,
    namespace::{SpannedNamespaceRule, SpannedNamespaceRuleRef},
    options::{
        FluentField, GeneratedVariantsOptions,
        choice::ChoiceOpts,
        r#enum::{EnumOpts, EnumVariantsOpts},
        label::LabelOpts,
        r#struct::{StructOpts, StructVariantsOpts},
    },
    semantic::{
        ArgumentModel, ChoiceModel, DerivePathList, GeneratedDocName, GeneratedEnumModel,
        GeneratedKeyIdent, GeneratedKeyName, GeneratedVariantMessageSeed, MessageEntryModel,
        MessageModel, RustSourceName, RustTypeName, SpannedValue, generated_label_message_value,
    },
    validation::{self, NamespaceSource, resolve_single_namespace_source},
};

/// Errors that can occur while building a derive expansion model.
#[derive(Debug, thiserror::Error)]
pub enum ExpansionError {
    /// A structured derive-core validation error.
    #[error(transparent)]
    Core(#[from] EsFluentCoreError),
    /// A `darling` option parsing error.
    #[error(transparent)]
    Darling(#[from] darling::Error),
    /// A `syn` parse or input-shape error.
    #[error(transparent)]
    Syn(#[from] syn::Error),
}

/// A result type for expansion model construction.
pub type ExpansionResult<T> = Result<T, ExpansionError>;

/// Derive surface whose raw input has been grammar-validated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeriveFamily {
    EsFluent,
    EsFluentLabel,
    EsFluentVariants,
    EsFluentChoice,
}

/// A derive input after the raw attribute grammar has been validated once.
#[derive(Clone, Debug)]
pub struct ValidatedDeriveInput<'a> {
    input: &'a syn::DeriveInput,
    family: DeriveFamily,
    envelope: Option<ContainerEnvelope>,
}

impl<'a> ValidatedDeriveInput<'a> {
    pub fn for_es_fluent(input: &'a syn::DeriveInput) -> ExpansionResult<Self> {
        validation::validate_es_fluent_attribute_context(input)?;
        Ok(Self {
            input,
            family: DeriveFamily::EsFluent,
            envelope: None,
        })
    }

    pub fn for_es_fluent_label(input: &'a syn::DeriveInput) -> ExpansionResult<Self> {
        validation::validate_es_fluent_label_attribute_context(input)?;
        let envelope = ContainerEnvelope::from_derive_input(input)?;
        Ok(Self {
            input,
            family: DeriveFamily::EsFluentLabel,
            envelope: Some(envelope),
        })
    }

    pub fn for_es_fluent_variants(input: &'a syn::DeriveInput) -> ExpansionResult<Self> {
        if matches!(&input.data, Data::Union(_)) {
            return Err(syn::Error::new(
                input.ident.span(),
                "EsFluentVariants can only be derived for structs and enums",
            )
            .into());
        }

        validation::validate_es_fluent_variants_attribute_context(input)?;
        let envelope = ContainerEnvelope::from_derive_input(input)?;
        Ok(Self {
            input,
            family: DeriveFamily::EsFluentVariants,
            envelope: Some(envelope),
        })
    }

    pub fn for_es_fluent_choice(input: &'a syn::DeriveInput) -> ExpansionResult<Self> {
        validation::validate_es_fluent_choice_attribute_context(input)?;
        Ok(Self {
            input,
            family: DeriveFamily::EsFluentChoice,
            envelope: None,
        })
    }

    pub fn input(&self) -> &'a syn::DeriveInput {
        self.input
    }

    pub fn family(&self) -> DeriveFamily {
        self.family
    }

    pub fn envelope(&self) -> Option<&ContainerEnvelope> {
        self.envelope.as_ref()
    }

    fn required_envelope(&self) -> ExpansionResult<&ContainerEnvelope> {
        self.envelope.as_ref().ok_or_else(|| {
            syn::Error::new(
                self.input.ident.span(),
                "internal error: validated derive input is missing container context",
            )
            .into()
        })
    }
}

/// Validated data needed to emit an `EsFluent` implementation.
#[derive(Clone, Debug)]
pub enum EsFluentExpansion {
    /// Struct implementation data.
    Struct(EsFluentStructExpansion),
    /// Enum implementation data.
    Enum(EsFluentEnumExpansion),
}

impl EsFluentExpansion {
    /// Builds a validated expansion model from the user's derive input.
    pub fn from_derive_input(input: &syn::DeriveInput) -> ExpansionResult<Self> {
        let input = ValidatedDeriveInput::for_es_fluent(input)?;
        let input = input.input();

        match &input.data {
            Data::Struct(_) => {
                let opts = StructOpts::from_derive_input(input)?;
                Ok(Self::Struct(EsFluentStructExpansion::from_options(&opts)?))
            },
            Data::Enum(_) => {
                let opts = EnumOpts::from_derive_input(input)?;
                Ok(Self::Enum(EsFluentEnumExpansion::from_options(&opts)?))
            },
            Data::Union(_) => Err(syn::Error::new(
                input.ident.span(),
                "EsFluent can only be derived for structs and enums",
            )
            .into()),
        }
    }
}

/// Validated data needed to emit an `EsFluent` struct implementation.
#[derive(Clone, Debug)]
pub struct EsFluentStructExpansion {
    ident: syn::Ident,
    generics: syn::Generics,
    fields: Vec<EsFluentStructField>,
    message_entry: MessageEntryModel,
    message_model: MessageModel,
}

impl EsFluentStructExpansion {
    /// Builds a validated struct expansion model from parsed options.
    pub fn from_options(opts: &StructOpts) -> ExpansionResult<Self> {
        let container_context = ContainerContext::from_struct_options(opts);
        validation::validate_struct(opts)?;
        validate_container_namespace(&container_context, opts.ident().span())?;

        let model = lowered::MessageStructModel::from_options(opts)?;
        let fields = model
            .fields()
            .iter()
            .map(|field| {
                let access = match field {
                    lowered::MessageStructField::Named { binding, .. } => {
                        EsFluentStructFieldAccess::Named((*binding).clone())
                    },
                    lowered::MessageStructField::Tuple {
                        declaration_index, ..
                    } => EsFluentStructFieldAccess::Tuple(*declaration_index),
                };

                Ok(EsFluentStructField {
                    access,
                    argument: field.argument_model()?,
                })
            })
            .collect::<Result<Vec<_>, EsFluentCoreError>>()?;
        let message_entry = MessageEntryModel::new(
            RustSourceName::from_ident(container_context.source_ident()),
            model.message_id().clone(),
            fields
                .iter()
                .map(|field| field.argument().clone())
                .collect(),
            crate::semantic::SourceLocation::new(model.message_id().span()),
        );
        let message_model = MessageModel::new(
            RustTypeName::from_ident(container_context.source_ident()),
            TypeKind::Struct,
            None,
            container_context
                .fluent_namespace()
                .map(SpannedNamespaceRule::rule)
                .cloned(),
            vec![message_entry.clone()],
            None,
        );

        Ok(Self {
            ident: container_context.source_ident().clone(),
            generics: container_context.generics().clone(),
            fields,
            message_entry,
            message_model,
        })
    }

    /// The source struct identifier.
    pub fn ident(&self) -> &syn::Ident {
        &self.ident
    }

    /// The source struct generics.
    pub fn generics(&self) -> &syn::Generics {
        &self.generics
    }

    /// Runtime field bindings and argument metadata.
    pub fn fields(&self) -> &[EsFluentStructField] {
        &self.fields
    }

    /// The final message metadata.
    pub fn message_entry(&self) -> &MessageEntryModel {
        &self.message_entry
    }

    /// The final inventory model.
    pub fn message_model(&self) -> &MessageModel {
        &self.message_model
    }
}

/// Runtime binding and metadata for one struct field argument.
#[derive(Clone, Debug)]
pub struct EsFluentStructField {
    access: EsFluentStructFieldAccess,
    argument: ArgumentModel,
}

impl EsFluentStructField {
    /// How token emission should access the field.
    pub fn access(&self) -> &EsFluentStructFieldAccess {
        &self.access
    }

    /// The final argument metadata.
    pub fn argument(&self) -> &ArgumentModel {
        &self.argument
    }
}

/// Field access strategy for a generated struct implementation.
#[derive(Clone, Debug)]
pub enum EsFluentStructFieldAccess {
    /// Named-field access through `self.name`.
    Named(syn::Ident),
    /// Tuple-field access through `self.N`.
    Tuple(lowered::DeclarationIndex),
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
}

impl EsFluentEnumExpansion {
    /// Builds a validated enum expansion model from parsed options.
    pub fn from_options(opts: &EnumOpts) -> ExpansionResult<Self> {
        let container_context = ContainerContext::from_enum_options(opts);
        validation::validate_enum(opts)?;
        validate_container_namespace(&container_context, opts.ident().span())?;

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

    /// The final inventory model.
    pub fn message_model(&self) -> &MessageModel {
        &self.message_model
    }
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

fn validate_container_namespace(
    container_context: &ContainerContext,
    fallback_span: proc_macro2::Span,
) -> Result<(), EsFluentCoreError> {
    validate_namespace(
        container_context
            .fluent_namespace()
            .map(SpannedNamespaceRule::rule),
        container_context
            .fluent_namespace()
            .map(SpannedNamespaceRule::span)
            .unwrap_or(fallback_span),
    )
}

/// Validated data needed to emit an `EsFluentChoice` implementation.
#[derive(Clone, Debug)]
pub struct EsFluentChoiceExpansion {
    ident: syn::Ident,
    generics: syn::Generics,
    choice: ChoiceModel,
}

impl EsFluentChoiceExpansion {
    /// Builds a validated expansion model from the user's derive input.
    pub fn from_derive_input(input: &syn::DeriveInput) -> ExpansionResult<Self> {
        let input = ValidatedDeriveInput::for_es_fluent_choice(input)?;
        let input = input.input();

        let opts = ChoiceOpts::from_derive_input(input)?;
        let lowered = lowered::ChoiceModel::from_options(&opts)?;
        let enum_ident = lowered.ident();
        let choice = ChoiceModel::from_variant_idents(
            enum_ident,
            lowered.variants().iter().map(|variant| variant.ident()),
            *opts.attr_args().rename_all(),
        )?;

        Ok(Self {
            ident: enum_ident.clone(),
            generics: opts.generics().clone(),
            choice,
        })
    }

    /// The enum identifier receiving the generated implementation.
    pub fn ident(&self) -> &syn::Ident {
        &self.ident
    }

    /// The enum generics preserved from the user-authored type.
    pub fn generics(&self) -> &syn::Generics {
        &self.generics
    }

    /// The final variant-to-choice-value mapping.
    pub fn choice(&self) -> &ChoiceModel {
        &self.choice
    }
}

/// Validated data needed to emit an `EsFluentLabel` implementation and inventory entry.
#[derive(Clone, Debug)]
pub struct EsFluentLabelExpansion {
    ident: syn::Ident,
    generics: syn::Generics,
    ftl_key: Option<SpannedValue<FluentMessageId>>,
    domain: Option<crate::semantic::DomainName>,
    label_inventory: Option<MessageModel>,
}

impl EsFluentLabelExpansion {
    /// Builds a validated expansion model from the user's derive input.
    pub fn from_derive_input(input: &syn::DeriveInput) -> ExpansionResult<Self> {
        let validated = ValidatedDeriveInput::for_es_fluent_label(input)?;
        let opts = LabelOpts::from_derive_input(validated.input())?;
        let container_context = ContainerContext::from_envelope(validated.required_envelope()?);
        let model = lowered::LabelModel::from_options(&opts)?;

        let original_ident = model.ident();
        let ftl_key = opts
            .attr_args()
            .is_origin()
            .then(|| model.message_id().clone());
        let label_inventory = match &ftl_key {
            Some(ftl_key) => Some(label_inventory_model(
                original_ident,
                model.type_kind().clone(),
                ftl_key.clone(),
                &opts,
                &container_context,
            )?),
            None => None,
        };

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

    /// The generated label message id, when `origin = true`.
    pub fn ftl_key(&self) -> Option<&FluentMessageId> {
        self.ftl_key.as_ref().map(SpannedValue::value)
    }

    /// The optional explicit Fluent domain inherited from the parent `#[fluent(...)]`.
    pub fn domain(&self) -> Option<&crate::semantic::DomainName> {
        self.domain.as_ref()
    }

    /// The generated label inventory model, when `origin = true`.
    pub fn label_inventory(&self) -> Option<&MessageModel> {
        self.label_inventory.as_ref()
    }
}

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

    /// Optional generated label key when `#[fluent_label(variants = true)]` is present.
    pub fn label_key(&self) -> Option<&FluentMessageId> {
        self.generated_model
            .label()
            .map(MessageEntryModel::message_id)
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
        validation::validate_generated_variants_struct_model(&model)?;
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
        validation::validate_generated_variants_enum_model(&model)?;
        let variant_seeds = build_enum_variant_seeds(&model)?;
        build_variants_expansion(container_context, opts, label_opts, &variant_seeds)
    }

    /// The source type identifier.
    pub fn origin_ident(&self) -> &syn::Ident {
        &self.origin_ident
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
        return Ok(EsFluentVariantsExpansion {
            origin_ident: opts.variants_ident().clone(),
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
    let derives = DerivePathList::from_paths(
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
            let label_key =
                variants_label_key(label_opts, &base_key, opts.variants_ident().span())?;
            let label_model = label_key.map(|label_key| {
                MessageEntryModel::new(
                    RustSourceName::from_ident(&target.ident),
                    SpannedValue::new(label_key, opts.variants_ident().span()),
                    Vec::new(),
                    crate::semantic::SourceLocation::new(opts.variants_ident().span()),
                )
            });
            let generated_model = GeneratedEnumModel::new(
                RustTypeName::from_ident(&target.ident),
                RustTypeName::from_ident(opts.variants_ident()),
                derives.clone(),
                variants
                    .iter()
                    .map(|variant| variant.message_entry().clone())
                    .collect(),
                label_model,
                container_context.fluent_domain().cloned(),
                namespace.clone(),
            );

            Ok(EsFluentVariantsTarget {
                ident: target.ident,
                key_name: target.key_name,
                variants,
                generated_model,
            })
        })
        .collect::<Result<Vec<_>, EsFluentCoreError>>()?;

    Ok(EsFluentVariantsExpansion {
        origin_ident: opts.variants_ident().clone(),
        domain: container_context.fluent_domain().cloned(),
        namespace,
        targets,
    })
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
    label_opts: Option<&LabelOpts>,
    base_key: &es_fluent_shared::namer::FluentKey,
    span: proc_macro2::Span,
) -> Result<Option<FluentMessageId>, EsFluentCoreError> {
    label_opts
        .filter(|opts| opts.attr_args().is_variants())
        .map(|_| generated_label_message_value(base_key, span, AttrContext::VariantsContainer))
        .transpose()
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

fn validate_namespace(
    namespace: Option<&NamespaceRule>,
    span: proc_macro2::Span,
) -> Result<(), EsFluentCoreError> {
    if let Some(ns) = namespace
        && let Err(error) = validation::validate_namespace(ns, Some(span))
    {
        return Err(error);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        EsFluentChoiceExpansion, EsFluentExpansion, EsFluentLabelExpansion, EsFluentMessageVariant,
        EsFluentVariantsExpansion, ExpansionError, ValidatedDeriveInput,
    };
    use crate::expansion::DeriveFamily;
    use es_fluent_shared::namespace::NamespaceRule;
    use syn::parse_quote;

    #[test]
    fn validated_input_covers_es_fluent_boundary() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "forms")]
            struct Login {
                name: String,
            }
        };

        let validated = ValidatedDeriveInput::for_es_fluent(&input).expect("input should validate");

        assert_eq!(validated.family(), DeriveFamily::EsFluent);
        assert_eq!(validated.input().ident, "Login");
        assert!(validated.envelope().is_none());
    }

    #[test]
    fn validated_input_captures_label_parent_context() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(domain = "shared", namespace = "labels")]
            #[fluent_label(origin = true)]
            enum Status {
                Active,
            }
        };

        let validated =
            ValidatedDeriveInput::for_es_fluent_label(&input).expect("input should validate");
        let envelope = validated.envelope().expect("label captures envelope");

        assert_eq!(validated.family(), DeriveFamily::EsFluentLabel);
        assert_eq!(envelope.source_ident(), "Status");
        assert_eq!(envelope.fluent_domain().expect("domain").as_str(), "shared");
        assert!(matches!(
            envelope.fluent_namespace().map(|namespace| namespace.rule()),
            Some(NamespaceRule::Literal(value)) if value == "labels"
        ));
    }

    #[test]
    fn validated_input_captures_variants_parent_context() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "forms")]
            #[fluent_variants(keys = ["label"])]
            struct Profile {
                name: String,
            }
        };

        let validated =
            ValidatedDeriveInput::for_es_fluent_variants(&input).expect("input should validate");
        let envelope = validated.envelope().expect("variants captures envelope");

        assert_eq!(validated.family(), DeriveFamily::EsFluentVariants);
        assert_eq!(envelope.source_ident(), "Profile");
        assert!(matches!(
            envelope.fluent_namespace().map(|namespace| namespace.rule()),
            Some(NamespaceRule::Literal(value)) if value == "forms"
        ));
    }

    #[test]
    fn validated_input_covers_choice_boundary() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent_choice(rename_all = "snake_case")]
            enum Priority {
                VeryHigh,
            }
        };

        let validated =
            ValidatedDeriveInput::for_es_fluent_choice(&input).expect("input should validate");

        assert_eq!(validated.family(), DeriveFamily::EsFluentChoice);
        assert_eq!(validated.input().ident, "Priority");
        assert!(validated.envelope().is_none());
    }

    #[test]
    fn choice_expansion_builds_validated_choice_model() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent_choice(rename_all = "snake_case")]
            enum Priority<T>
            where
                T: Clone,
            {
                VeryHigh,
                Low,
            }
        };

        let expansion = EsFluentChoiceExpansion::from_derive_input(&input)
            .expect("choice expansion should build");

        assert_eq!(expansion.ident().to_string(), "Priority");
        assert_eq!(
            expansion
                .generics()
                .type_params()
                .map(|param| param.ident.to_string())
                .collect::<Vec<_>>(),
            vec!["T"]
        );
        assert_eq!(expansion.choice().variants()[0].value(), "very_high");
        assert_eq!(expansion.choice().variants()[1].value(), "low");
    }

    #[test]
    fn choice_expansion_reports_darling_shape_errors() {
        let input: syn::DeriveInput = parse_quote! {
            struct NotAnEnum;
        };

        let err = EsFluentChoiceExpansion::from_derive_input(&input)
            .expect_err("struct input should fail");

        assert!(matches!(err, ExpansionError::Darling(_)));
    }

    #[test]
    fn choice_expansion_reports_core_attribute_errors_before_darling() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent_choice(rename_all = 123)]
            enum BadChoice {
                A,
            }
        };

        let err = EsFluentChoiceExpansion::from_derive_input(&input)
            .expect_err("wrong shape should fail");

        assert!(matches!(err, ExpansionError::Core(_)));
    }

    #[test]
    fn es_fluent_struct_expansion_builds_message_and_inventory_model() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "forms")]
            struct LoginForm {
                #[fluent(arg = "display_name")]
                name: String,
                attempts: u16,
            }
        };

        let EsFluentExpansion::Struct(expansion) =
            EsFluentExpansion::from_derive_input(&input).expect("struct expansion")
        else {
            panic!("expected struct expansion");
        };

        assert_eq!(expansion.ident().to_string(), "LoginForm");
        assert_eq!(
            expansion.message_entry().message_id().as_str(),
            "login_form"
        );
        assert_eq!(
            expansion
                .message_entry()
                .argument_names()
                .iter()
                .map(crate::semantic::ArgName::as_str)
                .collect::<Vec<_>>(),
            vec!["display_name", "attempts"]
        );
        assert!(matches!(
            expansion.message_model().namespace(),
            Some(NamespaceRule::Literal(value)) if value == "forms"
        ));
    }

    #[test]
    fn es_fluent_enum_expansion_builds_localized_and_skipped_variants() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(domain = "auth", namespace = "errors")]
            enum LoginError {
                Failed(
                    #[fluent(arg = "display_name")]
                    String,
                    u16,
                ),
                #[fluent(skip)]
                Other(String),
            }
        };

        let EsFluentExpansion::Enum(expansion) =
            EsFluentExpansion::from_derive_input(&input).expect("enum expansion")
        else {
            panic!("expected enum expansion");
        };

        assert_eq!(expansion.ident().to_string(), "LoginError");
        assert_eq!(expansion.domain().expect("domain").as_str(), "auth");
        assert!(matches!(
            expansion.message_model().namespace(),
            Some(NamespaceRule::Literal(value)) if value == "errors"
        ));
        assert_eq!(expansion.variants().len(), 2);
        let EsFluentMessageVariant::Localized(localized) = &expansion.variants()[0] else {
            panic!("first variant should localize");
        };
        assert_eq!(
            localized.message_entry().message_id().as_str(),
            "login_error-Failed"
        );
        assert_eq!(
            localized
                .message_entry()
                .argument_names()
                .iter()
                .map(crate::semantic::ArgName::as_str)
                .collect::<Vec<_>>(),
            vec!["display_name", "f1"]
        );
        assert!(matches!(
            &expansion.variants()[1],
            EsFluentMessageVariant::Skipped(skipped) if skipped.ident() == "Other"
        ));
        assert_eq!(expansion.message_model().messages().len(), 1);
    }

    #[test]
    fn label_expansion_builds_label_impl_and_inventory_model() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "ui")]
            #[fluent_label]
            struct LoginForm<T>(T);
        };

        let expansion = EsFluentLabelExpansion::from_derive_input(&input)
            .expect("label expansion should build");
        let inventory = expansion
            .label_inventory()
            .expect("origin=true builds inventory");

        assert_eq!(expansion.ident().to_string(), "LoginForm");
        assert_eq!(
            expansion.ftl_key().expect("ftl key").as_str(),
            "login_form_label"
        );
        assert_eq!(
            expansion
                .generics()
                .type_params()
                .map(|param| param.ident.to_string())
                .collect::<Vec<_>>(),
            vec!["T"]
        );
        assert!(matches!(
            inventory.namespace(),
            Some(NamespaceRule::Literal(value)) if value == "ui"
        ));
        assert_eq!(
            inventory
                .label()
                .expect("label entry")
                .message_id()
                .as_str(),
            "login_form_label"
        );
    }

    #[test]
    fn label_expansion_skips_inventory_when_origin_is_false() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent_label(origin = false)]
            enum NoOrigin {
                A,
            }
        };

        let expansion = EsFluentLabelExpansion::from_derive_input(&input)
            .expect("label expansion should build");

        assert!(expansion.ftl_key().is_none());
        assert!(expansion.label_inventory().is_none());
    }

    #[test]
    fn label_expansion_rejects_conflicting_namespace_sources() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "parent")]
            #[fluent_label(namespace = "child")]
            struct NamespacedLabel;
        };

        let err = EsFluentLabelExpansion::from_derive_input(&input)
            .expect_err("conflicting namespaces should fail");

        assert!(matches!(err, ExpansionError::Core(_)));
        assert!(
            err.to_string()
                .contains("conflicting namespace declarations")
        );
    }

    #[test]
    fn variants_expansion_builds_keyed_struct_targets() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "ui")]
            #[fluent_variants(keys = ["label", "placeholder"], derive(Debug))]
            struct LoginForm {
                username: String,
                #[fluent_variants(skip)]
                ignored: String,
            }
        };

        let expansion = EsFluentVariantsExpansion::from_derive_input(&input)
            .expect("variants expansion should build");

        assert_eq!(expansion.origin_ident().to_string(), "LoginForm");
        assert!(matches!(
            expansion.namespace(),
            Some(NamespaceRule::Literal(value)) if value == "ui"
        ));
        assert_eq!(expansion.targets().len(), 2);
        assert_eq!(
            expansion.targets()[0].ident().to_string(),
            "LoginFormLabelVariants"
        );
        assert_eq!(
            expansion.targets()[0]
                .key_name()
                .expect("key name")
                .as_str(),
            "label"
        );
        assert_eq!(
            expansion.targets()[0].generated_model().derives().paths()[0]
                .path()
                .segments[0]
                .ident,
            "Debug"
        );
        assert_eq!(expansion.targets()[0].variants().len(), 1);
        assert_eq!(
            expansion.targets()[0].variants()[0]
                .message_entry()
                .message_id()
                .as_str(),
            "login_form_label_variants-username"
        );
    }

    #[test]
    fn variants_expansion_builds_enum_label_key_and_domain() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(domain = "es-fluent-lang", namespace = "languages")]
            #[fluent_label(variants = true)]
            enum Language {
                English,
                French,
            }
        };

        let expansion = EsFluentVariantsExpansion::from_derive_input(&input)
            .expect("variants expansion should build");
        let target = expansion.targets().first().expect("target");

        assert_eq!(
            expansion.domain().expect("domain").as_str(),
            "es-fluent-lang"
        );
        assert_eq!(
            target.label_key().expect("label key").as_str(),
            "language_variants_label"
        );
        assert_eq!(
            target.variants()[0].message_entry().message_id().as_str(),
            "language_variants-English"
        );
        assert_eq!(
            target.variants()[1].message_entry().message_id().as_str(),
            "language_variants-French"
        );
    }

    #[test]
    fn variants_expansion_rejects_conflicting_namespace_sources() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "parent_ns")]
            #[fluent_variants(namespace = "variant_ns")]
            #[fluent_label(variants = true, namespace = "label_ns")]
            struct NamespaceHolder {
                field: String,
            }
        };

        let err = EsFluentVariantsExpansion::from_derive_input(&input)
            .expect_err("conflicting namespaces should fail");

        assert!(matches!(err, ExpansionError::Core(_)));
        assert!(
            err.to_string()
                .contains("conflicting namespace declarations")
        );
    }
}

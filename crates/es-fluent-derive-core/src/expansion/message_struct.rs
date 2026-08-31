use super::*;

/// Validated data needed to emit an `EsFluent` struct implementation.
#[derive(Clone, Debug)]
pub struct EsFluentStructExpansion {
    ident: syn::Ident,
    generics: syn::Generics,
    domain: Option<crate::semantic::DomainName>,
    fields: Vec<EsFluentStructField>,
    message_entry: MessageEntryModel,
    message_model: MessageModel,
}

impl EsFluentStructExpansion {
    /// Builds a validated struct expansion model from parsed options.
    pub fn from_options(opts: &StructOpts) -> ExpansionResult<Self> {
        let container_context = ContainerContext::from_struct_options(opts);
        derive_validation::validate_struct(opts)?;
        validate_container_namespace(&container_context, opts.ident().span())?;
        validate_container_domain(&container_context, opts.ident().span())?;

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
            container_context.fluent_domain().cloned(),
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
            domain: container_context.fluent_domain().cloned(),
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

    /// Optional explicit package-local Fluent domain.
    pub fn domain(&self) -> Option<&crate::semantic::DomainName> {
        self.domain.as_ref()
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

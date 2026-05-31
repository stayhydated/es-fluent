//! Lowered container models for parsed derive options.

use crate::{
    error::{AttrContext, AttrError, EsFluentCoreError, EsFluentCoreResult},
    options::{
        EnumDataOptions as _, FilteredEnumDataOptions as _, FluentField, Skippable as _,
        StructDataOptions as _, VariantFields as _,
        choice::ChoiceOpts,
        r#enum::{EnumOpts, EnumVariantsOpts, VariantOpts},
        label::LabelOpts,
        r#struct::{StructFieldOpts, StructOpts, StructVariantsOpts},
    },
};
use es_fluent_shared::meta::TypeKind;

#[derive(Clone, Copy, Debug)]
pub struct MessageStructModel<'a> {
    ident: &'a syn::Ident,
    fields: &'a darling::ast::Fields<StructFieldOpts>,
}

impl<'a> MessageStructModel<'a> {
    pub fn from_options(opts: &'a StructOpts) -> EsFluentCoreResult<Self> {
        let darling::ast::Data::Struct(fields) = opts.struct_data() else {
            return Err(internal_shape_error(
                AttrContext::MessageContainer,
                "EsFluent struct options must contain struct data",
                opts.ident().span(),
            ));
        };

        Ok(Self {
            ident: opts.ident(),
            fields,
        })
    }

    pub fn ident(&self) -> &'a syn::Ident {
        self.ident
    }

    pub fn fields(&self) -> Vec<MessageStructField<'a>> {
        self.fields
            .fields
            .iter()
            .enumerate()
            .filter(|(_, field)| !FluentField::is_skipped(*field))
            .map(|(declaration_index, field)| {
                if let Some(binding) = field.ident() {
                    MessageStructField::Named {
                        binding,
                        declaration_index,
                        field,
                    }
                } else {
                    MessageStructField::Tuple {
                        declaration_index,
                        field,
                    }
                }
            })
            .collect()
    }

    pub fn indexed_fields(&self) -> Vec<(usize, &'a StructFieldOpts)> {
        self.fields
            .fields
            .iter()
            .enumerate()
            .filter(|(_, field)| !FluentField::is_skipped(*field))
            .collect()
    }

    pub fn all_indexed_fields(&self) -> Vec<(usize, &'a StructFieldOpts)> {
        self.fields.fields.iter().enumerate().collect()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum MessageStructField<'a> {
    Named {
        binding: &'a syn::Ident,
        declaration_index: usize,
        field: &'a StructFieldOpts,
    },
    Tuple {
        declaration_index: usize,
        field: &'a StructFieldOpts,
    },
}

impl MessageStructField<'_> {
    pub fn declaration_index(&self) -> usize {
        match self {
            Self::Named {
                declaration_index, ..
            }
            | Self::Tuple {
                declaration_index, ..
            } => *declaration_index,
        }
    }

    pub fn field(&self) -> &StructFieldOpts {
        match self {
            Self::Named { field, .. } | Self::Tuple { field, .. } => field,
        }
    }

    pub fn binding(&self) -> Option<&syn::Ident> {
        match self {
            Self::Named { binding, .. } => Some(binding),
            Self::Tuple { .. } => None,
        }
    }
}

#[derive(Debug)]
pub struct MessageEnumModel<'a> {
    ident: &'a syn::Ident,
    variants: Vec<MessageEnumVariant<'a>>,
}

impl<'a> MessageEnumModel<'a> {
    pub fn from_options(opts: &'a EnumOpts) -> EsFluentCoreResult<Self> {
        let darling::ast::Data::Enum(variants) = opts.enum_data() else {
            return Err(internal_shape_error(
                AttrContext::MessageContainer,
                "EsFluent enum options must contain enum data",
                opts.ident().span(),
            ));
        };

        let variants = variants
            .iter()
            .map(MessageEnumVariant::from_options)
            .collect::<EsFluentCoreResult<Vec<_>>>()?;

        Ok(Self {
            ident: opts.ident(),
            variants,
        })
    }

    pub fn ident(&self) -> &'a syn::Ident {
        self.ident
    }

    pub fn variants(&self) -> &[MessageEnumVariant<'a>] {
        &self.variants
    }

    pub fn is_empty(&self) -> bool {
        self.variants.is_empty()
    }
}

#[derive(Debug)]
pub enum MessageEnumVariant<'a> {
    Unit {
        ident: &'a syn::Ident,
        skipped: bool,
        opts: &'a VariantOpts,
    },
    Tuple {
        ident: &'a syn::Ident,
        skipped: bool,
        opts: &'a VariantOpts,
        fields: Vec<MessageTupleField<'a>>,
    },
    Struct {
        ident: &'a syn::Ident,
        skipped: bool,
        opts: &'a VariantOpts,
        fields: Vec<MessageNamedField<'a>>,
        all_fields: Vec<MessageNamedField<'a>>,
        has_skipped_fields: bool,
    },
}

impl<'a> MessageEnumVariant<'a> {
    fn from_options(variant_opt: &'a VariantOpts) -> EsFluentCoreResult<Self> {
        let ident = variant_opt.ident();
        let skipped = variant_opt.is_skipped();

        match variant_opt.style() {
            darling::ast::Style::Unit => Ok(Self::Unit {
                ident,
                skipped,
                opts: variant_opt,
            }),
            darling::ast::Style::Tuple => Ok(Self::Tuple {
                ident,
                skipped,
                opts: variant_opt,
                fields: variant_opt
                    .all_fields()
                    .into_iter()
                    .enumerate()
                    .map(|(original_index, field)| MessageTupleField {
                        original_index,
                        field,
                    })
                    .collect(),
            }),
            darling::ast::Style::Struct => {
                let fields = variant_opt
                    .fields()
                    .into_iter()
                    .enumerate()
                    .map(|(exposed_index, field)| {
                        let Some(binding) = field.ident() else {
                            return Err(internal_shape_error(
                                AttrContext::EnumVariant,
                                "struct variant field is missing an identifier",
                                ident.span(),
                            ));
                        };
                        Ok(MessageNamedField {
                            binding,
                            exposed_index,
                            field,
                        })
                    })
                    .collect::<EsFluentCoreResult<Vec<_>>>()?;
                let all_fields = variant_opt
                    .all_fields()
                    .into_iter()
                    .enumerate()
                    .map(|(declaration_index, field)| {
                        let Some(binding) = field.ident() else {
                            return Err(internal_shape_error(
                                AttrContext::EnumVariant,
                                "struct variant field is missing an identifier",
                                ident.span(),
                            ));
                        };
                        Ok(MessageNamedField {
                            binding,
                            exposed_index: declaration_index,
                            field,
                        })
                    })
                    .collect::<EsFluentCoreResult<Vec<_>>>()?;
                let has_skipped_fields = all_fields.len() > fields.len();

                Ok(Self::Struct {
                    ident,
                    skipped,
                    opts: variant_opt,
                    fields,
                    all_fields,
                    has_skipped_fields,
                })
            },
        }
    }

    pub fn ident(&self) -> &'a syn::Ident {
        match self {
            Self::Unit { ident, .. } | Self::Tuple { ident, .. } | Self::Struct { ident, .. } => {
                ident
            },
        }
    }

    pub fn opts(&self) -> &'a VariantOpts {
        match self {
            Self::Unit { opts, .. } | Self::Tuple { opts, .. } | Self::Struct { opts, .. } => opts,
        }
    }

    pub fn all_fields(&self) -> Vec<MessageEnumField<'a>> {
        match self {
            Self::Unit { .. } => Vec::new(),
            Self::Tuple { fields, .. } => fields
                .iter()
                .copied()
                .map(MessageEnumField::Tuple)
                .collect(),
            Self::Struct { all_fields, .. } => all_fields
                .iter()
                .copied()
                .map(MessageEnumField::Named)
                .collect(),
        }
    }

    pub fn is_skipped(&self) -> bool {
        match self {
            Self::Unit { skipped, .. }
            | Self::Tuple { skipped, .. }
            | Self::Struct { skipped, .. } => *skipped,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MessageTupleField<'a> {
    pub original_index: usize,
    pub field: &'a crate::options::FluentFieldOpts,
}

#[derive(Clone, Copy, Debug)]
pub struct MessageNamedField<'a> {
    pub binding: &'a syn::Ident,
    pub exposed_index: usize,
    pub field: &'a crate::options::FluentFieldOpts,
}

#[derive(Clone, Copy, Debug)]
pub enum MessageEnumField<'a> {
    Tuple(MessageTupleField<'a>),
    Named(MessageNamedField<'a>),
}

impl MessageEnumField<'_> {
    pub fn declaration_index(&self) -> usize {
        match self {
            Self::Tuple(field) => field.original_index,
            Self::Named(field) => field.exposed_index,
        }
    }

    pub fn field(&self) -> &crate::options::FluentFieldOpts {
        match self {
            Self::Tuple(field) => field.field,
            Self::Named(field) => field.field,
        }
    }
}

#[derive(Debug)]
pub struct GeneratedVariantsStructModel<'a> {
    ident: &'a syn::Ident,
    fields: Vec<GeneratedVariantsField<'a>>,
}

impl<'a> GeneratedVariantsStructModel<'a> {
    pub fn from_options(opts: &'a StructVariantsOpts) -> EsFluentCoreResult<Self> {
        let darling::ast::Data::Struct(fields) = opts.struct_data() else {
            return Err(internal_shape_error(
                AttrContext::VariantsContainer,
                "EsFluentVariants struct options must contain struct data",
                opts.ident().span(),
            ));
        };

        let fields = fields
            .fields
            .iter()
            .filter(|field| !field.is_skipped())
            .map(|field| {
                let Some(ident) = field.ident() else {
                    return Err(internal_shape_error(
                        AttrContext::VariantsField,
                        "generated struct variant field is missing an identifier",
                        opts.ident().span(),
                    ));
                };
                Ok(GeneratedVariantsField { ident })
            })
            .collect::<EsFluentCoreResult<Vec<_>>>()?;

        Ok(Self {
            ident: opts.ident(),
            fields,
        })
    }

    pub fn ident(&self) -> &'a syn::Ident {
        self.ident
    }

    pub fn fields(&self) -> &[GeneratedVariantsField<'a>] {
        &self.fields
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GeneratedVariantsField<'a> {
    pub ident: &'a syn::Ident,
}

#[derive(Debug)]
pub struct GeneratedVariantsEnumModel<'a> {
    ident: &'a syn::Ident,
    variants: Vec<GeneratedVariantsVariant<'a>>,
}

impl<'a> GeneratedVariantsEnumModel<'a> {
    pub fn from_options(opts: &'a EnumVariantsOpts) -> EsFluentCoreResult<Self> {
        let darling::ast::Data::Enum(variants) = opts.enum_data() else {
            return Err(internal_shape_error(
                AttrContext::VariantsContainer,
                "EsFluentVariants enum options must contain enum data",
                opts.ident().span(),
            ));
        };

        let variants = variants
            .iter()
            .filter(|variant| !variant.is_skipped())
            .map(|variant| GeneratedVariantsVariant {
                ident: variant.ident(),
            })
            .collect();

        Ok(Self {
            ident: opts.ident(),
            variants,
        })
    }

    pub fn ident(&self) -> &'a syn::Ident {
        self.ident
    }

    pub fn variants(&self) -> &[GeneratedVariantsVariant<'a>] {
        &self.variants
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GeneratedVariantsVariant<'a> {
    pub ident: &'a syn::Ident,
}

#[derive(Debug)]
pub struct LabelModel<'a> {
    ident: &'a syn::Ident,
    type_kind: TypeKind,
}

impl<'a> LabelModel<'a> {
    pub fn from_options(opts: &'a LabelOpts) -> EsFluentCoreResult<Self> {
        let type_kind = match opts.data() {
            darling::ast::Data::Struct(_) => TypeKind::Struct,
            darling::ast::Data::Enum(_) => TypeKind::Enum,
        };

        Ok(Self {
            ident: opts.ident(),
            type_kind,
        })
    }

    pub fn ident(&self) -> &'a syn::Ident {
        self.ident
    }

    pub fn type_kind(&self) -> &TypeKind {
        &self.type_kind
    }
}

#[derive(Debug)]
pub struct ChoiceModel<'a> {
    ident: &'a syn::Ident,
    variants: Vec<ChoiceVariant<'a>>,
}

impl<'a> ChoiceModel<'a> {
    pub fn from_options(opts: &'a ChoiceOpts) -> EsFluentCoreResult<Self> {
        let darling::ast::Data::Enum(variants) = opts.data() else {
            return Err(internal_shape_error(
                AttrContext::ChoiceContainer,
                "EsFluentChoice options must contain enum data",
                opts.ident().span(),
            ));
        };

        let variants = variants
            .iter()
            .map(|variant| {
                if !matches!(variant.fields, syn::Fields::Unit) {
                    return Err(internal_shape_error(
                        AttrContext::ChoiceContainer,
                        "EsFluentChoice variants must be unit variants",
                        variant.ident.span(),
                    ));
                }

                Ok(ChoiceVariant {
                    ident: &variant.ident,
                })
            })
            .collect::<EsFluentCoreResult<Vec<_>>>()?;

        Ok(Self {
            ident: opts.ident(),
            variants,
        })
    }

    pub fn ident(&self) -> &'a syn::Ident {
        self.ident
    }

    pub fn variants(&self) -> &[ChoiceVariant<'a>] {
        &self.variants
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ChoiceVariant<'a> {
    pub ident: &'a syn::Ident,
}

fn internal_shape_error(
    context: AttrContext,
    message: impl Into<String>,
    span: proc_macro2::Span,
) -> EsFluentCoreError {
    EsFluentCoreError::StructuredAttributeError(AttrError::new(context, message, Some(span)))
}

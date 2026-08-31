use crate::error::{AttrContext, AttrError, EsFluentCoreError, EsFluentCoreResult};
use crate::index::FieldArgumentIndex;
use crate::semantic::{
    ArgName, ArgumentValueStrategy, SpannedValue, ValueTransform, parse_arg_name_in_context,
};
use bon::Builder;
use darling::{FromField, FromMeta};
use es_fluent_shared::namer;
use getset::Getters;
use syn::spanned::Spanned as _;

use super::{GeneratedVariantDirective, PresentFlag, SkipDirective, Skippable, ValueAttr};

/// Shared behavior for fields that expose Fluent arguments.
pub trait FluentField {
    /// Returns the source field identifier when present.
    fn ident(&self) -> Option<&syn::Ident>;
    /// Returns the source field type.
    fn ty(&self) -> &syn::Type;
    /// Returns the closed field directive built from the raw field attributes.
    fn directive(&self) -> &FieldDirective;

    /// Returns `true` if the field should be skipped.
    fn is_skipped(&self) -> bool {
        matches!(self.directive(), FieldDirective::Skip)
    }

    /// Returns the argument value strategy for fields that expose an argument.
    fn argument_value_strategy(&self, span: proc_macro2::Span) -> Option<ArgumentValueStrategy> {
        self.directive().argument_value_strategy(span)
    }

    /// Returns the explicit field argument name as a typed value if provided.
    fn arg_name(&self) -> Option<&SpannedValue<ArgName>> {
        self.directive().arg_name()
    }

    /// Resolves and validates the Fluent argument name for this field.
    fn fluent_arg_name(
        &self,
        index: impl FieldArgumentIndex,
        context: AttrContext,
    ) -> EsFluentCoreResult<SpannedValue<ArgName>> {
        if let Some(arg) = self.arg_name() {
            return Ok(arg.clone());
        }

        let index = index.argument_index();
        let (name, span) = self
            .ident()
            .map(|ident| (namer::rust_ident_name(ident), ident.span()))
            .unwrap_or_else(|| {
                (
                    namer::UnnamedItem::from(index).to_string(),
                    proc_macro2::Span::call_site(),
                )
            });
        let name = parse_arg_name_in_context(name, span, context)?;
        Ok(SpannedValue::new(name, span))
    }
}

impl SkipDirective for FieldDirective {
    fn is_skipped(&self) -> bool {
        matches!(self, Self::Skip)
    }
}

impl<T: FluentField> Skippable for T {
    type Directive = FieldDirective;

    fn skip_directive(&self) -> &Self::Directive {
        FluentField::directive(self)
    }
}

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
struct SkippableFieldAttributeArgs {
    /// Whether to skip this field.
    #[darling(default)]
    skip: Option<PresentFlag>,
}

impl SkippableFieldAttributeArgs {
    fn directive(&self) -> GeneratedVariantDirective {
        if self.skip.is_some_and(PresentFlag::is_present) {
            GeneratedVariantDirective::Skip
        } else {
            GeneratedVariantDirective::Include
        }
    }
}

#[derive(Clone, Debug)]
pub struct SkippableFieldOpts {
    /// The identifier of the field.
    ident: Option<syn::Ident>,
    /// The type of the field.
    ty: syn::Type,
    directive: GeneratedVariantDirective,
}

#[derive(Clone, Debug, FromField)]
#[darling(attributes(fluent_variants))]
struct RawSkippableFieldOpts {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    #[darling(flatten)]
    attr_args: SkippableFieldAttributeArgs,
}

impl FromField for SkippableFieldOpts {
    fn from_field(field: &syn::Field) -> darling::Result<Self> {
        let raw = RawSkippableFieldOpts::from_field(field)?;
        Ok(Self {
            ident: raw.ident,
            ty: raw.ty,
            directive: raw.attr_args.directive(),
        })
    }
}

impl SkippableFieldOpts {
    pub fn ident(&self) -> Option<&syn::Ident> {
        self.ident.as_ref()
    }

    pub fn ty(&self) -> &syn::Type {
        &self.ty
    }

    pub fn directive(&self) -> &GeneratedVariantDirective {
        &self.directive
    }
}

impl Skippable for SkippableFieldOpts {
    type Directive = GeneratedVariantDirective;

    fn skip_directive(&self) -> &Self::Directive {
        &self.directive
    }
}

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub(crate) struct FluentFieldAttributeArgs {
    /// Whether to skip this field.
    #[darling(default)]
    skip: Option<PresentFlag>,
    /// Whether this field is a selector for a Fluent select expression.
    #[darling(default)]
    selector: Option<PresentFlag>,
    /// A value transformation expression.
    #[darling(default)]
    value: Option<ValueAttr>,
    /// Optional argument name override.
    #[darling(default)]
    arg: Option<SpannedValue<ArgName>>,
}

impl FluentFieldAttributeArgs {
    fn is_skipped(&self) -> bool {
        self.skip.is_some_and(PresentFlag::is_present)
    }

    fn is_selector(&self) -> bool {
        self.selector.is_some_and(PresentFlag::is_present)
    }

    fn value(&self) -> Option<&syn::Expr> {
        self.value.as_ref().map(|value| &value.0)
    }

    fn directive(
        &self,
        ty: &syn::Type,
        span: proc_macro2::Span,
    ) -> EsFluentCoreResult<FieldDirective> {
        let is_skipped = self.is_skipped();
        let is_selector = self.is_selector();
        let has_value = self.value().is_some();
        let has_arg = self.arg.is_some();

        if is_skipped {
            if has_arg {
                return Err(field_strategy_error(
                    "Cannot use #[fluent(arg = \"...\")] on a skipped field",
                    span,
                ));
            }
            if is_selector {
                return Err(field_strategy_error(
                    "Cannot use #[fluent(selector)] on a skipped field",
                    span,
                ));
            }
            if has_value {
                return Err(field_strategy_error(
                    "Cannot use #[fluent(value = ...)] on a skipped field",
                    span,
                ));
            }

            return Ok(FieldDirective::Skip);
        }

        if is_selector && has_value {
            return Err(field_strategy_error(
                "Cannot combine #[fluent(selector)] and #[fluent(value = ...)] on the same field",
                span,
            ));
        }

        if is_selector {
            if let Some(inner_ty) = option_inner_type(ty) {
                return Ok(FieldDirective::Argument(Box::new(FieldArgumentDirective {
                    name: self.arg.clone(),
                    value: FieldValueDirective::OptionalChoice {
                        span: ty.span(),
                        inner_ty: inner_ty.clone(),
                    },
                })));
            }

            return Ok(FieldDirective::Argument(Box::new(FieldArgumentDirective {
                name: self.arg.clone(),
                value: FieldValueDirective::Choice {
                    span,
                    ty: ty.clone(),
                },
            })));
        }

        if let Some(expr) = self.value() {
            return Ok(FieldDirective::Argument(Box::new(FieldArgumentDirective {
                name: self.arg.clone(),
                value: FieldValueDirective::Transform(ValueTransform::new(
                    expr.clone(),
                    expr.span(),
                )),
            })));
        }

        if let Some(inner_ty) = option_inner_type(ty) {
            return Ok(FieldDirective::Argument(Box::new(FieldArgumentDirective {
                name: self.arg.clone(),
                value: FieldValueDirective::Optional {
                    span: ty.span(),
                    inner_ty: inner_ty.clone(),
                },
            })));
        }

        Ok(FieldDirective::Argument(Box::new(FieldArgumentDirective {
            name: self.arg.clone(),
            value: FieldValueDirective::Borrowed { span },
        })))
    }
}

/// Closed representation of a field's message-argument behavior.
#[derive(Clone, Debug)]
pub enum FieldDirective {
    /// The field is ignored by generated Fluent arguments.
    Skip,
    /// The field contributes one generated Fluent argument.
    Argument(Box<FieldArgumentDirective>),
}

impl FieldDirective {
    pub(super) fn from_attr_args(
        attr_args: &FluentFieldAttributeArgs,
        ty: &syn::Type,
        span: proc_macro2::Span,
    ) -> EsFluentCoreResult<Self> {
        attr_args.directive(ty, span)
    }

    pub fn argument(&self) -> Option<&FieldArgumentDirective> {
        match self {
            Self::Skip => None,
            Self::Argument(argument) => Some(argument.as_ref()),
        }
    }

    pub fn arg_name(&self) -> Option<&SpannedValue<ArgName>> {
        self.argument().and_then(FieldArgumentDirective::name)
    }

    pub fn argument_value_strategy(
        &self,
        fallback_span: proc_macro2::Span,
    ) -> Option<ArgumentValueStrategy> {
        self.argument()
            .map(|argument| argument.value().argument_value_strategy(fallback_span))
    }
}

/// Argument metadata for a field that contributes to a generated Fluent call.
#[derive(Clone, Debug)]
pub struct FieldArgumentDirective {
    name: Option<SpannedValue<ArgName>>,
    value: FieldValueDirective,
}

impl FieldArgumentDirective {
    pub fn name(&self) -> Option<&SpannedValue<ArgName>> {
        self.name.as_ref()
    }

    pub fn value(&self) -> &FieldValueDirective {
        &self.value
    }
}

/// Value handling strategy selected by field attributes.
#[derive(Clone, Debug)]
pub enum FieldValueDirective {
    /// Borrow the field value and let runtime autoref dispatch choose the final value form.
    Borrowed { span: proc_macro2::Span },
    /// Treat the field value as an `Option<T>`.
    Optional {
        span: proc_macro2::Span,
        inner_ty: syn::Type,
    },
    /// Convert the field value through `EsFluentChoice`.
    Choice {
        span: proc_macro2::Span,
        ty: syn::Type,
    },
    /// Convert an optional field value through `EsFluentChoice`, preserving `None`.
    OptionalChoice {
        span: proc_macro2::Span,
        inner_ty: syn::Type,
    },
    /// Apply an explicit field-level transform expression.
    Transform(ValueTransform),
}

impl FieldValueDirective {
    pub fn argument_value_strategy(
        &self,
        _fallback_span: proc_macro2::Span,
    ) -> ArgumentValueStrategy {
        match self {
            Self::Borrowed { span } => ArgumentValueStrategy::Borrowed { span: *span },
            Self::Optional { span, .. } => ArgumentValueStrategy::Optional { span: *span },
            Self::Choice { span, ty } => ArgumentValueStrategy::Choice {
                span: *span,
                ty: Box::new(ty.clone()),
            },
            Self::OptionalChoice { span, inner_ty } => ArgumentValueStrategy::OptionalChoice {
                span: *span,
                ty: Box::new(inner_ty.clone()),
            },
            Self::Transform(transform) => {
                ArgumentValueStrategy::Transform(Box::new(transform.clone()))
            },
        }
    }

    pub fn optional_inner_ty(&self) -> Option<&syn::Type> {
        match self {
            Self::Optional { inner_ty, .. } => Some(inner_ty),
            _ => None,
        }
    }
}

fn field_strategy_error(message: impl Into<String>, span: proc_macro2::Span) -> EsFluentCoreError {
    EsFluentCoreError::StructuredAttributeError(AttrError::new(
        AttrContext::MessageField,
        message,
        Some(span),
    ))
}

fn option_inner_type(ty: &syn::Type) -> Option<&syn::Type> {
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path
        .path
        .segments
        .last()
        .filter(|segment| segment.ident == "Option")?;
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    arguments.args.iter().find_map(|arg| match arg {
        syn::GenericArgument::Type(ty) => Some(ty),
        _ => None,
    })
}

#[derive(Clone, Debug)]
pub struct FluentFieldOpts {
    /// The identifier of the field.
    ident: Option<syn::Ident>,
    /// The type of the field.
    ty: syn::Type,
    directive: FieldDirective,
}

#[derive(Clone, Debug, FromField)]
#[darling(attributes(fluent))]
struct RawFluentFieldOpts {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    #[darling(flatten)]
    attr_args: FluentFieldAttributeArgs,
}

impl FromField for FluentFieldOpts {
    fn from_field(field: &syn::Field) -> darling::Result<Self> {
        let raw = RawFluentFieldOpts::from_field(field)?;
        let span = raw
            .ident
            .as_ref()
            .map_or_else(|| raw.ty.span(), syn::Ident::span);
        let directive = FieldDirective::from_attr_args(&raw.attr_args, &raw.ty, span)
            .map_err(|error| darling::Error::custom(error.to_string()).with_span(field))?;
        Ok(Self {
            ident: raw.ident,
            ty: raw.ty,
            directive,
        })
    }
}

impl FluentFieldOpts {
    pub fn ident(&self) -> Option<&syn::Ident> {
        self.ident.as_ref()
    }

    pub fn ty(&self) -> &syn::Type {
        &self.ty
    }

    pub fn directive(&self) -> &FieldDirective {
        &self.directive
    }
}

impl FluentField for FluentFieldOpts {
    fn ident(&self) -> Option<&syn::Ident> {
        self.ident.as_ref()
    }

    fn ty(&self) -> &syn::Type {
        &self.ty
    }

    fn directive(&self) -> &FieldDirective {
        &self.directive
    }
}

use proc_macro2::Span;

use super::{ArgName, SpannedValue};

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
    Choice { span: Span, ty: Box<syn::Type> },
    /// Convert an optional field value through `EsFluentChoice`.
    OptionalChoice { span: Span, ty: Box<syn::Type> },
    /// Apply an explicit field-level transform expression.
    Transform(Box<ValueTransform>),
}

impl ArgumentValueStrategy {
    pub fn span(&self) -> Span {
        match self {
            Self::Borrowed { span }
            | Self::Optional { span }
            | Self::Choice { span, .. }
            | Self::OptionalChoice { span, .. } => *span,
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

//! This module provides types for parsing `es-fluent` attributes.

use crate::error::{AttrContext, AttrError, EsFluentCoreError, EsFluentCoreResult};
use crate::index::{DeclarationIndex, FieldArgumentIndex};
use crate::namespace::SpannedNamespaceRule;
use crate::semantic::{
    ArgName, ArgumentValueStrategy, DomainName, FluentMessageId, GeneratedKeyIdent,
    GeneratedKeyName, SpannedValue, ValueTransform, VariantKey, parse_arg_name_in_context,
    parse_domain_name_in_context, parse_fluent_message_id_in_context, parse_variant_key_in_context,
};
use bon::Builder;
use darling::{FromField, FromMeta};
use es_fluent_shared::{namer, namespace::NamespaceRule};
use getset::Getters;
use syn::spanned::Spanned as _;

pub mod choice;
pub mod r#enum;
pub mod label;
pub mod r#struct;

fn string_literal_value(item: &syn::Meta) -> darling::Result<(String, proc_macro2::Span)> {
    match item {
        syn::Meta::NameValue(name_value) => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(value),
                ..
            }) = &name_value.value
            {
                Ok((value.value(), value.span()))
            } else {
                Err(darling::Error::unexpected_type("expected string literal"))
            }
        },
        _ => Err(darling::Error::unsupported_format("string literal")),
    }
}

/// Marker for a bare attribute flag whose grammar accepts only path syntax.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PresentFlag;

impl PresentFlag {
    fn is_present(self) -> bool {
        true
    }
}

impl FromMeta for PresentFlag {
    fn from_word() -> darling::Result<Self> {
        Ok(Self)
    }

    fn from_bool(_value: bool) -> darling::Result<Self> {
        Err(darling::Error::custom(
            "expected bare flag syntax without `= true`",
        ))
    }
}

impl FromMeta for SpannedValue<GeneratedKeyName> {
    fn from_value(value: &syn::Lit) -> darling::Result<Self> {
        let syn::Lit::Str(value) = value else {
            return Err(darling::Error::unexpected_lit_type(value));
        };
        let key =
            GeneratedKeyName::try_new(value.value(), value.span(), AttrContext::VariantsContainer)
                .map_err(|error| darling::Error::custom(error.to_string()).with_span(value))?;
        Ok(SpannedValue::new(key, value.span()))
    }
}

impl FromMeta for SpannedValue<FluentMessageId> {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        let (value, span) = string_literal_value(item)?;
        let message_id =
            parse_fluent_message_id_in_context(value, span, AttrContext::MessageContainer)
                .map_err(|error| darling::Error::custom(error.to_string()).with_span(item))?;
        Ok(SpannedValue::new(message_id, span))
    }
}

impl FromMeta for SpannedValue<ArgName> {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        let (value, span) = string_literal_value(item)?;
        let arg = parse_arg_name_in_context(value, span, AttrContext::MessageField)
            .map_err(|error| darling::Error::custom(error.to_string()).with_span(item))?;
        Ok(SpannedValue::new(arg, span))
    }
}

impl FromMeta for SpannedValue<VariantKey> {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        let (value, span) = string_literal_value(item)?;
        let key = parse_variant_key_in_context(value, span, AttrContext::EnumVariant)
            .map_err(|error| darling::Error::custom(error.to_string()).with_span(item))?;
        Ok(SpannedValue::new(key, span))
    }
}

impl FromMeta for SpannedValue<DomainName> {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        let (value, span) = string_literal_value(item)?;
        let domain = parse_domain_name_in_context(value, span, AttrContext::MessageContainer)
            .map_err(|error| darling::Error::custom(error.to_string()).with_span(item))?;
        Ok(SpannedValue::new(domain, span))
    }
}

#[derive(Clone, Debug, Default)]
pub struct GeneratedKeyList {
    keys: Vec<SpannedValue<GeneratedKeyName>>,
}

impl GeneratedKeyList {
    fn new(keys: Vec<SpannedValue<GeneratedKeyName>>) -> darling::Result<Self> {
        let mut seen_values = std::collections::HashSet::new();
        let mut seen_idents = std::collections::HashSet::new();
        for key in &keys {
            if !seen_values.insert(key.value().clone()) {
                return Err(darling::Error::custom(format!(
                    "duplicate key '{}' in #[fluent_variants(keys = [...])]",
                    key.value().as_str()
                )));
            }
            let generated_ident_fragment = key.value().to_pascal_case();
            if !seen_idents.insert(generated_ident_fragment.clone()) {
                return Err(darling::Error::custom(format!(
                    "key '{}' generates duplicate Rust identifier fragment '{}'",
                    key.value().as_str(),
                    generated_ident_fragment
                )));
            }
        }

        Ok(Self { keys })
    }

    pub fn as_slice(&self) -> &[SpannedValue<GeneratedKeyName>] {
        &self.keys
    }
}

impl FromMeta for GeneratedKeyList {
    fn from_list(items: &[darling::ast::NestedMeta]) -> darling::Result<Self> {
        let keys = items
            .iter()
            .map(<SpannedValue<GeneratedKeyName> as FromMeta>::from_nested_meta)
            .collect::<darling::Result<Vec<_>>>()?;
        Self::new(keys)
    }

    fn from_value(value: &syn::Lit) -> darling::Result<Self> {
        let expr_array = syn::ExprArray::from_value(value)?;
        Self::from_expr(&syn::Expr::Array(expr_array))
    }

    fn from_expr(expr: &syn::Expr) -> darling::Result<Self> {
        match expr {
            syn::Expr::Array(expr_array) => {
                let keys = expr_array
                    .elems
                    .iter()
                    .map(<SpannedValue<GeneratedKeyName> as FromMeta>::from_expr)
                    .collect::<darling::Result<Vec<_>>>()?;
                Self::new(keys)
            },
            syn::Expr::Lit(expr_lit) => Self::from_value(&expr_lit.lit),
            syn::Expr::Group(group) => Self::from_expr(&group.expr),
            _ => Err(darling::Error::unexpected_expr_type(expr)),
        }
    }
}

pub fn keyed_variant_idents(
    ident: &syn::Ident,
    keys: Option<&[SpannedValue<GeneratedKeyName>]>,
    suffix: &str,
) -> EsFluentCoreResult<Vec<syn::Ident>> {
    Ok(keys
        .map(|keys| {
            keys.iter()
                .map(|key| GeneratedKeyIdent::variants(ident, key, suffix).into_ident())
                .collect()
        })
        .unwrap_or_default())
}

pub fn keyed_base_idents(
    ident: &syn::Ident,
    keys: Option<&[SpannedValue<GeneratedKeyName>]>,
) -> EsFluentCoreResult<Vec<syn::Ident>> {
    Ok(keys
        .map(|keys| {
            keys.iter()
                .map(|key| GeneratedKeyIdent::base(ident, key).into_ident())
                .collect()
        })
        .unwrap_or_default())
}

pub fn variants_enum_ident(ident: &syn::Ident, suffix: &str) -> syn::Ident {
    syn::Ident::new(
        &format!("{}{}", namer::rust_ident_name(ident), suffix),
        ident.span(),
    )
}

pub fn collect_items<T>(items: &[T]) -> Vec<&T> {
    items.iter().collect()
}

pub fn indexed_items<T>(items: &[T]) -> Vec<(DeclarationIndex, &T)> {
    items
        .iter()
        .enumerate()
        .map(|(index, item)| (DeclarationIndex::new(index), item))
        .collect()
}

pub trait SkipDirective {
    fn is_skipped(&self) -> bool;
}

pub trait Skippable {
    type Directive: SkipDirective;

    fn skip_directive(&self) -> &Self::Directive;
}

pub fn filter_unskipped<T: Skippable>(items: &[T]) -> Vec<&T> {
    items
        .iter()
        .filter(|item| !item.skip_directive().is_skipped())
        .collect()
}

pub fn indexed_unskipped<T: Skippable>(items: &[T]) -> Vec<(DeclarationIndex, &T)> {
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| !item.skip_directive().is_skipped())
        .map(|(index, item)| (DeclarationIndex::new(index), item))
        .collect()
}

pub fn struct_items<T: Skippable>(data: &darling::ast::Data<darling::util::Ignored, T>) -> Vec<&T> {
    match data {
        darling::ast::Data::Struct(fields) => filter_unskipped(&fields.fields),
        _ => Vec::new(),
    }
}

pub fn indexed_struct_items<T: Skippable>(
    data: &darling::ast::Data<darling::util::Ignored, T>,
) -> Vec<(DeclarationIndex, &T)> {
    match data {
        darling::ast::Data::Struct(fields) => indexed_unskipped(&fields.fields),
        _ => Vec::new(),
    }
}

pub fn all_indexed_struct_items<T>(
    data: &darling::ast::Data<darling::util::Ignored, T>,
) -> Vec<(DeclarationIndex, &T)> {
    match data {
        darling::ast::Data::Struct(fields) => indexed_items(&fields.fields),
        _ => Vec::new(),
    }
}

pub fn enum_items<T>(data: &darling::ast::Data<T, darling::util::Ignored>) -> Vec<&T> {
    match data {
        darling::ast::Data::Enum(variants) => variants.iter().collect(),
        _ => Vec::new(),
    }
}

pub fn filtered_enum_items<T: Skippable>(
    data: &darling::ast::Data<T, darling::util::Ignored>,
) -> Vec<&T> {
    match data {
        darling::ast::Data::Enum(variants) => filter_unskipped(variants),
        _ => Vec::new(),
    }
}

pub fn variant_style<T>(fields: &darling::ast::Fields<T>) -> darling::ast::Style {
    fields.style
}

pub fn filtered_variant_fields<T: Skippable>(fields: &darling::ast::Fields<T>) -> Vec<&T> {
    filter_unskipped(&fields.fields)
}

pub fn all_variant_fields<T>(fields: &darling::ast::Fields<T>) -> Vec<&T> {
    collect_items(&fields.fields)
}

pub fn is_single_tuple_variant<T>(fields: &darling::ast::Fields<T>) -> bool {
    matches!(variant_style(fields), darling::ast::Style::Tuple) && fields.len() == 1
}

/// Shared behavior for enum-like variants that expose a `darling::ast::Fields` payload.
pub trait VariantFields {
    type Field;

    /// Returns the raw field collection for the variant.
    fn variant_fields(&self) -> &darling::ast::Fields<Self::Field>;

    /// Returns the style of the variant's fields.
    fn style(&self) -> darling::ast::Style {
        variant_style(self.variant_fields())
    }

    /// Returns the fields of the variant that are not skipped.
    fn fields(&self) -> Vec<&Self::Field>
    where
        Self::Field: Skippable,
    {
        filtered_variant_fields(self.variant_fields())
    }

    /// Returns all fields of the variant.
    fn all_fields(&self) -> Vec<&Self::Field> {
        all_variant_fields(self.variant_fields())
    }

    /// Returns true if this is a tuple variant with exactly one field.
    fn is_single_tuple(&self) -> bool {
        is_single_tuple_variant(self.variant_fields())
    }
}

/// Shared behavior for variants that allow overriding their localization key.
pub trait KeyedVariant {
    /// Returns the explicit localization key for the variant, if provided.
    fn directive(&self) -> &MessageVariantDirective;
}

pub fn ftl_variants_ident(ident: &syn::Ident) -> syn::Ident {
    variants_enum_ident(ident, "Variants")
}

pub fn keyed_variants_idents(
    ident: &syn::Ident,
    attr_args: &VariantsFluentAttributeArgs,
) -> EsFluentCoreResult<Vec<syn::Ident>> {
    keyed_variant_idents(
        ident,
        attr_args.keys.as_ref().map(GeneratedKeyList::as_slice),
        "Variants",
    )
}

pub fn keyed_variants_base_idents(
    ident: &syn::Ident,
    attr_args: &VariantsFluentAttributeArgs,
) -> EsFluentCoreResult<Vec<syn::Ident>> {
    keyed_base_idents(
        ident,
        attr_args.keys.as_ref().map(GeneratedKeyList::as_slice),
    )
}

/// Shared behavior for option types backed by struct data.
pub trait StructDataOptions {
    type Field;

    /// Returns the raw `darling` data payload for the struct.
    fn struct_data(&self) -> &darling::ast::Data<darling::util::Ignored, Self::Field>;

    /// Returns the fields of the struct that are not skipped.
    fn fields(&self) -> Vec<&Self::Field>
    where
        Self::Field: Skippable,
    {
        struct_items(self.struct_data())
    }

    /// Returns the fields of the struct paired with their declaration index.
    fn indexed_fields(&self) -> Vec<(DeclarationIndex, &Self::Field)>
    where
        Self::Field: Skippable,
    {
        indexed_struct_items(self.struct_data())
    }

    /// Returns all fields (including skipped) paired with their declaration index.
    fn all_indexed_fields(&self) -> Vec<(DeclarationIndex, &Self::Field)> {
        all_indexed_struct_items(self.struct_data())
    }
}

/// Shared behavior for option types backed by enum data.
pub trait EnumDataOptions {
    type Variant;

    /// Returns the raw `darling` data payload for the enum.
    fn enum_data(&self) -> &darling::ast::Data<Self::Variant, darling::util::Ignored>;

    /// Returns all variants declared in the enum.
    fn variants(&self) -> Vec<&Self::Variant> {
        enum_items(self.enum_data())
    }
}

/// Shared behavior for enum option types that expose only unskipped variants.
pub trait FilteredEnumDataOptions {
    type Variant: Skippable;

    /// Returns the raw `darling` data payload for the enum.
    fn enum_data(&self) -> &darling::ast::Data<Self::Variant, darling::util::Ignored>;

    /// Returns the variants of the enum that are not skipped.
    fn variants(&self) -> Vec<&Self::Variant> {
        filtered_enum_items(self.enum_data())
    }
}

/// Shared behavior for `#[fluent_variants]` container option types.
pub trait GeneratedVariantsOptions {
    /// Returns the source type identifier used to build generated enum names.
    fn variants_ident(&self) -> &syn::Ident;

    /// Returns the shared variants attribute payload.
    fn variants_attr_args(&self) -> &VariantsFluentAttributeArgs;

    /// Returns the identifier of the generated FTL enum.
    fn ftl_enum_ident(&self) -> syn::Ident {
        ftl_variants_ident(self.variants_ident())
    }

    /// Returns the identifiers of the keyed FTL enums.
    fn keyed_idents(&self) -> EsFluentCoreResult<Vec<syn::Ident>> {
        keyed_variants_idents(self.variants_ident(), self.variants_attr_args())
    }

    /// Returns the identifiers used to build base FTL keys (without suffixes).
    fn keyed_base_idents(&self) -> EsFluentCoreResult<Vec<syn::Ident>> {
        keyed_variants_base_idents(self.variants_ident(), self.variants_attr_args())
    }
}

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
struct FluentFieldAttributeArgs {
    /// Whether to skip this field.
    #[darling(default)]
    skip: Option<PresentFlag>,
    /// Whether this field is a selector for a Fluent select expression.
    #[darling(default)]
    selector: Option<PresentFlag>,
    /// Whether this field should be treated as an optional argument.
    #[darling(default)]
    optional: Option<PresentFlag>,
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

    fn is_optional(&self) -> bool {
        self.optional.is_some_and(PresentFlag::is_present)
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
        let is_optional = self.is_optional();
        let has_value = self.value().is_some();
        let has_arg = self.arg.is_some();

        if is_skipped {
            if has_arg {
                return Err(field_strategy_error(
                    "Cannot use #[fluent(arg = \"...\")] on a skipped field",
                    span,
                ));
            }
            if is_optional {
                return Err(field_strategy_error(
                    "Cannot use #[fluent(optional)] on a skipped field",
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

        if is_optional && is_selector {
            return Err(field_strategy_error(
                "Cannot combine #[fluent(optional)] and #[fluent(selector)] on the same field",
                span,
            ));
        }

        if is_optional && has_value {
            return Err(field_strategy_error(
                "Cannot combine #[fluent(optional)] and #[fluent(value = ...)] on the same field",
                span,
            ));
        }

        if is_optional {
            let Some(inner_ty) = option_inner_type(ty) else {
                return Err(field_strategy_error(
                    "#[fluent(optional)] can only be used on Option<T> fields",
                    span,
                ));
            };
            return Ok(FieldDirective::Argument(Box::new(FieldArgumentDirective {
                name: self.arg.clone(),
                value: FieldValueDirective::Optional {
                    span: ty.span(),
                    inner_ty: inner_ty.clone(),
                },
            })));
        }

        if is_selector {
            return Ok(FieldDirective::Argument(Box::new(FieldArgumentDirective {
                name: self.arg.clone(),
                value: FieldValueDirective::Choice { span },
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
    fn from_attr_args(
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
    Choice { span: proc_macro2::Span },
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
            Self::Choice { span } => ArgumentValueStrategy::Choice { span: *span },
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

/// Closed representation of a message variant's localization behavior.
#[derive(Clone, Debug)]
pub enum MessageVariantDirective {
    Localized {
        key: Option<SpannedValue<VariantKey>>,
    },
    Skipped {
        key: Option<SpannedValue<VariantKey>>,
    },
}

impl MessageVariantDirective {
    pub fn key(&self) -> Option<&SpannedValue<VariantKey>> {
        match self {
            Self::Localized { key } | Self::Skipped { key } => key.as_ref(),
        }
    }

    pub fn variant_key(
        &self,
        _context: AttrContext,
    ) -> EsFluentCoreResult<Option<SpannedValue<VariantKey>>> {
        Ok(self.key().cloned())
    }
}

impl SkipDirective for MessageVariantDirective {
    fn is_skipped(&self) -> bool {
        matches!(self, Self::Skipped { .. })
    }
}

/// Closed representation of generated-variant inclusion behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeneratedVariantDirective {
    Include,
    Skip,
}

impl SkipDirective for GeneratedVariantDirective {
    fn is_skipped(&self) -> bool {
        matches!(self, Self::Skip)
    }
}

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
struct SkippedVariantAttributeArgs {
    /// Whether to skip this variant.
    #[darling(default)]
    skip: Option<PresentFlag>,
}

impl SkippedVariantAttributeArgs {
    fn directive(&self) -> GeneratedVariantDirective {
        if self.skip.is_some_and(PresentFlag::is_present) {
            GeneratedVariantDirective::Skip
        } else {
            GeneratedVariantDirective::Include
        }
    }
}

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
struct KeyedVariantAttributeArgs {
    #[darling(flatten)]
    skipped_args: SkippedVariantAttributeArgs,
    /// Overrides the localization key suffix for this variant.
    #[darling(default)]
    key: Option<SpannedValue<VariantKey>>,
}

impl KeyedVariantAttributeArgs {
    fn directive(&self) -> MessageVariantDirective {
        if matches!(
            self.skipped_args.directive(),
            GeneratedVariantDirective::Skip
        ) {
            MessageVariantDirective::Skipped {
                key: self.key.clone(),
            }
        } else {
            MessageVariantDirective::Localized {
                key: self.key.clone(),
            }
        }
    }
}

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct NamespacedAttributeArgs {
    /// Optional namespace for FTL file generation.
    /// - `namespace = "name"` - writes to `{lang}/{crate}/{name}.ftl`
    /// - `namespace = file` - writes to `{lang}/{crate}/{source_file_stem}.ftl`
    /// - `namespace = file_relative` - writes to `{lang}/{crate}/{relative_path}.ftl`
    /// - `namespace = folder` - writes to `{lang}/{crate}/{source_parent_folder}.ftl`
    /// - `namespace = folder_relative` - writes to `{lang}/{crate}/{relative_parent_folder_path}.ftl`
    #[darling(default)]
    namespace: Option<SpannedNamespaceRule>,
}

impl NamespacedAttributeArgs {
    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&NamespaceRule> {
        self.namespace.as_ref().map(SpannedNamespaceRule::rule)
    }

    /// Returns the span of the namespace value if provided.
    pub fn namespace_span(&self) -> Option<proc_macro2::Span> {
        self.namespace.as_ref().map(SpannedNamespaceRule::span)
    }

    /// Returns the parsed namespace spec if provided.
    pub fn namespace_spec(&self) -> Option<&SpannedNamespaceRule> {
        self.namespace.as_ref()
    }
}

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct DerivedNamespacedAttributeArgs {
    /// The traits to derive on the FTL enum.
    #[getset(get = "pub")]
    #[darling(default)]
    derive: darling::util::PathList,
    #[darling(flatten)]
    namespace_args: NamespacedAttributeArgs,
}

impl DerivedNamespacedAttributeArgs {
    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&NamespaceRule> {
        self.namespace_args.namespace()
    }

    /// Returns the span of the namespace value if provided.
    pub fn namespace_span(&self) -> Option<proc_macro2::Span> {
        self.namespace_args.namespace_span()
    }
}

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct VariantsFluentAttributeArgs {
    #[darling(default)]
    keys: Option<GeneratedKeyList>,
    #[darling(flatten)]
    derived_args: DerivedNamespacedAttributeArgs,
}

impl VariantsFluentAttributeArgs {
    /// Returns the traits to derive on the generated enum.
    pub fn derive(&self) -> &darling::util::PathList {
        self.derived_args.derive()
    }

    /// Returns the typed generated variant keys if provided.
    pub fn keys(&self) -> Option<&[SpannedValue<GeneratedKeyName>]> {
        self.keys.as_ref().map(GeneratedKeyList::as_slice)
    }

    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&NamespaceRule> {
        self.derived_args.namespace()
    }

    /// Returns the span of the namespace value if provided.
    pub fn namespace_span(&self) -> Option<proc_macro2::Span> {
        self.derived_args.namespace_span()
    }
}

#[derive(Clone, Debug)]
pub struct ValueAttr(pub syn::Expr);

impl darling::FromMeta for ValueAttr {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        match item {
            syn::Meta::NameValue(nv) => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    Err(darling::Error::custom(format!(
                        "expected Rust expression, not string literal; use `value = {}`",
                        s.value()
                    )))
                } else {
                    Ok(ValueAttr(nv.value.clone()))
                }
            },
            _ => Err(darling::Error::unsupported_format(
                "name-value expression, such as `value = |x: &String| x.len()`",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::r#struct::StructOpts;
    use darling::FromDeriveInput as _;
    use darling::FromVariant as _;

    fn generated_key(name: &str) -> SpannedValue<GeneratedKeyName> {
        let span = proc_macro2::Span::call_site();
        SpannedValue::new(
            GeneratedKeyName::try_new(name, span, AttrContext::VariantsContainer)
                .expect("generated key"),
            span,
        )
    }

    #[test]
    fn generated_key_name_accepts_and_rejects_expected_values() {
        let span = proc_macro2::Span::call_site();
        let good = GeneratedKeyName::try_new("user_label", span, AttrContext::VariantsContainer)
            .expect("valid snake_case");
        assert_eq!(good.as_str(), "user_label");
        assert_eq!(good.to_pascal_case(), "UserLabel");

        let err = GeneratedKeyName::try_new("UserLabel", span, AttrContext::VariantsContainer)
            .expect_err("invalid key should fail");
        let message = err.to_string();
        assert!(message.contains("lowercase snake_case"));
        assert!(message.contains("help: use values like"));
    }

    #[test]
    fn value_attr_from_meta_supports_name_value_expression() {
        let nv_meta: syn::Meta = syn::parse_quote!(value = |x: &str| x.len());
        let nv = ValueAttr::from_meta(&nv_meta).expect("name-value expression");
        let nv_expr = nv.0;
        assert_eq!(
            quote::quote!(#nv_expr).to_string(),
            "| x : & str | x . len ()"
        );
    }

    #[test]
    fn bare_flag_parser_rejects_name_value_booleans() {
        let input: syn::DeriveInput = syn::parse_quote! {
            struct Message {
                #[fluent(skip = true)]
                hidden: String,
            }
        };

        let err = match StructOpts::from_derive_input(&input) {
            Ok(_) => panic!("name-value booleans should not parse as bare flags"),
            Err(error) => error,
        };

        assert!(
            err.to_string()
                .contains("expected bare flag syntax without `= true`")
        );
    }

    #[test]
    fn value_attr_from_meta_rejects_non_expression_formats() {
        let string_meta: syn::Meta = syn::parse_quote!(value = "|x: &str| x.len()");
        let string_err = ValueAttr::from_meta(&string_meta).expect_err("string should fail");
        assert!(string_err.to_string().contains("not string literal"));

        let list_meta: syn::Meta = syn::parse_quote!(value(|x: &String| x.len()));
        let list_err = ValueAttr::from_meta(&list_meta).expect_err("list format should fail");
        assert!(!list_err.to_string().is_empty());

        let path_meta: syn::Meta = syn::parse_quote!(value);
        let path_err = ValueAttr::from_meta(&path_meta).expect_err("path format should fail");
        assert!(!path_err.to_string().is_empty());
    }

    #[test]
    fn shared_helpers_cover_typed_keys_and_item_filtering() {
        #[derive(Clone, Debug, PartialEq)]
        struct Item {
            directive: GeneratedVariantDirective,
        }

        impl Skippable for Item {
            type Directive = GeneratedVariantDirective;

            fn skip_directive(&self) -> &Self::Directive {
                &self.directive
            }
        }

        let items = vec![
            Item {
                directive: GeneratedVariantDirective::Include,
            },
            Item {
                directive: GeneratedVariantDirective::Skip,
            },
            Item {
                directive: GeneratedVariantDirective::Include,
            },
        ];

        assert_eq!(collect_items(&items).len(), 3);
        assert_eq!(indexed_items(&items).len(), 3);
        assert_eq!(filter_unskipped(&items).len(), 2);
        assert_eq!(indexed_unskipped(&items).len(), 2);

        let keys = [generated_key("label"), generated_key("description")];
        let key_names: Vec<_> = keys.iter().map(|key| key.value().as_str()).collect();
        assert_eq!(key_names, vec!["label", "description"]);

        let ident: syn::Ident = syn::parse_quote!(ProfileForm);
        assert_eq!(
            variants_enum_ident(&ident, "Variants").to_string(),
            "ProfileFormVariants"
        );
    }

    #[test]
    fn shared_field_and_variant_helpers_cover_closed_directives() {
        #[derive(Clone, Debug, PartialEq)]
        struct LocalItem {
            directive: GeneratedVariantDirective,
        }

        impl Skippable for LocalItem {
            type Directive = GeneratedVariantDirective;

            fn skip_directive(&self) -> &Self::Directive {
                &self.directive
            }
        }

        let skipped_field: syn::Field = syn::parse_quote! {
            #[fluent(skip)]
            hidden: bool
        };
        let skipped_field = FluentFieldOpts::from_field(&skipped_field).expect("field parse");
        assert!(skipped_field.directive().is_skipped());

        let transformed_field: syn::Field = syn::parse_quote! {
            #[fluent(arg = "display_name", value = |x: &str| x.len())]
            name: String
        };
        let transformed_field =
            FluentFieldOpts::from_field(&transformed_field).expect("field parse");
        assert_eq!(
            transformed_field
                .directive()
                .arg_name()
                .expect("arg")
                .value()
                .as_str(),
            "display_name"
        );
        assert!(matches!(
            transformed_field
                .directive()
                .argument()
                .expect("argument")
                .value(),
            FieldValueDirective::Transform(_)
        ));

        let skipped_variant: syn::Variant = syn::parse_quote!(
            #[fluent(skip, key = "skipped")]
            Skipped
        );
        let skipped_variant = crate::options::r#enum::VariantOpts::from_variant(&skipped_variant)
            .expect("variant parse");
        assert!(skipped_variant.directive().is_skipped());
        assert_eq!(
            skipped_variant
                .directive()
                .key()
                .expect("key")
                .value()
                .as_str(),
            "skipped"
        );

        let generated_variant: syn::Variant = syn::parse_quote!(
            #[fluent_variants(skip)]
            Hidden
        );
        let generated_variant =
            crate::options::r#enum::EnumVariantOpts::from_variant(&generated_variant)
                .expect("generated variant parse");
        assert!(generated_variant.skip_directive().is_skipped());

        let tuple_fields = darling::ast::Fields::new(
            darling::ast::Style::Tuple,
            vec![LocalItem {
                directive: GeneratedVariantDirective::Include,
            }],
        );
        assert!(is_single_tuple_variant(&tuple_fields));
        assert_eq!(filtered_variant_fields(&tuple_fields).len(), 1);
        assert_eq!(all_variant_fields(&tuple_fields).len(), 1);
    }

    #[test]
    fn field_directive_rejects_conflicting_strategies_at_typed_boundary() {
        fn err_for(field: syn::Field) -> String {
            FluentFieldOpts::from_field(&field)
                .expect_err("conflicting field strategy should fail")
                .to_string()
        }

        assert!(
            err_for(syn::parse_quote! {
                #[fluent(skip, arg = "display_name")]
                name: String
            })
            .contains("arg")
        );
        assert!(
            err_for(syn::parse_quote! {
                #[fluent(skip, optional)]
                name: Option<String>
            })
            .contains("optional")
        );
        assert!(
            err_for(syn::parse_quote! {
                #[fluent(skip, selector)]
                name: String
            })
            .contains("selector")
        );
        assert!(
            err_for(syn::parse_quote! {
                #[fluent(skip, value = |x: &str| x.len())]
                name: String
            })
            .contains("value")
        );
        assert!(
            err_for(syn::parse_quote! {
                #[fluent(selector, value = |x: &str| x.len())]
                name: String
            })
            .contains("selector")
        );
        assert!(
            err_for(syn::parse_quote! {
                #[fluent(optional, selector)]
                name: Option<String>
            })
            .contains("optional")
        );
        assert!(
            err_for(syn::parse_quote! {
                #[fluent(optional, value = |x: &str| x.len())]
                name: Option<String>
            })
            .contains("optional")
        );
        assert!(
            err_for(syn::parse_quote! {
                #[fluent(optional)]
                name: String
            })
            .contains("Option<T>")
        );
    }
}

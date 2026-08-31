use std::collections::HashSet;

use super::{MessageVariantDirective, VariantsFluentAttributeArgs};
use crate::error::EsFluentCoreResult;
use crate::index::DeclarationIndex;
use crate::semantic::{GeneratedKeyIdent, GeneratedKeyName, SpannedValue};
use darling::FromMeta;
use es_fluent_shared::namer;

#[derive(Clone, Debug, Default)]
pub struct GeneratedKeyList {
    keys: Vec<SpannedValue<GeneratedKeyName>>,
}

impl GeneratedKeyList {
    fn new(keys: Vec<SpannedValue<GeneratedKeyName>>) -> darling::Result<Self> {
        let mut seen_values = HashSet::new();
        let mut seen_idents = HashSet::new();
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

    pub fn span(&self) -> Option<proc_macro2::Span> {
        self.keys.first().map(SpannedValue::span)
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

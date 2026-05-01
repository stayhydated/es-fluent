//! This module provides types for parsing `es-fluent` attributes.

use crate::error::{ErrorExt as _, EsFluentCoreError, EsFluentCoreResult};
use bon::Builder;
use darling::{FromField, FromMeta};
use es_fluent_shared::{namer, namespace::NamespaceRule};
use getset::Getters;
use heck::{ToPascalCase as _, ToSnakeCase as _};
use quote::format_ident;

pub mod choice;
pub mod r#enum;
pub mod r#struct;
pub mod this;

/// Validate that a key is lowercase snake_case and return its PascalCase version.
///
/// This is a shared helper for `#[fluent_variants]` key validation used by both
/// `EnumVariantsOpts` and `StructVariantsOpts`.
pub fn validate_snake_case_key(key: &syn::LitStr) -> EsFluentCoreResult<String> {
    let key_str = key.value();
    let snake_cased = key_str.to_snake_case();
    let is_lower_snake =
        !key_str.is_empty() && key_str == snake_cased && key_str == key_str.to_ascii_lowercase();

    if !is_lower_snake {
        return Err(EsFluentCoreError::AttributeError {
            message: format!(
                "keys in #[fluent_variants] must be lowercase snake_case; found \"{}\"",
                key_str
            ),
            span: Some(key.span()),
        }
        .with_help("Use values like \"description\" or \"label\".".to_string()));
    }

    Ok(key_str.to_pascal_case())
}

pub fn keyed_variant_idents(
    ident: &syn::Ident,
    keys: Option<Vec<syn::LitStr>>,
    suffix: &str,
) -> EsFluentCoreResult<Vec<syn::Ident>> {
    keys.map_or_else(
        || Ok(Vec::new()),
        |keys| {
            keys.into_iter()
                .map(|key| {
                    let pascal_key = validate_snake_case_key(&key)?;
                    Ok(format_ident!(
                        "{}{}{}",
                        namer::rust_ident_name(ident),
                        pascal_key,
                        suffix
                    ))
                })
                .collect()
        },
    )
}

pub fn keyed_base_idents(
    ident: &syn::Ident,
    keys: Option<Vec<syn::LitStr>>,
) -> EsFluentCoreResult<Vec<syn::Ident>> {
    keys.map_or_else(
        || Ok(Vec::new()),
        |keys| {
            keys.into_iter()
                .map(|key| {
                    let pascal_key = validate_snake_case_key(&key)?;
                    Ok(format_ident!(
                        "{}{}",
                        namer::rust_ident_name(ident),
                        pascal_key
                    ))
                })
                .collect()
        },
    )
}

pub fn variants_enum_ident(ident: &syn::Ident, suffix: &str) -> syn::Ident {
    format_ident!("{}{}", namer::rust_ident_name(ident), suffix)
}

pub fn key_strings(keys: Option<&[syn::LitStr]>) -> Option<Vec<String>> {
    keys.map(|keys| keys.iter().map(syn::LitStr::value).collect())
}

pub fn collect_items<T>(items: &[T]) -> Vec<&T> {
    items.iter().collect()
}

pub fn indexed_items<T>(items: &[T]) -> Vec<(usize, &T)> {
    items.iter().enumerate().collect()
}

pub trait Skippable {
    fn is_skipped(&self) -> bool;
}

pub fn filter_unskipped<T: Skippable>(items: &[T]) -> Vec<&T> {
    items.iter().filter(|item| !item.is_skipped()).collect()
}

pub fn indexed_unskipped<T: Skippable>(items: &[T]) -> Vec<(usize, &T)> {
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| !item.is_skipped())
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
) -> Vec<(usize, &T)> {
    match data {
        darling::ast::Data::Struct(fields) => indexed_unskipped(&fields.fields),
        _ => Vec::new(),
    }
}

pub fn all_indexed_struct_items<T>(
    data: &darling::ast::Data<darling::util::Ignored, T>,
) -> Vec<(usize, &T)> {
    match data {
        darling::ast::Data::Struct(fields) => indexed_items(&fields.fields),
        _ => Vec::new(),
    }
}

pub fn enum_items<T>(data: &darling::ast::Data<T, darling::util::Ignored>) -> Vec<&T> {
    match data {
        darling::ast::Data::Enum(variants) => variants.iter().collect(),
        _ => unreachable!("Unexpected data type for enum"),
    }
}

pub fn filtered_enum_items<T: Skippable>(
    data: &darling::ast::Data<T, darling::util::Ignored>,
) -> Vec<&T> {
    match data {
        darling::ast::Data::Enum(variants) => filter_unskipped(variants),
        _ => unreachable!("Unexpected data type for enum"),
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
    fn key(&self) -> Option<&str>;
}

pub fn ftl_variants_ident(ident: &syn::Ident) -> syn::Ident {
    variants_enum_ident(ident, "Variants")
}

pub fn keyed_variants_idents(
    ident: &syn::Ident,
    attr_args: &VariantsFluentAttributeArgs,
) -> EsFluentCoreResult<Vec<syn::Ident>> {
    keyed_variant_idents(ident, attr_args.clone().keys, "Variants")
}

pub fn keyed_variants_base_idents(
    ident: &syn::Ident,
    attr_args: &VariantsFluentAttributeArgs,
) -> EsFluentCoreResult<Vec<syn::Ident>> {
    keyed_base_idents(ident, attr_args.clone().keys)
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
    fn indexed_fields(&self) -> Vec<(usize, &Self::Field)>
    where
        Self::Field: Skippable,
    {
        indexed_struct_items(self.struct_data())
    }

    /// Returns all fields (including skipped) paired with their declaration index.
    fn all_indexed_fields(&self) -> Vec<(usize, &Self::Field)> {
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
    /// Returns the shared fluent field attribute arguments.
    fn field_attr_args(&self) -> &FluentFieldAttributeArgs;

    /// Returns `true` if the field should be skipped.
    fn is_skipped(&self) -> bool {
        self.field_attr_args().is_skipped()
    }

    /// Returns `true` if the field is a choice.
    fn is_choice(&self) -> bool {
        self.field_attr_args().is_choice()
    }

    /// Returns the value expression if present.
    fn value(&self) -> Option<&syn::Expr> {
        self.field_attr_args().value()
    }

    /// Returns explicit field argument name if provided.
    fn arg_name(&self) -> Option<String> {
        self.field_attr_args().arg_name()
    }

    /// Resolves the Fluent argument name for this field.
    fn fluent_arg_name(&self, index: usize) -> String {
        self.arg_name()
            .or_else(|| self.ident().map(namer::rust_ident_name))
            .unwrap_or_else(|| namer::UnnamedItem::from(index).to_string())
    }
}

impl<T: FluentField> Skippable for T {
    fn is_skipped(&self) -> bool {
        FluentField::is_skipped(self)
    }
}

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct SkippableFieldAttributeArgs {
    /// Whether to skip this field.
    #[darling(default)]
    skip: Option<bool>,
}

impl SkippableFieldAttributeArgs {
    pub fn is_skipped(&self) -> bool {
        self.skip.unwrap_or(false)
    }
}

#[derive(Clone, Debug, FromField)]
#[darling(attributes(fluent_variants))]
pub struct SkippableFieldOpts {
    /// The identifier of the field.
    ident: Option<syn::Ident>,
    /// The type of the field.
    ty: syn::Type,
    #[darling(flatten)]
    attr_args: SkippableFieldAttributeArgs,
}

impl SkippableFieldOpts {
    pub fn ident(&self) -> Option<&syn::Ident> {
        self.ident.as_ref()
    }

    pub fn ty(&self) -> &syn::Type {
        &self.ty
    }

    pub fn is_skipped(&self) -> bool {
        self.attr_args.is_skipped()
    }
}

impl Skippable for SkippableFieldOpts {
    fn is_skipped(&self) -> bool {
        self.attr_args.is_skipped()
    }
}

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct FluentFieldAttributeArgs {
    /// Whether to skip this field.
    #[darling(default)]
    skip: Option<bool>,
    /// Whether this field is a choice.
    #[darling(default)]
    choice: Option<bool>,
    /// A value transformation expression.
    #[darling(default)]
    value: Option<ValueAttr>,
    /// Optional argument name override.
    #[darling(default)]
    arg_name: Option<syn::LitStr>,
}

impl FluentFieldAttributeArgs {
    pub fn is_skipped(&self) -> bool {
        self.skip.unwrap_or(false)
    }

    pub fn is_choice(&self) -> bool {
        self.choice.unwrap_or(false)
    }

    pub fn value(&self) -> Option<&syn::Expr> {
        self.value.as_ref().map(|value| &value.0)
    }

    pub fn arg_name(&self) -> Option<String> {
        self.arg_name.as_ref().map(syn::LitStr::value)
    }
}

#[derive(Clone, Debug, FromField)]
#[darling(attributes(fluent))]
pub struct FluentFieldOpts {
    /// The identifier of the field.
    ident: Option<syn::Ident>,
    /// The type of the field.
    ty: syn::Type,
    #[darling(flatten)]
    attr_args: FluentFieldAttributeArgs,
}

impl FluentFieldOpts {
    pub fn ident(&self) -> Option<&syn::Ident> {
        self.ident.as_ref()
    }

    pub fn ty(&self) -> &syn::Type {
        &self.ty
    }
}

impl FluentField for FluentFieldOpts {
    fn ident(&self) -> Option<&syn::Ident> {
        self.ident.as_ref()
    }

    fn ty(&self) -> &syn::Type {
        &self.ty
    }

    fn field_attr_args(&self) -> &FluentFieldAttributeArgs {
        &self.attr_args
    }
}

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct SkippedVariantAttributeArgs {
    /// Whether to skip this variant.
    #[darling(default)]
    skip: Option<bool>,
}

impl SkippedVariantAttributeArgs {
    pub fn is_skipped(&self) -> bool {
        self.skip.unwrap_or(false)
    }
}

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct KeyedVariantAttributeArgs {
    #[darling(flatten)]
    skipped_args: SkippedVariantAttributeArgs,
    /// Overrides the localization key suffix for this variant.
    #[darling(default)]
    key: Option<String>,
}

impl KeyedVariantAttributeArgs {
    pub fn is_skipped(&self) -> bool {
        self.skipped_args.is_skipped()
    }

    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
}

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct NamespacedAttributeArgs {
    /// Optional namespace for FTL file generation.
    /// - `namespace = "name"` - writes to `{lang}/{crate}/{name}.ftl`
    /// - `namespace = file` - writes to `{lang}/{crate}/{source_file_stem}.ftl`
    /// - `namespace(file(relative))` - writes to `{lang}/{crate}/{relative_path}.ftl`
    /// - `namespace = folder` - writes to `{lang}/{crate}/{source_parent_folder}.ftl`
    /// - `namespace(folder(relative))` - writes to `{lang}/{crate}/{relative_parent_folder_path}.ftl`
    #[darling(default)]
    namespace: Option<NamespaceRule>,
}

impl NamespacedAttributeArgs {
    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&NamespaceRule> {
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
}

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct VariantsFluentAttributeArgs {
    #[darling(default)]
    keys: Option<Vec<syn::LitStr>>,
    #[darling(flatten)]
    derived_args: DerivedNamespacedAttributeArgs,
}

impl VariantsFluentAttributeArgs {
    /// Returns the traits to derive on the generated enum.
    pub fn derive(&self) -> &darling::util::PathList {
        self.derived_args.derive()
    }

    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&NamespaceRule> {
        self.derived_args.namespace()
    }

    /// Returns the raw key strings if provided.
    pub fn key_strings(&self) -> Option<Vec<String>> {
        key_strings(self.keys.as_deref())
    }
}

#[derive(Clone, Debug)]
pub struct ValueAttr(pub syn::Expr);

impl darling::FromMeta for ValueAttr {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        match item {
            syn::Meta::List(list) => {
                let expr: syn::Expr = syn::parse2(list.tokens.clone())?;
                Ok(ValueAttr(expr))
            },
            syn::Meta::NameValue(nv) => {
                // Also support value = "expr" for convenience
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    let expr: syn::Expr = s.parse()?;
                    Ok(ValueAttr(expr))
                } else {
                    Err(darling::Error::unexpected_type("non-string literal"))
                }
            },
            _ => Err(darling::Error::unsupported_format("list or name-value")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_snake_case_key_accepts_and_rejects_expected_values() {
        let good: syn::LitStr = syn::parse_quote!("user_label");
        let converted = validate_snake_case_key(&good).expect("valid snake_case");
        assert_eq!(converted, "UserLabel");

        let bad: syn::LitStr = syn::parse_quote!("UserLabel");
        let err = validate_snake_case_key(&bad).expect_err("invalid key should fail");
        let message = err.to_string();
        assert!(message.contains("lowercase snake_case"));
        assert!(message.contains("help: Use values like"));
    }

    #[test]
    fn value_attr_from_meta_supports_list_and_name_value_string() {
        let list_meta: syn::Meta = syn::parse_quote!(value(|x: &String| x.len()));
        let list = ValueAttr::from_meta(&list_meta).expect("list format");
        let list_expr = list.0;
        assert_eq!(
            quote::quote!(#list_expr).to_string(),
            "| x : & String | x . len ()"
        );

        let nv_meta: syn::Meta = syn::parse_quote!(value = "|x: &str| x.len()");
        let nv = ValueAttr::from_meta(&nv_meta).expect("name-value string");
        let nv_expr = nv.0;
        assert_eq!(
            quote::quote!(#nv_expr).to_string(),
            "| x : & str | x . len ()"
        );
    }

    #[test]
    fn value_attr_from_meta_rejects_non_string_and_unsupported_formats() {
        let non_string_meta: syn::Meta = syn::parse_quote!(value = 123);
        let non_string_err =
            ValueAttr::from_meta(&non_string_meta).expect_err("non-string should fail");
        assert!(!non_string_err.to_string().is_empty());

        let path_meta: syn::Meta = syn::parse_quote!(value);
        let path_err = ValueAttr::from_meta(&path_meta).expect_err("path format should fail");
        assert!(!path_err.to_string().is_empty());
    }

    #[test]
    fn shared_helpers_cover_key_strings_and_item_filtering() {
        #[derive(Clone, Debug, PartialEq)]
        struct Item {
            skipped: bool,
        }

        impl Skippable for Item {
            fn is_skipped(&self) -> bool {
                self.skipped
            }
        }

        let items = vec![
            Item { skipped: false },
            Item { skipped: true },
            Item { skipped: false },
        ];

        assert_eq!(collect_items(&items).len(), 3);
        assert_eq!(indexed_items(&items).len(), 3);
        assert_eq!(filter_unskipped(&items).len(), 2);
        assert_eq!(indexed_unskipped(&items).len(), 2);

        let keys = vec![syn::parse_quote!("label"), syn::parse_quote!("description")];
        assert_eq!(
            key_strings(Some(keys.as_slice())),
            Some(vec!["label".to_string(), "description".to_string()])
        );

        let ident: syn::Ident = syn::parse_quote!(ProfileForm);
        assert_eq!(
            variants_enum_ident(&ident, "Variants").to_string(),
            "ProfileFormVariants"
        );
    }

    #[test]
    fn shared_field_and_variant_helpers_cover_common_attribute_args() {
        #[derive(Clone, Debug, PartialEq)]
        struct LocalItem {
            skipped: bool,
        }

        impl Skippable for LocalItem {
            fn is_skipped(&self) -> bool {
                self.skipped
            }
        }

        let field_args = FluentFieldAttributeArgs {
            skip: Some(true),
            choice: Some(true),
            value: Some(ValueAttr(syn::parse_quote!(|x: &str| x.len()))),
            arg_name: Some(syn::parse_quote!("display_name")),
        };
        assert!(field_args.is_skipped());
        assert!(field_args.is_choice());
        assert_eq!(field_args.arg_name(), Some("display_name".to_string()));
        assert!(field_args.value().is_some());

        let skipped_variant = SkippedVariantAttributeArgs { skip: Some(true) };
        assert!(skipped_variant.is_skipped());

        let keyed_variant = KeyedVariantAttributeArgs {
            skipped_args: SkippedVariantAttributeArgs { skip: Some(false) },
            key: Some("custom".to_string()),
        };
        assert!(!keyed_variant.is_skipped());
        assert_eq!(keyed_variant.key(), Some("custom"));

        let tuple_fields = darling::ast::Fields::new(
            darling::ast::Style::Tuple,
            vec![LocalItem { skipped: false }],
        );
        assert!(is_single_tuple_variant(&tuple_fields));
        assert_eq!(filtered_variant_fields(&tuple_fields).len(), 1);
        assert_eq!(all_variant_fields(&tuple_fields).len(), 1);
    }
}

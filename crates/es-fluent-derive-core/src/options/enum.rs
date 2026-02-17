use bon::Builder;
use darling::{FromDeriveInput, FromField, FromMeta, FromVariant};
use getset::Getters;
use heck::ToSnakeCase as _;
use quote::format_ident;

use crate::error::EsFluentCoreResult;

/// Options for an enum field.
#[derive(Clone, Debug, FromField, Getters)]
#[darling(attributes(fluent))]
pub struct EnumFieldOpts {
    /// The identifier of the field.
    #[getset(get = "pub")]
    ident: Option<syn::Ident>,
    /// The type of the field.
    #[getset(get = "pub")]
    ty: syn::Type,
    /// Whether to skip this field.
    #[darling(default)]
    skip: Option<bool>,
    /// Whether this field is a choice.
    #[darling(default)]
    choice: Option<bool>,
    /// A value transformation expression.
    #[darling(default)]
    value: Option<super::ValueAttr>,
}

impl EnumFieldOpts {
    /// Returns `true` if the field should be skipped.
    pub fn is_skipped(&self) -> bool {
        self.skip.unwrap_or(false)
    }
    /// Returns `true` if the field is a choice.
    pub fn is_choice(&self) -> bool {
        self.choice.unwrap_or(false)
    }
    /// Returns the value expression if present.
    pub fn value(&self) -> Option<&syn::Expr> {
        self.value.as_ref().map(|v| &v.0)
    }
}

/// Options for an enum variant.
#[derive(Clone, Debug, FromVariant, Getters)]
#[darling(attributes(fluent))]
pub struct VariantOpts {
    /// The identifier of the variant.
    #[getset(get = "pub")]
    ident: syn::Ident,
    fields: darling::ast::Fields<EnumFieldOpts>,
    /// Whether to skip this variant.
    #[darling(default)]
    skip: Option<bool>,
    /// Overrides the localization key suffix for this variant.
    #[darling(default)]
    key: Option<String>,
}

impl VariantOpts {
    /// Returns `true` if the variant should be skipped.
    pub fn is_skipped(&self) -> bool {
        self.skip.unwrap_or(false)
    }
    /// Returns the style of the variant's fields.
    pub fn style(&self) -> darling::ast::Style {
        self.fields.style
    }
    /// Returns the fields of the variant that are not skipped.
    pub fn fields(&self) -> Vec<&EnumFieldOpts> {
        self.fields
            .iter()
            .filter(|field| !field.is_skipped())
            .collect()
    }
    /// Returns all fields of the variant.
    pub fn all_fields(&self) -> Vec<&EnumFieldOpts> {
        self.fields.iter().collect()
    }
    /// Returns the explicit localization key for the variant, if provided.
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
}

/// Options for an enum.
#[derive(Clone, Debug, FromDeriveInput, Getters)]
#[darling(supports(enum_unit, enum_named, enum_tuple), attributes(fluent))]
#[getset(get = "pub")]
pub struct EnumOpts {
    /// The identifier of the enum.
    ident: syn::Ident,
    /// The generics of the enum.
    generics: syn::Generics,
    data: darling::ast::Data<VariantOpts, darling::util::Ignored>,
    #[darling(flatten)]
    attr_args: EnumFluentAttributeArgs,
}

impl EnumOpts {
    /// Returns the variants of the enum.
    pub fn variants(&self) -> Vec<&VariantOpts> {
        match &self.data {
            darling::ast::Data::Enum(variants) => variants.iter().collect(),
            _ => unreachable!("Unexpected data type for enum"),
        }
    }

    /// Returns the base localization key used for this enum.
    pub fn base_key(&self) -> String {
        if let Some(resource) = self.attr_args().resource() {
            resource.to_string()
        } else {
            self.ident().to_string().to_snake_case()
        }
    }
}

/// Attribute arguments for an enum.
#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct EnumFluentAttributeArgs {
    #[darling(default)]
    resource: Option<String>,
    /// Whether to skip inventory registration for this enum.
    /// Used by `#[es_fluent_language]` to prevent language enums from being registered.
    #[darling(default)]
    skip_inventory: Option<bool>,
    /// Optional namespace for FTL file generation.
    /// - `namespace = "name"` - writes to `{lang}/{crate}/{name}.ftl`
    /// - `namespace = file` - writes to `{lang}/{crate}/{source_file_stem}.ftl`
    /// - `namespace(file(relative))` - writes to `{lang}/{crate}/{relative_file_path}.ftl`
    /// - `namespace = folder` - writes to `{lang}/{crate}/{source_parent_folder}.ftl`
    /// - `namespace(folder(relative))` - writes to `{lang}/{crate}/{relative_parent_folder_path}.ftl`
    #[darling(default)]
    namespace: Option<super::namespace::NamespaceValue>,
}

impl EnumFluentAttributeArgs {
    /// Returns the explicit resource base key if provided.
    pub fn resource(&self) -> Option<&str> {
        self.resource.as_deref()
    }

    /// Returns `true` if inventory registration should be skipped.
    pub fn skip_inventory(&self) -> bool {
        self.skip_inventory.unwrap_or(false)
    }

    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&super::namespace::NamespaceValue> {
        self.namespace.as_ref()
    }
}

/// Options for an enum that can be used as a choice.
#[derive(Clone, Debug, FromDeriveInput, Getters)]
#[darling(supports(enum_unit), attributes(fluent_choice))]
#[getset(get = "pub")]
pub struct EnumChoiceOpts {
    /// The identifier of the enum.
    ident: syn::Ident,
    /// The generics of the enum.
    generics: syn::Generics,
    data: darling::ast::Data<darling::util::Ignored, darling::util::Ignored>,
    #[darling(flatten)]
    attr_args: EnumChoiceAttributeArgs,
}

/// Attribute arguments for an enum that can be used as a choice.
#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
#[getset(get = "pub")]
pub struct EnumChoiceAttributeArgs {
    #[darling(default)]
    serialize_all: Option<String>,
}

/// Options for an enum variant in EsFluentVariants context.
#[derive(Clone, Debug, FromVariant, Getters)]
#[darling(attributes(fluent_variants))]
pub struct EnumVariantOpts {
    /// The identifier of the variant.
    #[getset(get = "pub")]
    ident: syn::Ident,
    fields: darling::ast::Fields<darling::util::Ignored>,
    /// Whether to skip this variant.
    #[darling(default)]
    skip: Option<bool>,
}

impl EnumVariantOpts {
    /// Returns `true` if the variant should be skipped.
    pub fn is_skipped(&self) -> bool {
        self.skip.unwrap_or(false)
    }

    /// Returns the style of the variant's fields.
    pub fn style(&self) -> darling::ast::Style {
        self.fields.style
    }

    /// Returns true if this is a tuple variant with exactly one field.
    pub fn is_single_tuple(&self) -> bool {
        matches!(self.fields.style, darling::ast::Style::Tuple) && self.fields.len() == 1
    }
}

/// Options for an enum with EsFluentVariants.
#[derive(Clone, Debug, FromDeriveInput, Getters)]
#[darling(
    supports(enum_unit, enum_named, enum_tuple),
    attributes(fluent_variants)
)]
#[getset(get = "pub")]
pub struct EnumVariantsOpts {
    /// The identifier of the enum.
    ident: syn::Ident,
    /// The generics of the enum.
    generics: syn::Generics,
    data: darling::ast::Data<EnumVariantOpts, darling::util::Ignored>,
    #[darling(flatten)]
    attr_args: EnumVariantsFluentAttributeArgs,
}

impl EnumVariantsOpts {
    const FTL_ENUM_IDENT: &str = "Variants";

    /// Returns the identifier of the FTL enum.
    pub fn ftl_enum_ident(&self) -> syn::Ident {
        format_ident!("{}{}", &self.ident, Self::FTL_ENUM_IDENT)
    }

    /// Returns the identifiers of the keyed FTL enums.
    pub fn keyed_idents(&self) -> EsFluentCoreResult<Vec<syn::Ident>> {
        self.attr_args.clone().keys.map_or_else(
            || Ok(Vec::new()),
            |keys| {
                keys.into_iter()
                    .map(|key| {
                        let pascal_key = super::validate_snake_case_key(&key)?;
                        Ok(format_ident!(
                            "{}{}{}",
                            &self.ident,
                            pascal_key,
                            Self::FTL_ENUM_IDENT
                        ))
                    })
                    .collect()
            },
        )
    }

    /// Returns the identifiers used to build base FTL keys (without suffixes).
    pub fn keyed_base_idents(&self) -> EsFluentCoreResult<Vec<syn::Ident>> {
        self.attr_args.clone().keys.map_or_else(
            || Ok(Vec::new()),
            |keys| {
                keys.into_iter()
                    .map(|key| {
                        let pascal_key = super::validate_snake_case_key(&key)?;
                        Ok(format_ident!("{}{}", &self.ident, pascal_key))
                    })
                    .collect()
            },
        )
    }

    /// Returns the variants of the enum that are not skipped.
    pub fn variants(&self) -> Vec<&EnumVariantOpts> {
        match &self.data {
            darling::ast::Data::Enum(variants) => {
                variants.iter().filter(|v| !v.is_skipped()).collect()
            },
            _ => unreachable!("Unexpected data type for enum"),
        }
    }
}

/// Attribute arguments for an enum with EsFluentVariants.
#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct EnumVariantsFluentAttributeArgs {
    #[darling(default)]
    keys: Option<Vec<syn::LitStr>>,
    /// The traits to derive on the FTL enum.
    #[getset(get = "pub")]
    #[darling(default)]
    derive: darling::util::PathList,
    /// Optional namespace for FTL file generation.
    /// - `namespace = "name"` - writes to `{lang}/{crate}/{name}.ftl`
    /// - `namespace = file` - writes to `{lang}/{crate}/{source_file_stem}.ftl`
    /// - `namespace(file(relative))` - writes to `{lang}/{crate}/{relative_path}.ftl`
    /// - `namespace = folder` - writes to `{lang}/{crate}/{source_parent_folder}.ftl`
    /// - `namespace(folder(relative))` - writes to `{lang}/{crate}/{relative_parent_folder_path}.ftl`
    #[darling(default)]
    namespace: Option<super::namespace::NamespaceValue>,
}

impl EnumVariantsFluentAttributeArgs {
    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&super::namespace::NamespaceValue> {
        self.namespace.as_ref()
    }

    /// Returns the raw key strings if provided.
    pub fn key_strings(&self) -> Option<Vec<String>> {
        self.keys
            .as_ref()
            .map(|keys| keys.iter().map(|k| k.value()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::namespace::NamespaceValue;
    use quote::quote;
    use syn::{DeriveInput, parse_quote};

    #[test]
    fn enum_opts_cover_base_key_variant_helpers_and_field_flags() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            #[fluent(resource = "custom_error", skip_inventory, namespace = "errors")]
            enum StatusCode {
                Data {
                    #[fluent(choice, value = "|x: &String| x.len()")]
                    label: String,
                    #[fluent(skip)]
                    hidden: bool,
                },
                Tuple(#[fluent(skip)] u8, String),
                #[fluent(skip, key = "skipped")]
                Skipped,
            }
        };

        let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
        assert_eq!(opts.base_key(), "custom_error");
        assert!(opts.attr_args().skip_inventory());
        assert!(matches!(
            opts.attr_args().namespace(),
            Some(NamespaceValue::Literal(value)) if value == "errors"
        ));

        let variants = opts.variants();
        let data = variants
            .iter()
            .find(|variant| variant.ident().to_string() == "Data")
            .expect("Data variant should exist");
        assert_eq!(data.fields().len(), 1);
        assert_eq!(data.all_fields().len(), 2);
        assert!(data.fields()[0].is_choice());

        let value_expr = data.fields()[0]
            .value()
            .expect("value expression should be present");
        assert_eq!(
            quote!(#value_expr).to_string(),
            "| x : & String | x . len ()"
        );

        let tuple = variants
            .iter()
            .find(|variant| variant.ident().to_string() == "Tuple")
            .expect("Tuple variant should exist");
        assert_eq!(tuple.fields().len(), 1);
        assert_eq!(tuple.all_fields().len(), 2);
        assert!(tuple.fields()[0].ident().is_none());

        let skipped = variants
            .iter()
            .find(|variant| variant.ident().to_string() == "Skipped")
            .expect("Skipped variant should exist");
        assert!(skipped.is_skipped());
        assert_eq!(skipped.key(), Some("skipped"));

        let no_resource_input: DeriveInput = parse_quote! {
            enum HttpStatus {
                Ok
            }
        };
        let no_resource_opts =
            EnumOpts::from_derive_input(&no_resource_input).expect("EnumOpts should parse");
        assert_eq!(no_resource_opts.base_key(), "http_status");
    }

    #[test]
    fn enum_variants_opts_cover_key_generation_styles_and_skip_filtering() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluentVariants)]
            #[fluent_variants(
                keys = ["primary_key", "secondary_key"],
                derive(Debug),
                namespace = "ui"
            )]
            enum Status {
                One(u8),
                Two(u8, u8),
                Three { value: u8 },
                #[fluent_variants(skip)]
                Hidden,
            }
        };

        let opts =
            EnumVariantsOpts::from_derive_input(&input).expect("EnumVariantsOpts should parse");
        assert_eq!(opts.ftl_enum_ident().to_string(), "StatusVariants");

        let keyed_idents: Vec<String> = opts
            .keyed_idents()
            .expect("keyed idents should parse")
            .into_iter()
            .map(|ident| ident.to_string())
            .collect();
        assert_eq!(
            keyed_idents,
            vec!["StatusPrimaryKeyVariants", "StatusSecondaryKeyVariants"]
        );

        let keyed_base_idents: Vec<String> = opts
            .keyed_base_idents()
            .expect("keyed base idents should parse")
            .into_iter()
            .map(|ident| ident.to_string())
            .collect();
        assert_eq!(
            keyed_base_idents,
            vec!["StatusPrimaryKey", "StatusSecondaryKey"]
        );
        assert_eq!(
            opts.attr_args().key_strings(),
            Some(vec!["primary_key".to_string(), "secondary_key".to_string()])
        );
        assert!(matches!(
            opts.attr_args().namespace(),
            Some(NamespaceValue::Literal(value)) if value == "ui"
        ));

        let variants = opts.variants();
        assert_eq!(variants.len(), 3, "Skipped variants should be filtered");

        let one = variants
            .iter()
            .find(|variant| variant.ident().to_string() == "One")
            .expect("One variant should exist");
        assert!(matches!(one.style(), darling::ast::Style::Tuple));
        assert!(one.is_single_tuple());

        let two = variants
            .iter()
            .find(|variant| variant.ident().to_string() == "Two")
            .expect("Two variant should exist");
        assert!(matches!(two.style(), darling::ast::Style::Tuple));
        assert!(!two.is_single_tuple());

        let three = variants
            .iter()
            .find(|variant| variant.ident().to_string() == "Three")
            .expect("Three variant should exist");
        assert!(matches!(three.style(), darling::ast::Style::Struct));
        assert!(!three.is_single_tuple());
    }

    #[test]
    fn enum_variants_opts_key_methods_cover_empty_and_invalid_keys() {
        let no_key_input: DeriveInput = parse_quote! {
            #[derive(EsFluentVariants)]
            enum Plain {
                Only
            }
        };
        let no_key_opts =
            EnumVariantsOpts::from_derive_input(&no_key_input).expect("parse without keys");
        assert!(no_key_opts.keyed_idents().expect("keyed_idents").is_empty());
        assert!(
            no_key_opts
                .keyed_base_idents()
                .expect("keyed_base_idents")
                .is_empty()
        );

        let invalid_key_input: DeriveInput = parse_quote! {
            #[derive(EsFluentVariants)]
            #[fluent_variants(keys = ["NotSnake"])]
            enum Invalid {
                A
            }
        };
        let invalid_opts =
            EnumVariantsOpts::from_derive_input(&invalid_key_input).expect("input should parse");

        let idents_err = invalid_opts
            .keyed_idents()
            .expect_err("invalid key should fail");
        assert!(idents_err.to_string().contains("lowercase snake_case"));

        let base_err = invalid_opts
            .keyed_base_idents()
            .expect_err("invalid key should fail");
        assert!(base_err.to_string().contains("lowercase snake_case"));
    }

    #[test]
    fn enum_methods_panic_on_unexpected_internal_shapes() {
        let enum_input: DeriveInput = parse_quote! {
            enum InternalShape {
                A
            }
        };
        let mut enum_opts = EnumOpts::from_derive_input(&enum_input).expect("EnumOpts parse");
        enum_opts.data = darling::ast::Data::Struct(darling::ast::Fields::new(
            darling::ast::Style::Unit,
            Vec::<darling::util::Ignored>::new(),
        ));

        let variants_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = enum_opts.variants();
        }));
        assert!(variants_result.is_err());

        let variants_input: DeriveInput = parse_quote! {
            #[derive(EsFluentVariants)]
            enum InternalVariantsShape {
                A
            }
        };
        let mut variants_opts =
            EnumVariantsOpts::from_derive_input(&variants_input).expect("EnumVariantsOpts parse");
        variants_opts.data = darling::ast::Data::Struct(darling::ast::Fields::new(
            darling::ast::Style::Unit,
            Vec::<darling::util::Ignored>::new(),
        ));

        let filtered_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = variants_opts.variants();
        }));
        assert!(filtered_result.is_err());
    }
}

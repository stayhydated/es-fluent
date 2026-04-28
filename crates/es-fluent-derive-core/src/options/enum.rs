use bon::Builder;
use darling::{FromDeriveInput, FromMeta, FromVariant};
use getset::Getters;
use heck::ToSnakeCase as _;

use crate::options::{
    EnumDataOptions, FilteredEnumDataOptions, GeneratedVariantsOptions, KeyedVariant, Skippable,
    VariantFields,
};

/// Options for an enum variant.
#[derive(Clone, Debug, FromVariant, Getters)]
#[darling(attributes(fluent))]
pub struct VariantOpts {
    /// The identifier of the variant.
    #[getset(get = "pub")]
    ident: syn::Ident,
    fields: darling::ast::Fields<super::FluentFieldOpts>,
    #[darling(flatten)]
    attr_args: super::KeyedVariantAttributeArgs,
}

impl VariantFields for VariantOpts {
    type Field = super::FluentFieldOpts;

    fn variant_fields(&self) -> &darling::ast::Fields<Self::Field> {
        &self.fields
    }
}

impl KeyedVariant for VariantOpts {
    fn key(&self) -> Option<&str> {
        self.attr_args.key()
    }
}

impl Skippable for VariantOpts {
    fn is_skipped(&self) -> bool {
        self.attr_args.is_skipped()
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
    attr_args: FluentEnumAttributeArgs,
}

impl EnumOpts {
    /// Returns the base localization key used for this enum.
    pub fn base_key(&self) -> String {
        if let Some(resource) = self.attr_args().resource() {
            resource.to_string()
        } else {
            self.ident().to_string().to_snake_case()
        }
    }
}

impl EnumDataOptions for EnumOpts {
    type Variant = VariantOpts;

    fn enum_data(&self) -> &darling::ast::Data<Self::Variant, darling::util::Ignored> {
        &self.data
    }
}

/// Attribute arguments for an enum.
#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct FluentEnumAttributeArgs {
    #[darling(default)]
    resource: Option<String>,
    #[darling(default)]
    domain: Option<String>,
    /// Whether to skip inventory registration for this enum.
    /// Used by `#[es_fluent_language]` to prevent language enums from being registered.
    #[darling(default)]
    skip_inventory: Option<bool>,
    #[darling(flatten)]
    namespace_args: super::NamespacedAttributeArgs,
}

impl FluentEnumAttributeArgs {
    /// Returns the explicit resource base key if provided.
    pub fn resource(&self) -> Option<&str> {
        self.resource.as_deref()
    }

    /// Returns the explicit lookup domain override if provided.
    pub fn domain(&self) -> Option<&str> {
        self.domain.as_deref()
    }

    /// Returns `true` if inventory registration should be skipped.
    pub fn skip_inventory(&self) -> bool {
        self.skip_inventory.unwrap_or(false)
    }

    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&super::namespace::NamespaceValue> {
        self.namespace_args.namespace()
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
    #[darling(flatten)]
    attr_args: super::SkippedVariantAttributeArgs,
}

impl VariantFields for EnumVariantOpts {
    type Field = darling::util::Ignored;

    fn variant_fields(&self) -> &darling::ast::Fields<Self::Field> {
        &self.fields
    }
}

impl Skippable for EnumVariantOpts {
    fn is_skipped(&self) -> bool {
        self.attr_args.is_skipped()
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
    attr_args: super::VariantsFluentAttributeArgs,
}

impl FilteredEnumDataOptions for EnumVariantsOpts {
    type Variant = EnumVariantOpts;

    fn enum_data(&self) -> &darling::ast::Data<Self::Variant, darling::util::Ignored> {
        &self.data
    }
}

impl GeneratedVariantsOptions for EnumVariantsOpts {
    fn variants_ident(&self) -> &syn::Ident {
        &self.ident
    }

    fn variants_attr_args(&self) -> &super::VariantsFluentAttributeArgs {
        &self.attr_args
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::FluentField as _;
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
        assert_eq!(opts.attr_args().domain(), None);
        assert!(opts.attr_args().skip_inventory());
        assert!(matches!(
            opts.attr_args().namespace(),
            Some(NamespaceValue::Literal(value)) if value == "errors"
        ));

        let variants = opts.variants();
        let data = variants
            .iter()
            .find(|variant| *variant.ident() == "Data")
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
            .find(|variant| *variant.ident() == "Tuple")
            .expect("Tuple variant should exist");
        assert_eq!(tuple.fields().len(), 1);
        assert_eq!(tuple.all_fields().len(), 2);
        assert!(tuple.fields()[0].ident().is_none());

        let skipped = variants
            .iter()
            .find(|variant| *variant.ident() == "Skipped")
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

        let domain_input: DeriveInput = parse_quote! {
            #[fluent(resource = "custom_error", domain = "shared-errors")]
            enum DomainLinked {
                A
            }
        };
        let domain_opts = EnumOpts::from_derive_input(&domain_input).expect("domain parse");
        assert_eq!(domain_opts.base_key(), "custom_error");
        assert_eq!(domain_opts.attr_args().domain(), Some("shared-errors"));
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
            .find(|variant| *variant.ident() == "One")
            .expect("One variant should exist");
        assert!(matches!(one.style(), darling::ast::Style::Tuple));
        assert!(one.is_single_tuple());

        let two = variants
            .iter()
            .find(|variant| *variant.ident() == "Two")
            .expect("Two variant should exist");
        assert!(matches!(two.style(), darling::ast::Style::Tuple));
        assert!(!two.is_single_tuple());

        let three = variants
            .iter()
            .find(|variant| *variant.ident() == "Three")
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

    #[test]
    fn enum_field_arg_name_parse_for_single_tuple_variant() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            enum TupleNames {
                Something(#[fluent(arg_name = "value")] String),
            }
        };

        let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
        let variants = opts.variants();
        let variant = variants
            .iter()
            .find(|v| *v.ident() == "Something")
            .expect("Something variant should exist");

        let fields = variant.all_fields();
        let field_arg_name = fields[0].arg_name().expect("field arg_name should parse");
        assert_eq!(field_arg_name, "value".to_string());
    }

    #[test]
    fn enum_variant_arg_name_is_rejected() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            enum TupleNames {
                #[fluent(arg_name = "value")]
                Something(String),
            }
        };

        let err =
            EnumOpts::from_derive_input(&input).expect_err("variant-level arg_name is removed");
        assert!(err.to_string().contains("arg_name"));
    }
}

use crate::attribute::{AttributeLocation, AttributeName, validate_attribute_for_location};
use crate::options::{
    EnumDataOptions, FilteredEnumDataOptions, GeneratedVariantsOptions, KeyedVariant, Skippable,
    VariantFields,
};
use crate::{
    error::{AttrContext, EsFluentCoreResult},
    semantic::{
        DomainName, FluentMessageId, SpannedValue, VariantKey, spanned_message_id_from_value,
    },
};
use bon::Builder;
use darling::{FromDeriveInput, FromMeta, FromVariant};
use es_fluent_shared::{namer, namespace::NamespaceRule};
use getset::Getters;

/// Options for an enum variant.
#[derive(Clone, Debug, Getters)]
pub struct VariantOpts {
    /// The identifier of the variant.
    #[getset(get = "pub")]
    ident: syn::Ident,
    fields: darling::ast::Fields<super::FluentFieldOpts>,
    attr_args: super::KeyedVariantAttributeArgs,
}

#[derive(Clone, Debug, FromVariant, Getters)]
#[darling(attributes(fluent))]
struct RawVariantOpts {
    /// The identifier of the variant.
    #[getset(get = "pub")]
    ident: syn::Ident,
    fields: darling::ast::Fields<super::FluentFieldOpts>,
    #[darling(flatten)]
    attr_args: super::KeyedVariantAttributeArgs,
}

impl FromVariant for VariantOpts {
    fn from_variant(variant: &syn::Variant) -> darling::Result<Self> {
        validate_variant_fluent_attribute_context(variant)?;

        let raw = RawVariantOpts::from_variant(variant)?;
        Ok(Self {
            ident: raw.ident,
            fields: raw.fields,
            attr_args: raw.attr_args,
        })
    }
}

impl VariantOpts {
    /// Returns the explicit variant key suffix as a typed value if provided.
    pub fn variant_key(
        &self,
        context: AttrContext,
    ) -> EsFluentCoreResult<Option<SpannedValue<VariantKey>>> {
        self.attr_args.variant_key(context)
    }
}

fn validate_variant_fluent_attribute_context(variant: &syn::Variant) -> darling::Result<()> {
    for attr in &variant.attrs {
        validate_attribute_for_location(
            attr,
            AttributeName::Fluent,
            AttributeLocation::EnumVariant,
            Some(&variant.ident),
        )
        .map_err(|error| darling::Error::custom(error.to_string()).with_span(attr))?;
    }

    Ok(())
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
        if let Some(resource) = self.attr_args().resource_message_id() {
            resource.value().as_str().to_string()
        } else {
            namer::FluentKey::from(self.ident()).to_string()
        }
    }

    /// Returns the base localization key as a typed Fluent message id.
    pub fn base_message_id(
        &self,
        context: AttrContext,
    ) -> EsFluentCoreResult<SpannedValue<FluentMessageId>> {
        if let Some(resource) = self.attr_args().resource_message_id() {
            return Ok(resource.clone());
        }

        spanned_message_id_from_value(
            namer::FluentKey::from(self.ident()).to_string(),
            self.ident().span(),
            context,
        )
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
    resource: Option<SpannedValue<FluentMessageId>>,
    #[darling(default)]
    domain: Option<SpannedValue<DomainName>>,
    #[darling(flatten)]
    namespace_args: super::NamespacedAttributeArgs,
}

impl FluentEnumAttributeArgs {
    /// Returns the span of the explicit resource base key if provided.
    pub fn resource_span(&self) -> Option<proc_macro2::Span> {
        self.resource.as_ref().map(SpannedValue::span)
    }

    /// Returns the typed explicit resource base key if provided.
    pub fn resource_message_id(&self) -> Option<&SpannedValue<FluentMessageId>> {
        self.resource.as_ref()
    }

    /// Returns the typed explicit lookup domain if provided.
    pub fn domain_name(&self) -> Option<&SpannedValue<DomainName>> {
        self.domain.as_ref()
    }

    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&NamespaceRule> {
        self.namespace_args.namespace()
    }

    /// Returns the span of the namespace value if provided.
    pub fn namespace_span(&self) -> Option<proc_macro2::Span> {
        self.namespace_args.namespace_span()
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
    rename_all: Option<String>,
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
    use es_fluent_shared::namespace::NamespaceRule;
    use quote::quote;
    use syn::{DeriveInput, parse_quote};

    #[test]
    fn enum_opts_cover_base_key_variant_helpers_and_field_flags() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            #[fluent(resource = "custom_error", namespace = "errors")]
            enum StatusCode {
                Data {
                    #[fluent(choice)]
                    label: String,
                    #[fluent(value = |x: &String| x.len())]
                    display: String,
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
        assert_eq!(
            opts.attr_args()
                .resource_message_id()
                .expect("resource")
                .value()
                .as_str(),
            "custom_error"
        );
        assert!(opts.attr_args().domain_name().is_none());
        assert!(matches!(
            opts.attr_args().namespace(),
            Some(NamespaceRule::Literal(value)) if value == "errors"
        ));

        let variants = opts.variants();
        let data = variants
            .iter()
            .find(|variant| *variant.ident() == "Data")
            .expect("Data variant should exist");
        assert_eq!(data.fields().len(), 2);
        assert_eq!(data.all_fields().len(), 3);
        assert!(data.fields()[0].is_choice());

        let value_expr = data.fields()[1]
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
        assert_eq!(
            skipped
                .variant_key(crate::error::AttrContext::EnumVariant)
                .expect("variant key")
                .expect("key")
                .value()
                .as_str(),
            "skipped"
        );

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
        assert_eq!(
            domain_opts
                .attr_args()
                .domain_name()
                .expect("domain")
                .value()
                .as_str(),
            "shared-errors"
        );
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
        let key_names: Vec<_> = opts
            .attr_args()
            .keys()
            .expect("typed keys")
            .iter()
            .map(|key| key.value().as_str())
            .collect();
        assert_eq!(key_names, vec!["primary_key", "secondary_key"]);
        assert!(matches!(
            opts.attr_args().namespace(),
            Some(NamespaceRule::Literal(value)) if value == "ui"
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
        let err = EnumVariantsOpts::from_derive_input(&invalid_key_input)
            .expect_err("invalid key should fail during parsing");
        assert!(err.to_string().contains("lowercase snake_case"));

        let duplicate_key_input: DeriveInput = parse_quote! {
            #[derive(EsFluentVariants)]
            #[fluent_variants(keys = ["label", "label"])]
            enum Duplicate {
                A
            }
        };
        let err = EnumVariantsOpts::from_derive_input(&duplicate_key_input)
            .expect_err("duplicate keys should fail during parsing");
        assert!(err.to_string().contains("duplicate key 'label'"));
    }

    #[test]
    fn lowered_enum_models_reject_unexpected_internal_shapes() {
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

        let err = crate::lowered::MessageEnumModel::from_options(&enum_opts)
            .expect_err("lowering rejects wrong data shape");
        assert!(err.to_string().contains("must contain enum data"));

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

        let err = crate::lowered::GeneratedVariantsEnumModel::from_options(&variants_opts)
            .expect_err("lowering rejects wrong data shape");
        assert!(err.to_string().contains("must contain enum data"));
    }

    #[test]
    fn enum_field_arg_parse_for_single_tuple_variant() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            enum TupleNames {
                Something(#[fluent(arg = "value")] String),
            }
        };

        let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
        let variants = opts.variants();
        let variant = variants
            .iter()
            .find(|v| *v.ident() == "Something")
            .expect("Something variant should exist");

        let fields = variant.all_fields();
        let field_arg = fields[0]
            .arg_name(crate::error::AttrContext::MessageField)
            .expect("field arg should parse")
            .expect("field arg");
        assert_eq!(field_arg.value().as_str(), "value");
    }

    #[test]
    fn enum_variant_arg_is_rejected() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            enum TupleNames {
                #[fluent(arg = "value")]
                Something(String),
            }
        };

        let err = EnumOpts::from_derive_input(&input).expect_err("variant-level arg is removed");
        let message = err.to_string();
        assert!(message.contains("field-only attribute"));
        assert!(message.contains("enum variant `Something`"));
    }
}

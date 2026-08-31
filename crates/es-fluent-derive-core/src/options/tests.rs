use super::*;
use crate::error::AttrContext;
use crate::options::r#struct::StructOpts;
use crate::semantic::{GeneratedKeyName, SpannedValue};
use darling::{FromDeriveInput as _, FromField as _, FromMeta as _, FromVariant as _};

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
fn bare_flag_parser_rejects_non_bare_shapes() {
    let input: syn::DeriveInput = syn::parse_quote! {
        struct Message {
            #[fluent(skip("hidden"))]
            hidden: String,
        }
    };

    let err = match StructOpts::from_derive_input(&input) {
        Ok(_) => panic!("non-bare attribute shapes should not parse as bare flags"),
        Err(error) => error,
    };

    assert!(!err.to_string().is_empty());
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
    let transformed_field = FluentFieldOpts::from_field(&transformed_field).expect("field parse");
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
        #[fluent(skip)]
        Skipped
    );
    let skipped_variant =
        crate::options::r#enum::VariantOpts::from_variant(&skipped_variant).expect("variant parse");
    assert!(skipped_variant.directive().is_skipped());

    let invalid_skipped_variant: syn::Variant = syn::parse_quote!(
        #[fluent(skip, key = "skipped")]
        Skipped
    );
    let err = crate::options::r#enum::VariantOpts::from_variant(&invalid_skipped_variant)
        .expect_err("skip and key should conflict");
    assert!(
        err.to_string()
            .contains("Cannot use #[fluent(key = \"...\")] on a skipped variant")
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
}

#[test]
fn field_directive_infers_optional_strategy_for_option_fields() {
    let field: syn::Field = syn::parse_quote! {
        maybe_name: Option<String>
    };
    let opts = FluentFieldOpts::from_field(&field).expect("option field should parse");

    let Some(FieldValueDirective::Optional { inner_ty, .. }) = opts
        .directive()
        .argument()
        .map(FieldArgumentDirective::value)
    else {
        panic!("Option<T> should infer optional argument handling");
    };

    assert_eq!(quote::quote!(#inner_ty).to_string(), "String");

    let transformed: syn::Field = syn::parse_quote! {
        #[fluent(value = |value: &Option<String>| value.is_some())]
        maybe_name: Option<String>
    };
    let opts = FluentFieldOpts::from_field(&transformed)
        .expect("explicit value transform should override Option inference");
    assert!(matches!(
        opts.directive()
            .argument()
            .map(FieldArgumentDirective::value),
        Some(FieldValueDirective::Transform(_))
    ));
}

#[test]
fn field_directive_infers_optional_choice_strategy_for_option_selectors() {
    let field: syn::Field = syn::parse_quote! {
        #[fluent(selector)]
        maybe_status: Option<Status>
    };
    let opts = FluentFieldOpts::from_field(&field).expect("option selector should parse");

    let Some(FieldValueDirective::OptionalChoice { inner_ty, .. }) = opts
        .directive()
        .argument()
        .map(FieldArgumentDirective::value)
    else {
        panic!("Option<T> selector should infer optional choice handling");
    };

    assert_eq!(quote::quote!(#inner_ty).to_string(), "Status");
}

//! Tests for validation functions.

use darling::FromDeriveInput as _;
use es_fluent_derive_core::expansion::EsFluentExpansion;
use es_fluent_derive_core::options::r#enum::EnumOpts;
use es_fluent_derive_core::options::r#struct::StructOpts;
use syn::{DeriveInput, parse_quote};

#[test]
fn validate_empty_struct_succeeds() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        pub struct EmptyStruct;
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    es_fluent_derive_core::validation::validate_struct(&opts).expect("Validation should succeed");
}

#[test]
fn validate_empty_named_struct_succeeds() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        pub struct EmptyStruct {}
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    es_fluent_derive_core::validation::validate_struct(&opts).expect("Validation should succeed");
}

#[test]
fn parse_rejects_variant_level_arg() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        pub enum TestEnum {
            #[fluent(arg = "value")]
            Something(String),
        }
    };

    let err = EsFluentExpansion::from_derive_input(&input).expect_err("Expected validation error");
    let message = err.to_string();
    assert!(message.contains("field-only attribute"));
    assert!(message.contains("enum variant `Something`"));
}

#[test]
fn validate_enum_field_arg_on_named_variant_succeeds() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        pub enum TestEnum {
            Named {
                #[fluent(arg = "display_value")]
                value: String,
            },
        }
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    es_fluent_derive_core::validation::validate_enum(&opts).expect("Validation should succeed");
}

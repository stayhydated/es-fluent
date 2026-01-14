//! Tests for validation functions.

use darling::FromDeriveInput;
use es_fluent_derive_core::options::r#struct::StructOpts;
use es_fluent_derive_core::validation::validate_struct;
use syn::{DeriveInput, parse_quote};

#[test]
fn validate_struct_multiple_defaults_produces_expected_error_message() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        pub struct TestStruct {
            #[fluent(default)]
            field1: String,
            #[fluent(default)]
            field2: i32,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    let err = validate_struct(&opts).expect_err("Expected validation error");

    insta::assert_debug_snapshot!(
        "validate_struct_multiple_defaults_produces_expected_error_message",
        err.to_string()
    );
}

#[test]
fn validate_struct_multiple_defaults_tuple_struct_produces_expected_error_message() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        pub struct TestTupleStruct(
            #[fluent(default)]
            String,
            #[fluent(default)]
            i32,
        );
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    let err = validate_struct(&opts).expect_err("Expected validation error");

    insta::assert_debug_snapshot!(
        "validate_struct_multiple_defaults_tuple_struct_produces_expected_error_message",
        err.to_string()
    );
}

#[test]
fn validate_empty_struct_succeeds() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        pub struct EmptyStruct;
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    validate_struct(&opts).expect("Validation should succeed");
}

#[test]
fn validate_empty_named_struct_succeeds() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        pub struct EmptyStruct {}
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    validate_struct(&opts).expect("Validation should succeed");
}

#[test]
fn validate_struct_single_default_succeeds() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        pub struct TestStruct {
            #[fluent(default)]
            field1: String,
            field2: i32,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    validate_struct(&opts).expect("Validation should succeed");
}

#[test]
fn validate_struct_skip_and_default_conflict_produces_error() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        pub struct TestStruct {
            #[fluent(skip, default)]
            field1: String,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    let err = validate_struct(&opts).expect_err("Expected validation error");

    insta::assert_debug_snapshot!(
        "validate_struct_skip_and_default_conflict_produces_error",
        err.to_string()
    );
}

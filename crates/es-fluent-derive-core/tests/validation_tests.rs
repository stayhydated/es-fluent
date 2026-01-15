//! Tests for validation functions.

use darling::FromDeriveInput;
use es_fluent_derive_core::options::r#struct::StructOpts;
use es_fluent_derive_core::validation::validate_struct;
use syn::{DeriveInput, parse_quote};

mod validate_struct_tests {
    use super::*;

    #[test]
    fn multiple_defaults_produces_error() {
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

        let err_msg = err.to_string();
        assert!(err_msg.contains("multiple fields"));
        assert!(err_msg.contains("field1"));
    }

    #[test]
    fn multiple_defaults_tuple_struct_produces_error() {
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

        let err_msg = err.to_string();
        assert!(err_msg.contains("multiple fields"));
    }

    #[test]
    fn empty_unit_struct_succeeds() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct EmptyStruct;
        };

        let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
        validate_struct(&opts).expect("Validation should succeed");
    }

    #[test]
    fn empty_named_struct_succeeds() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct EmptyStruct {}
        };

        let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
        validate_struct(&opts).expect("Validation should succeed");
    }

    #[test]
    fn single_default_succeeds() {
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
    fn skip_and_default_conflict_produces_error() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct TestStruct {
                #[fluent(skip, default)]
                field1: String,
            }
        };

        let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
        let err = validate_struct(&opts).expect_err("Expected validation error");

        let err_msg = err.to_string();
        assert!(err_msg.contains("skip"));
        assert!(err_msg.contains("default"));
    }

    #[test]
    fn no_defaults_succeeds() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct TestStruct {
                field1: String,
                field2: i32,
            }
        };

        let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
        validate_struct(&opts).expect("Validation should succeed");
    }
}

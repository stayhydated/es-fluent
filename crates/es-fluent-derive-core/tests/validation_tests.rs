//! Tests for validation functions.

use darling::FromDeriveInput;
use es_fluent_derive_core::options::r#enum::EnumOpts;
use es_fluent_derive_core::options::namespace::NamespaceValue;
use es_fluent_derive_core::options::r#struct::StructOpts;
use es_fluent_derive_core::validation::{
    validate_enum, validate_namespace, validate_namespace_against_allowed, validate_struct,
};
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

    #[test]
    fn arg_name_on_named_struct_field_succeeds() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct TestStruct {
                #[fluent(arg_name = "display_name")]
                name: String,
                value: String,
            }
        };

        let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
        validate_struct(&opts).expect("Validation should succeed");
    }

    #[test]
    fn duplicate_struct_arg_name_fails() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct TestStruct {
                #[fluent(arg_name = "value")]
                first: String,
                value: String,
            }
        };

        let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
        let err = validate_struct(&opts).expect_err("Expected validation error");
        let err_msg = err.to_string();
        assert!(err_msg.contains("duplicate argument name"));
        assert!(err_msg.contains("value"));
    }

    #[test]
    fn arg_name_on_skipped_field_fails() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct TestStruct {
                #[fluent(skip, arg_name = "hidden")]
                hidden: String,
            }
        };

        let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
        let err = validate_struct(&opts).expect_err("Expected validation error");
        let err_msg = err.to_string();
        assert!(err_msg.contains("arg_name"));
        assert!(err_msg.contains("skipped"));
    }
}

mod validate_enum_tests {
    use super::*;

    #[test]
    fn field_arg_name_on_single_tuple_variant_succeeds() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub enum TestEnum {
                Something(#[fluent(arg_name = "value")] String),
            }
        };

        let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
        validate_enum(&opts).expect("Single tuple field with field-level arg_name should pass");
    }

    #[test]
    fn field_arg_name_on_named_variant_succeeds() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub enum TestEnum {
                Named {
                    #[fluent(arg_name = "display_value")]
                    value: String,
                },
            }
        };

        let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
        validate_enum(&opts).expect("Named field with field-level arg_name should pass");
    }

    #[test]
    fn variant_level_arg_name_is_rejected() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub enum TestEnum {
                #[fluent(arg_name = "value")]
                Something(String),
            }
        };

        let err = EnumOpts::from_derive_input(&input).expect_err("Expected parse error");
        let err_msg = err.to_string();

        assert!(err_msg.contains("arg_name"));
    }

    #[test]
    fn field_arg_name_duplicate_with_named_field_fails() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub enum TestEnum {
                Named {
                    #[fluent(arg_name = "value")]
                    left: String,
                    value: String,
                },
            }
        };

        let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
        let err = validate_enum(&opts).expect_err("Expected validation error");
        let err_msg = err.to_string();

        assert!(err_msg.contains("duplicate resolved argument name"));
        assert!(err_msg.contains("value"));
    }

    #[test]
    fn duplicate_field_arg_name_overrides_fail() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub enum TestEnum {
                Something(
                    #[fluent(arg_name = "same")] String,
                    #[fluent(arg_name = "same")] String,
                ),
            }
        };

        let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
        let err = validate_enum(&opts).expect_err("Expected validation error");
        let err_msg = err.to_string();

        assert!(err_msg.contains("duplicate"));
        assert!(err_msg.contains("same"));
    }
}

mod validate_namespace_tests {
    use super::*;

    #[test]
    fn file_namespace_always_passes() {
        // File-based namespaces are deferred to CLI validation
        let ns = NamespaceValue::File;
        validate_namespace(&ns, None).expect("File namespace should always pass at compile time");
    }

    #[test]
    fn file_relative_namespace_always_passes() {
        // File-based namespaces are deferred to CLI validation
        let ns = NamespaceValue::FileRelative;
        validate_namespace(&ns, None)
            .expect("FileRelative namespace should always pass at compile time");
    }

    #[test]
    fn folder_namespace_always_passes() {
        // Folder-based namespaces are deferred to CLI validation
        let ns = NamespaceValue::Folder;
        validate_namespace(&ns, None).expect("Folder namespace should always pass at compile time");
    }

    #[test]
    fn folder_relative_namespace_always_passes() {
        // Folder-based namespaces are deferred to CLI validation
        let ns = NamespaceValue::FolderRelative;
        validate_namespace(&ns, None)
            .expect("FolderRelative namespace should always pass at compile time");
    }

    #[test]
    fn literal_namespace_passes_without_config() {
        // When no i18n.toml exists (or CARGO_MANIFEST_DIR is not set),
        // validation should pass for any literal namespace.
        // This test runs without setting up a config file, so it relies on
        // the graceful fallback behavior.
        let ns = NamespaceValue::Literal("any_namespace".into());
        // This will pass because there's no i18n.toml in the test environment
        // or the config doesn't have namespaces configured
        validate_namespace(&ns, None).expect("Should pass when no config exists");
    }
}

mod validate_namespace_against_allowed_tests {
    use super::*;

    #[test]
    fn valid_namespace_in_allowed_list() {
        let allowed = vec![
            "ui".to_string(),
            "errors".to_string(),
            "components".to_string(),
        ];

        validate_namespace_against_allowed("ui", &allowed, None).expect("'ui' should be allowed");
        validate_namespace_against_allowed("errors", &allowed, None)
            .expect("'errors' should be allowed");
        validate_namespace_against_allowed("components", &allowed, None)
            .expect("'components' should be allowed");
    }

    #[test]
    fn invalid_namespace_not_in_allowed_list() {
        let allowed = vec!["ui".to_string(), "errors".to_string()];

        let err = validate_namespace_against_allowed("unknown", &allowed, None)
            .expect_err("'unknown' should not be allowed");

        let msg = err.to_string();
        assert!(msg.contains("unknown"));
        assert!(msg.contains("not in the allowed list"));
        // Check help message contains allowed namespaces
        assert!(msg.contains("ui"));
        assert!(msg.contains("errors"));
    }

    #[test]
    fn empty_allowed_list_rejects_everything() {
        let allowed: Vec<String> = vec![];

        let err = validate_namespace_against_allowed("anything", &allowed, None)
            .expect_err("Empty allowed list should reject all namespaces");

        assert!(err.to_string().contains("not in the allowed list"));
    }

    #[test]
    fn namespace_matching_is_case_sensitive() {
        let allowed = vec!["UI".to_string(), "Errors".to_string()];

        // Exact case should pass
        validate_namespace_against_allowed("UI", &allowed, None)
            .expect("'UI' should match exactly");

        // Different case should fail
        let err = validate_namespace_against_allowed("ui", &allowed, None)
            .expect_err("'ui' should not match 'UI'");
        assert!(err.to_string().contains("ui"));
    }

    #[test]
    fn namespace_with_special_characters() {
        let allowed = vec![
            "my-namespace".to_string(),
            "my_namespace".to_string(),
            "my.namespace".to_string(),
        ];

        validate_namespace_against_allowed("my-namespace", &allowed, None)
            .expect("hyphenated namespace should be allowed");
        validate_namespace_against_allowed("my_namespace", &allowed, None)
            .expect("underscored namespace should be allowed");
        validate_namespace_against_allowed("my.namespace", &allowed, None)
            .expect("dotted namespace should be allowed");
    }
}

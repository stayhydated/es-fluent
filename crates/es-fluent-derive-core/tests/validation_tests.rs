//! Tests for validation functions.

use darling::FromDeriveInput as _;
use es_fluent_derive_core::options::r#enum::EnumOpts;
use es_fluent_derive_core::options::r#struct::StructOpts;
use es_fluent_shared::namespace::NamespaceRule;
use insta::assert_snapshot;
use std::path::Path;
use syn::{DeriveInput, parse_quote};

fn with_manifest_dir<T>(manifest_dir: Option<&std::path::Path>, f: impl FnOnce() -> T) -> T {
    temp_env::with_var("CARGO_MANIFEST_DIR", manifest_dir, f)
}

fn normalize_temp_paths(text: &str, manifest_dir: &Path) -> String {
    let manifest = manifest_dir.to_string_lossy();
    let manifest_escaped = manifest.replace('\\', "\\\\");
    let config = manifest_dir.join("i18n.toml");
    let config = config.to_string_lossy();
    let config_escaped = config.replace('\\', "\\\\");

    text.replace(config.as_ref(), "<manifest-dir>/i18n.toml")
        .replace(config_escaped.as_str(), "<manifest-dir>/i18n.toml")
        .replace(manifest.as_ref(), "<manifest-dir>")
        .replace(manifest_escaped.as_str(), "<manifest-dir>")
}

mod validate_struct_tests {
    use super::*;

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
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
        let err = es_fluent_derive_core::validation::validate_struct(&opts)
            .expect_err("Expected validation error");

        assert_snapshot!(
            "validate_struct_multiple_defaults_produces_error",
            err.to_string()
        );
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
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
        let err = es_fluent_derive_core::validation::validate_struct(&opts)
            .expect_err("Expected validation error");

        assert_snapshot!(
            "validate_struct_multiple_defaults_tuple_struct_produces_error",
            err.to_string()
        );
    }

    #[test]
    fn empty_unit_struct_succeeds() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct EmptyStruct;
        };

        let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
        es_fluent_derive_core::validation::validate_struct(&opts)
            .expect("Validation should succeed");
    }

    #[test]
    fn empty_named_struct_succeeds() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct EmptyStruct {}
        };

        let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
        es_fluent_derive_core::validation::validate_struct(&opts)
            .expect("Validation should succeed");
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
        es_fluent_derive_core::validation::validate_struct(&opts)
            .expect("Validation should succeed");
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn skip_and_default_conflict_produces_error() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct TestStruct {
                #[fluent(skip, default)]
                field1: String,
            }
        };

        let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
        let err = es_fluent_derive_core::validation::validate_struct(&opts)
            .expect_err("Expected validation error");

        assert_snapshot!(
            "validate_struct_skip_and_default_conflict_produces_error",
            err.to_string()
        );
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
        es_fluent_derive_core::validation::validate_struct(&opts)
            .expect("Validation should succeed");
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
        es_fluent_derive_core::validation::validate_struct(&opts)
            .expect("Validation should succeed");
    }

    #[test]
    fn empty_struct_arg_name_fails() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct TestStruct {
                #[fluent(arg_name = "")]
                value: String,
            }
        };

        let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
        let err = es_fluent_derive_core::validation::validate_struct(&opts)
            .expect_err("empty arg_name should fail");

        assert!(err.to_string().contains("cannot be empty"));
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
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
        let err = es_fluent_derive_core::validation::validate_struct(&opts)
            .expect_err("Expected validation error");
        assert_snapshot!("validate_struct_duplicate_arg_name_fails", err.to_string());
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn arg_name_on_skipped_field_fails() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct TestStruct {
                #[fluent(skip, arg_name = "hidden")]
                hidden: String,
            }
        };

        let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
        let err = es_fluent_derive_core::validation::validate_struct(&opts)
            .expect_err("Expected validation error");
        assert_snapshot!(
            "validate_struct_arg_name_on_skipped_field_fails",
            err.to_string()
        );
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
        es_fluent_derive_core::validation::validate_enum(&opts)
            .expect("Single tuple field with field-level arg_name should pass");
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
        es_fluent_derive_core::validation::validate_enum(&opts)
            .expect("Named field with field-level arg_name should pass");
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn variant_level_arg_name_is_rejected() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub enum TestEnum {
                #[fluent(arg_name = "value")]
                Something(String),
            }
        };

        let err = EnumOpts::from_derive_input(&input).expect_err("Expected parse error");
        assert_snapshot!(
            "validate_enum_variant_level_arg_name_is_rejected",
            err.to_string()
        );
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
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
        let err = es_fluent_derive_core::validation::validate_enum(&opts)
            .expect_err("Expected validation error");
        assert_snapshot!(
            "validate_enum_field_arg_name_duplicate_with_named_field_fails",
            err.to_string()
        );
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
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
        let err = es_fluent_derive_core::validation::validate_enum(&opts)
            .expect_err("Expected validation error");
        assert_snapshot!(
            "validate_enum_duplicate_field_arg_name_overrides_fail",
            err.to_string()
        );
    }

    #[test]
    fn field_arg_name_on_skipped_variant_field_fails() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub enum TestEnum {
                Something(#[fluent(skip, arg_name = "hidden")] String),
            }
        };

        let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
        let err = es_fluent_derive_core::validation::validate_enum(&opts)
            .expect_err("arg_name on skipped field should fail");

        assert!(
            err.to_string()
                .contains("cannot be used on a skipped field")
        );
    }

    #[test]
    fn empty_variant_field_arg_name_fails() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub enum TestEnum {
                Something(#[fluent(arg_name = "")] String),
            }
        };

        let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
        let err = es_fluent_derive_core::validation::validate_enum(&opts)
            .expect_err("empty arg_name should fail");

        assert!(err.to_string().contains("cannot be empty"));
    }

    #[test]
    fn skipped_variant_field_is_ignored_when_checking_resolved_names() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub enum TestEnum {
                Something(
                    #[fluent(arg_name = "value")] String,
                    #[fluent(skip)] String,
                ),
            }
        };

        let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
        es_fluent_derive_core::validation::validate_enum(&opts)
            .expect("skipped field should not participate in resolved arg names");
    }
}

#[serial_test::serial(manifest)]
mod validate_namespace_tests {
    use super::*;

    #[test]
    fn file_namespace_always_passes() {
        // File-based namespaces are deferred to CLI validation
        let ns = NamespaceRule::File;
        es_fluent_derive_core::validation::validate_namespace(&ns, None)
            .expect("File namespace should always pass at compile time");
    }

    #[test]
    fn file_relative_namespace_always_passes() {
        // File-based namespaces are deferred to CLI validation
        let ns = NamespaceRule::FileRelative;
        es_fluent_derive_core::validation::validate_namespace(&ns, None)
            .expect("FileRelative namespace should always pass at compile time");
    }

    #[test]
    fn folder_namespace_always_passes() {
        // Folder-based namespaces are deferred to CLI validation
        let ns = NamespaceRule::Folder;
        es_fluent_derive_core::validation::validate_namespace(&ns, None)
            .expect("Folder namespace should always pass at compile time");
    }

    #[test]
    fn folder_relative_namespace_always_passes() {
        // Folder-based namespaces are deferred to CLI validation
        let ns = NamespaceRule::FolderRelative;
        es_fluent_derive_core::validation::validate_namespace(&ns, None)
            .expect("FolderRelative namespace should always pass at compile time");
    }

    #[test]
    fn literal_namespace_passes_without_config() {
        // When no i18n.toml exists (or CARGO_MANIFEST_DIR is not set),
        // validation should pass for any literal namespace.
        // This test runs without setting up a config file, so it relies on
        // the graceful fallback behavior.
        let ns = NamespaceRule::Literal("any_namespace".into());
        with_manifest_dir(None, || {
            es_fluent_derive_core::validation::validate_namespace(&ns, None)
                .expect("Should pass when no config exists");
        });
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn literal_namespace_reports_parse_errors_from_config() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(temp.path().join("i18n.toml"), "not = [valid").expect("write i18n.toml");
        let ns = NamespaceRule::Literal("ui".into());

        let err = with_manifest_dir(Some(temp.path()), || {
            es_fluent_derive_core::validation::validate_namespace(&ns, None)
                .expect_err("invalid config should be surfaced")
        });

        assert_snapshot!(
            "validate_namespace_reports_parse_errors_from_config",
            normalize_temp_paths(&err.to_string(), temp.path())
        );
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

        es_fluent_derive_core::validation::validate_namespace_against_allowed("ui", &allowed, None)
            .expect("'ui' should be allowed");
        es_fluent_derive_core::validation::validate_namespace_against_allowed(
            "errors", &allowed, None,
        )
        .expect("'errors' should be allowed");
        es_fluent_derive_core::validation::validate_namespace_against_allowed(
            "components",
            &allowed,
            None,
        )
        .expect("'components' should be allowed");
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn invalid_namespace_not_in_allowed_list() {
        let allowed = vec!["ui".to_string(), "errors".to_string()];

        let err = es_fluent_derive_core::validation::validate_namespace_against_allowed(
            "unknown", &allowed, None,
        )
        .expect_err("'unknown' should not be allowed");

        assert_snapshot!(
            "validate_namespace_against_allowed_invalid_namespace_not_in_allowed_list",
            err.to_string()
        );
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn empty_allowed_list_rejects_everything() {
        let allowed: Vec<String> = vec![];

        let err = es_fluent_derive_core::validation::validate_namespace_against_allowed(
            "anything", &allowed, None,
        )
        .expect_err("Empty allowed list should reject all namespaces");

        assert_snapshot!(
            "validate_namespace_against_allowed_empty_allowed_list_rejects_everything",
            err.to_string()
        );
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn namespace_matching_is_case_sensitive() {
        let allowed = vec!["UI".to_string(), "Errors".to_string()];

        // Exact case should pass
        es_fluent_derive_core::validation::validate_namespace_against_allowed("UI", &allowed, None)
            .expect("'UI' should match exactly");

        // Different case should fail
        let err = es_fluent_derive_core::validation::validate_namespace_against_allowed(
            "ui", &allowed, None,
        )
        .expect_err("'ui' should not match 'UI'");
        assert_snapshot!(
            "validate_namespace_against_allowed_namespace_matching_is_case_sensitive",
            err.to_string()
        );
    }

    #[test]
    fn namespace_with_special_characters() {
        let allowed = vec![
            "my-namespace".to_string(),
            "my_namespace".to_string(),
            "my.namespace".to_string(),
        ];

        es_fluent_derive_core::validation::validate_namespace_against_allowed(
            "my-namespace",
            &allowed,
            None,
        )
        .expect("hyphenated namespace should be allowed");
        es_fluent_derive_core::validation::validate_namespace_against_allowed(
            "my_namespace",
            &allowed,
            None,
        )
        .expect("underscored namespace should be allowed");
        es_fluent_derive_core::validation::validate_namespace_against_allowed(
            "my.namespace",
            &allowed,
            None,
        )
        .expect("dotted namespace should be allowed");
    }
}

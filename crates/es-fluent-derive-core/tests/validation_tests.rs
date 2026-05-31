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

mod attribute_context_tests {
    use super::*;

    #[test]
    fn message_struct_container_rejects_enum_only_fluent_keys() {
        for input in [
            parse_quote! {
                #[derive(EsFluent)]
                #[fluent(domain = "shared")]
                pub struct LoginForm {
                    username: String,
                }
            },
            parse_quote! {
                #[derive(EsFluent)]
                #[fluent(resource = "shared")]
                pub struct LoginForm {
                    username: String,
                }
            },
            parse_quote! {
                #[derive(EsFluent)]
                #[fluent(skip_inventory)]
                pub struct LoginForm {
                    username: String,
                }
            },
        ] {
            let err =
                es_fluent_derive_core::validation::validate_es_fluent_attribute_context(&input)
                    .expect_err("struct-only context should reject enum-only keys");
            let message = err.to_string();
            assert!(message.contains("message struct container"));
            assert!(message.contains("accepted key here is namespace"));
        }
    }

    #[test]
    fn message_struct_container_rejects_removed_default_field_key() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct LoginForm {
                #[fluent(default)]
                username: String,
            }
        };

        let err = es_fluent_derive_core::validation::validate_es_fluent_attribute_context(&input)
            .expect_err("removed default key should fail in raw validation");
        let message = err.to_string();
        assert!(message.contains("#[fluent(default)]"));
        assert!(message.contains("message field"));
        assert!(message.contains("accepted keys here are skip, choice, optional, arg, and value"));
    }

    #[test]
    fn message_container_unknown_keys_use_shape_specific_help() {
        let struct_input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            #[fluent(unknown)]
            pub struct LoginForm {
                username: String,
            }
        };
        let err =
            es_fluent_derive_core::validation::validate_es_fluent_attribute_context(&struct_input)
                .expect_err("unknown struct key should fail");
        assert!(err.to_string().contains("message struct container"));
        assert!(err.to_string().contains("accepted key here is namespace"));

        let enum_input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            #[fluent(unknown)]
            pub enum LoginError {
                InvalidPassword,
            }
        };
        let err =
            es_fluent_derive_core::validation::validate_es_fluent_attribute_context(&enum_input)
                .expect_err("unknown enum key should fail");
        assert!(err.to_string().contains("message enum container"));
        assert!(
            err.to_string()
                .contains("accepted keys here are resource, domain, namespace, and skip_inventory")
        );
    }

    #[test]
    fn enum_fluent_keys_remain_allowed_on_enum_containers() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            #[fluent(resource = "login_error", domain = "auth", namespace = "errors", skip_inventory)]
            pub enum LoginError {
                InvalidPassword,
            }
        };

        es_fluent_derive_core::validation::validate_es_fluent_attribute_context(&input)
            .expect("enum-only keys should pass on enum containers");
    }

    #[test]
    fn variants_variant_context_reports_variants_variant() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluentVariants)]
            pub enum LoginError {
                #[fluent_variants(keys = ["label"])]
                InvalidPassword,
            }
        };

        let err = es_fluent_derive_core::validation::validate_es_fluent_variants_attribute_context(
            &input,
        )
        .expect_err("container-only fluent_variants key should fail on variant");
        assert!(err.to_string().contains("variants variant"));
        assert!(err.to_string().contains("accepted key here is skip"));
    }
}

mod validate_struct_tests {
    use super::*;

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
    fn optional_conflicts_with_choice_value_and_skip_on_struct_fields() {
        for input in [
            parse_quote! {
                #[derive(EsFluent)]
                pub struct TestStruct {
                    #[fluent(optional, choice)]
                    field: Option<String>,
                }
            },
            parse_quote! {
                #[derive(EsFluent)]
                pub struct TestStruct {
                    #[fluent(optional, value = |value: &Option<String>| value.is_some())]
                    field: Option<String>,
                }
            },
            parse_quote! {
                #[derive(EsFluent)]
                pub struct TestStruct {
                    #[fluent(optional, skip)]
                    field: Option<String>,
                }
            },
        ] {
            let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
            let err = es_fluent_derive_core::validation::validate_struct(&opts)
                .expect_err("optional conflict should fail");

            assert!(err.to_string().contains("#[fluent(optional)]"));
        }
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
    fn arg_on_named_struct_field_succeeds() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct TestStruct {
                #[fluent(arg = "display_name")]
                name: String,
                value: String,
            }
        };

        let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
        es_fluent_derive_core::validation::validate_struct(&opts)
            .expect("Validation should succeed");
    }

    #[test]
    fn empty_struct_arg_fails() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct TestStruct {
                #[fluent(arg = "")]
                value: String,
            }
        };

        let err = StructOpts::from_derive_input(&input).expect_err("empty arg should fail");

        assert!(err.to_string().contains("must not be empty"));
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn duplicate_struct_arg_fails() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct TestStruct {
                #[fluent(arg = "value")]
                first: String,
                value: String,
            }
        };

        let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
        let err = es_fluent_derive_core::validation::validate_struct(&opts)
            .expect_err("Expected validation error");
        assert_snapshot!("validate_struct_duplicate_arg_fails", err.to_string());
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn arg_on_skipped_field_fails() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct TestStruct {
                #[fluent(skip, arg = "hidden")]
                hidden: String,
            }
        };

        let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
        let err = es_fluent_derive_core::validation::validate_struct(&opts)
            .expect_err("Expected validation error");
        assert_snapshot!(
            "validate_struct_arg_on_skipped_field_fails",
            err.to_string()
        );
    }

    #[test]
    fn choice_and_value_on_same_struct_field_fails() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct TestStruct {
                #[fluent(choice, value = |name: &String| name.len())]
                name: String,
            }
        };

        let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
        let err = es_fluent_derive_core::validation::validate_struct(&opts)
            .expect_err("choice and value should conflict");

        assert!(
            err.to_string()
                .contains("Cannot combine #[fluent(choice)] and #[fluent(value = ...)]")
        );
    }
}

mod validate_enum_tests {
    use super::*;

    #[test]
    fn field_arg_on_single_tuple_variant_succeeds() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub enum TestEnum {
                Something(#[fluent(arg = "value")] String),
            }
        };

        let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
        es_fluent_derive_core::validation::validate_enum(&opts)
            .expect("Single tuple field with field-level arg should pass");
    }

    #[test]
    fn field_arg_on_named_variant_succeeds() {
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
        es_fluent_derive_core::validation::validate_enum(&opts)
            .expect("Named field with field-level arg should pass");
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn variant_level_arg_is_rejected() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub enum TestEnum {
                #[fluent(arg = "value")]
                Something(String),
            }
        };

        let err = es_fluent_derive_core::validation::validate_es_fluent_attribute_context(&input)
            .expect_err("Expected validation error");
        assert_snapshot!(
            "validate_enum_variant_level_arg_is_rejected",
            err.to_string()
        );
    }

    #[test]
    fn variant_level_skip_and_key_are_allowed() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub enum TestEnum {
                #[fluent(skip)]
                Hidden,
                #[fluent(key = "visible")]
                Visible(String),
            }
        };

        es_fluent_derive_core::validation::validate_es_fluent_attribute_context(&input)
            .expect("variant-level skip and key should pass raw context validation");
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn field_arg_duplicate_with_named_field_fails() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub enum TestEnum {
                Named {
                    #[fluent(arg = "value")]
                    left: String,
                    value: String,
                },
            }
        };

        let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
        let err = es_fluent_derive_core::validation::validate_enum(&opts)
            .expect_err("Expected validation error");
        assert_snapshot!(
            "validate_enum_field_arg_duplicate_with_named_field_fails",
            err.to_string()
        );
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn duplicate_field_arg_overrides_fail() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub enum TestEnum {
                Something(
                    #[fluent(arg = "same")] String,
                    #[fluent(arg = "same")] String,
                ),
            }
        };

        let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
        let err = es_fluent_derive_core::validation::validate_enum(&opts)
            .expect_err("Expected validation error");
        assert_snapshot!(
            "validate_enum_duplicate_field_arg_overrides_fail",
            err.to_string()
        );
    }

    #[test]
    fn field_arg_on_skipped_variant_field_fails() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub enum TestEnum {
                Something(#[fluent(skip, arg = "hidden")] String),
            }
        };

        let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
        let err = es_fluent_derive_core::validation::validate_enum(&opts)
            .expect_err("arg on skipped field should fail");

        assert!(
            err.to_string()
                .contains("cannot be used on a skipped field")
        );
    }

    #[test]
    fn optional_conflicts_with_choice_value_and_skip_on_enum_fields() {
        for input in [
            parse_quote! {
                #[derive(EsFluent)]
                pub enum TestEnum {
                    Something(#[fluent(optional, choice)] Option<String>),
                }
            },
            parse_quote! {
                #[derive(EsFluent)]
                pub enum TestEnum {
                    Something(#[fluent(optional, value = |value: &Option<String>| value.is_some())] Option<String>),
                }
            },
            parse_quote! {
                #[derive(EsFluent)]
                pub enum TestEnum {
                    Something(#[fluent(optional, skip)] Option<String>),
                }
            },
        ] {
            let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
            let err = es_fluent_derive_core::validation::validate_enum(&opts)
                .expect_err("optional conflict should fail");

            assert!(err.to_string().contains("#[fluent(optional)]"));
        }
    }

    #[test]
    fn empty_variant_field_arg_fails() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub enum TestEnum {
                Something(#[fluent(arg = "")] String),
            }
        };

        let err = EnumOpts::from_derive_input(&input).expect_err("empty arg should fail");

        assert!(err.to_string().contains("must not be empty"));
    }

    #[test]
    fn skipped_variant_field_is_ignored_when_checking_resolved_names() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub enum TestEnum {
                Something(
                    #[fluent(arg = "value")] String,
                    #[fluent(skip)] String,
                ),
            }
        };

        let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
        es_fluent_derive_core::validation::validate_enum(&opts)
            .expect("skipped field should not participate in resolved arg names");
    }

    #[test]
    fn choice_and_value_on_same_variant_field_fails() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub enum TestEnum {
                Something(#[fluent(choice, value = |name: &String| name.len())] String),
            }
        };

        let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
        let err = es_fluent_derive_core::validation::validate_enum(&opts)
            .expect_err("choice and value should conflict");

        assert!(
            err.to_string()
                .contains("Cannot combine #[fluent(choice)] and #[fluent(value = ...)]")
        );
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

    fn allowed(namespaces: &[&str]) -> Vec<es_fluent_shared::namespace::ResolvedNamespace> {
        namespaces
            .iter()
            .copied()
            .map(es_fluent_shared::namespace::ResolvedNamespace::new)
            .collect::<Result<_, _>>()
            .expect("test namespaces")
    }

    #[test]
    fn valid_namespace_in_allowed_list() {
        let allowed = allowed(&["ui", "errors", "components"]);

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
        let allowed = allowed(&["ui", "errors"]);

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
        let allowed = vec![];

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
        let allowed = allowed(&["UI", "Errors"]);

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
        let allowed = allowed(&["my-namespace", "my_namespace", "my.namespace"]);

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

mod r#enum;
mod r#struct;

use es_fluent_derive_core::expansion::{EsFluentExpansion, ExpansionError};
use syn::{DeriveInput, parse_macro_input};

use crate::macros::utils::CodegenContext;

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let context = CodegenContext::resolve();
    expand_es_fluent_with_context(input, &context).into()
}

#[cfg(test)]
fn expand_es_fluent(input: DeriveInput) -> proc_macro2::TokenStream {
    let context = CodegenContext::fallback();
    expand_es_fluent_with_context(input, &context)
}

fn expand_es_fluent_with_context(
    input: DeriveInput,
    context: &CodegenContext,
) -> proc_macro2::TokenStream {
    match EsFluentExpansion::from_derive_input(&input) {
        Ok(EsFluentExpansion::Struct(expansion)) => r#struct::process_struct(context, &expansion),
        Ok(EsFluentExpansion::Enum(expansion)) => r#enum::process_enum(context, &expansion),
        Err(error) => expansion_error_to_tokens(error),
    }
}

fn expansion_error_to_tokens(error: ExpansionError) -> proc_macro2::TokenStream {
    match error {
        ExpansionError::Core(error) => crate::macros::utils::core_error_to_compile_error(error),
        ExpansionError::Darling(error) => error.write_errors(),
        ExpansionError::Syn(error) => error.to_compile_error(),
    }
}

#[cfg(test)]
#[serial_test::serial(manifest)]
mod tests {
    use fs_err as fs;
    use insta::assert_snapshot;
    use std::path::Path;
    use syn::parse_quote;
    use tempfile::TempDir;
    use toml::Value;

    fn string_value(value: &str) -> Value {
        Value::String(value.to_string())
    }

    fn table(
        entries: impl IntoIterator<Item = (&'static str, Value)>,
    ) -> toml::map::Map<String, Value> {
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect()
    }

    fn write_toml(path: &Path, value: &Value) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent directory");
        }
        fs::write(
            path,
            toml::to_string(value).expect("serialize TOML fixture"),
        )
        .expect("write TOML fixture");
    }

    fn i18n_config(namespaces: &[&str]) -> Value {
        Value::Table(table([
            ("fallback_language", string_value("en-US")),
            ("assets_dir", string_value("i18n")),
            (
                "namespaces",
                Value::Array(namespaces.iter().copied().map(string_value).collect()),
            ),
        ]))
    }

    fn with_manifest_dir<T>(namespaces: &[&str], f: impl FnOnce() -> T) -> T {
        let temp_dir = TempDir::new().expect("create temp manifest dir");
        let manifest_dir = temp_dir.path();

        write_toml(&manifest_dir.join("i18n.toml"), &i18n_config(namespaces));

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            temp_env::with_var("CARGO_MANIFEST_DIR", Some(&manifest_dir), f)
        }));

        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn expand_es_fluent_generates_tokens_for_enum_and_struct() {
        let enum_input: syn::DeriveInput = parse_quote! {
            enum Status {
                Ready,
            }
        };
        let enum_tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent(enum_input));
        assert_snapshot!("expand_es_fluent_generates_tokens_for_enum", enum_tokens);

        let struct_input: syn::DeriveInput = parse_quote! {
            struct User {
                id: u64
            }
        };
        let struct_tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent(struct_input));
        assert_snapshot!(
            "expand_es_fluent_generates_tokens_for_struct",
            struct_tokens
        );
    }

    #[test]
    fn expand_es_fluent_normalizes_raw_identifiers_in_inventory_metadata() {
        let struct_input: syn::DeriveInput = parse_quote! {
            struct r#type {
                r#match: String,
            }
        };

        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent(struct_input));

        assert!(tokens.contains("mod __es_fluent_inventory_type"));
        assert!(tokens.contains("StaticFluentDomain"));
        assert!(tokens.contains("CARGO_PKG_NAME"));
        assert!(tokens.contains("StaticFluentEntryId"));
        assert!(tokens.contains("\"type\""));
        assert!(tokens.contains("Some(&args)"));
        assert!(tokens.contains("registry::__macro::ftl_type_info"));
        assert!(tokens.contains("registry::__macro::ftl_variant"));
        assert!(tokens.contains("StaticFluentArgumentName"));
        assert!(tokens.contains("\"match\""));
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn expand_es_fluent_returns_compile_errors_for_attribute_parse_failures() {
        let enum_input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = 123)]
            enum BadEnum {
                A
            }
        };
        let enum_tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent(enum_input));
        assert_snapshot!(
            "expand_es_fluent_returns_compile_errors_for_bad_enum_attribute",
            enum_tokens
        );

        let struct_input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = 123)]
            struct BadStruct {
                a: i32
            }
        };
        let struct_tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent(struct_input));
        assert_snapshot!(
            "expand_es_fluent_returns_compile_errors_for_bad_struct_attribute",
            struct_tokens
        );
    }

    #[test]
    fn expand_es_fluent_returns_compile_errors_for_struct_validation_and_union_inputs() {
        let invalid_struct_input: syn::DeriveInput = parse_quote! {
            struct Invalid {
                #[fluent(skip, arg = "value")]
                value: i32,
            }
        };
        let validation_tokens = crate::snapshot_support::pretty_file_tokens(
            super::expand_es_fluent(invalid_struct_input),
        );
        assert_snapshot!(
            "expand_es_fluent_returns_compile_error_for_struct_validation_failure",
            validation_tokens
        );

        let union_input: syn::DeriveInput = parse_quote! {
            union NotSupported {
                a: u32,
                b: f32,
            }
        };
        let union_tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent(union_input));
        assert_snapshot!(
            "expand_es_fluent_returns_compile_error_for_union_input",
            union_tokens
        );
    }

    #[test]
    fn expand_es_fluent_returns_compile_errors_for_namespaces_not_allowed_by_config() {
        with_manifest_dir(&["allowed"], || {
            let enum_input: syn::DeriveInput = parse_quote! {
                #[fluent(namespace = "blocked")]
                enum NamespaceEnum {
                    A
                }
            };
            let enum_tokens =
                crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent(enum_input));
            assert_snapshot!(
                "expand_es_fluent_returns_compile_error_for_blocked_enum_namespace",
                enum_tokens
            );

            let struct_input: syn::DeriveInput = parse_quote! {
                #[fluent(namespace = "blocked")]
                struct NamespaceStruct {
                    value: i32
                }
            };
            let struct_tokens =
                crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent(struct_input));
            assert_snapshot!(
                "expand_es_fluent_returns_compile_error_for_blocked_struct_namespace",
                struct_tokens
            );
        });
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn expand_es_fluent_emits_field_level_tuple_arg() {
        let enum_input: syn::DeriveInput = parse_quote! {
            enum LoginError {
                Something(#[fluent(arg = "value")] String),
            }
        };

        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent(enum_input));
        assert_snapshot!("expand_es_fluent_emits_field_level_tuple_arg", tokens);
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn expand_es_fluent_keeps_later_tuple_default_names_after_field_arg_override() {
        let enum_input: syn::DeriveInput = parse_quote! {
            enum LoginError {
                Something(String, #[fluent(arg = "f1")] String, String),
            }
        };

        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent(enum_input));
        assert_snapshot!(
            "expand_es_fluent_keeps_later_tuple_default_names_after_field_arg_override",
            tokens
        );
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn expand_es_fluent_uses_explicit_domain_override_for_enum_lookup() {
        let enum_input: syn::DeriveInput = parse_quote! {
            #[fluent(id = "es-fluent-lang", domain = "es-fluent-lang")]
            enum Languages {
                #[fluent(key = "en")]
                En,
            }
        };

        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent(enum_input));
        assert_snapshot!(
            "expand_es_fluent_uses_explicit_domain_override_for_enum_lookup",
            tokens
        );
    }

    #[test]
    fn expand_es_fluent_returns_compile_error_for_invalid_enum_resource_override() {
        let enum_input: syn::DeriveInput = parse_quote! {
            #[fluent(id = "bad key")]
            enum BadResource {
                Ready,
            }
        };

        let tokens = super::expand_es_fluent(enum_input).to_string();

        assert!(tokens.contains("compile_error"));
        assert!(tokens.contains("Fluent message id"));
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn expand_es_fluent_handles_tuple_variant_with_all_fields_skipped() {
        let enum_input: syn::DeriveInput = parse_quote! {
            enum LoginError {
                Something(#[fluent(skip)] String, #[fluent(skip)] u32),
            }
        };

        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent(enum_input));
        assert_snapshot!(
            "expand_es_fluent_handles_tuple_variant_with_all_fields_skipped",
            tokens
        );
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn expand_es_fluent_delegates_skipped_single_field_variant() {
        let enum_input: syn::DeriveInput = parse_quote! {
            enum LoginError {
                #[fluent(skip)]
                Network(NetworkError),
            }
        };

        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent(enum_input));
        assert_snapshot!(
            "expand_es_fluent_delegates_skipped_single_field_variant",
            tokens
        );
    }
}

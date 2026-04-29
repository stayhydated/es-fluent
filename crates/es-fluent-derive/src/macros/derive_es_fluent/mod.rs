mod r#enum;
mod r#struct;

use darling::FromDeriveInput as _;
use es_fluent_derive_core::{
    options::{r#enum::EnumOpts, r#struct::StructOpts},
    validation,
};
use es_fluent_shared::namespace::NamespaceRule;
use syn::{Data, DeriveInput, parse_macro_input};

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_es_fluent(input).into()
}

fn validate_namespace(namespace: Option<&NamespaceRule>, span: proc_macro2::Span) {
    if let Some(ns) = namespace
        && let Err(err) = validation::validate_namespace(ns, Some(span))
    {
        err.abort();
    }
}

fn expand_es_fluent(input: DeriveInput) -> proc_macro2::TokenStream {
    match &input.data {
        Data::Enum(data) => {
            let opts = match EnumOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors(),
            };

            if let Err(err) = validation::validate_enum(&opts) {
                err.abort();
            }

            validate_namespace(opts.attr_args().namespace(), opts.ident().span());

            r#enum::process_enum(&opts, data)
        },
        Data::Struct(data) => {
            let opts = match StructOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors(),
            };

            if let Err(err) = validation::validate_struct(&opts) {
                err.abort();
            }

            validate_namespace(opts.attr_args().namespace(), opts.ident().span());

            r#struct::process_struct(&opts, data)
        },
        _ => proc_macro_error2::abort!(
            input.ident.span(),
            "EsFluent can only be derived for structs and enums"
        ),
    }
}

#[cfg(test)]
#[serial_test::serial(manifest)]
mod tests {
    use super::expand_es_fluent;
    use crate::snapshot_support::pretty_file_tokens;
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
        let enum_tokens = pretty_file_tokens(expand_es_fluent(enum_input));
        assert_snapshot!("expand_es_fluent_generates_tokens_for_enum", enum_tokens);

        let struct_input: syn::DeriveInput = parse_quote! {
            struct User {
                id: u64
            }
        };
        let struct_tokens = pretty_file_tokens(expand_es_fluent(struct_input));
        assert_snapshot!(
            "expand_es_fluent_generates_tokens_for_struct",
            struct_tokens
        );
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
        let enum_tokens = pretty_file_tokens(expand_es_fluent(enum_input));
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
        let struct_tokens = pretty_file_tokens(expand_es_fluent(struct_input));
        assert_snapshot!(
            "expand_es_fluent_returns_compile_errors_for_bad_struct_attribute",
            struct_tokens
        );
    }

    #[test]
    fn expand_es_fluent_panics_for_struct_validation_and_union_inputs() {
        let invalid_struct_input: syn::DeriveInput = parse_quote! {
            struct Invalid {
                #[fluent(default)]
                a: i32,
                #[fluent(default)]
                b: i32,
            }
        };
        let validation_panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = expand_es_fluent(invalid_struct_input);
        }));
        assert!(validation_panic.is_err());

        let union_input: syn::DeriveInput = parse_quote! {
            union NotSupported {
                a: u32,
                b: f32,
            }
        };
        let union_panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = expand_es_fluent(union_input);
        }));
        assert!(union_panic.is_err());
    }

    #[test]
    fn expand_es_fluent_panics_for_namespaces_not_allowed_by_config() {
        with_manifest_dir(&["allowed"], || {
            let enum_input: syn::DeriveInput = parse_quote! {
                #[fluent(namespace = "blocked")]
                enum NamespaceEnum {
                    A
                }
            };
            let enum_panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = expand_es_fluent(enum_input);
            }));
            assert!(enum_panic.is_err());

            let struct_input: syn::DeriveInput = parse_quote! {
                #[fluent(namespace = "blocked")]
                struct NamespaceStruct {
                    value: i32
                }
            };
            let struct_panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = expand_es_fluent(struct_input);
            }));
            assert!(struct_panic.is_err());
        });
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn expand_es_fluent_emits_field_level_tuple_arg_name() {
        let enum_input: syn::DeriveInput = parse_quote! {
            enum LoginError {
                Something(#[fluent(arg_name = "value")] String),
            }
        };

        let tokens = pretty_file_tokens(expand_es_fluent(enum_input));
        assert_snapshot!("expand_es_fluent_emits_field_level_tuple_arg_name", tokens);
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn expand_es_fluent_keeps_later_tuple_default_names_after_field_arg_name_override() {
        let enum_input: syn::DeriveInput = parse_quote! {
            enum LoginError {
                Something(String, #[fluent(arg_name = "f1")] String, String),
            }
        };

        let tokens = pretty_file_tokens(expand_es_fluent(enum_input));
        assert_snapshot!(
            "expand_es_fluent_keeps_later_tuple_default_names_after_field_arg_name_override",
            tokens
        );
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn expand_es_fluent_uses_explicit_domain_override_for_enum_lookup() {
        let enum_input: syn::DeriveInput = parse_quote! {
            #[fluent(resource = "es-fluent-lang", domain = "es-fluent-lang")]
            enum Languages {
                #[fluent(key = "en")]
                En,
            }
        };

        let tokens = pretty_file_tokens(expand_es_fluent(enum_input));
        assert_snapshot!(
            "expand_es_fluent_uses_explicit_domain_override_for_enum_lookup",
            tokens
        );
    }

    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
    fn expand_es_fluent_handles_tuple_variant_with_all_fields_skipped() {
        let enum_input: syn::DeriveInput = parse_quote! {
            enum LoginError {
                Something(#[fluent(skip)] String, #[fluent(skip)] u32),
            }
        };

        let tokens = pretty_file_tokens(expand_es_fluent(enum_input));
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

        let tokens = pretty_file_tokens(expand_es_fluent(enum_input));
        assert_snapshot!(
            "expand_es_fluent_delegates_skipped_single_field_variant",
            tokens
        );
    }
}

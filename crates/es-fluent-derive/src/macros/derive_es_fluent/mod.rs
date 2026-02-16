mod r#enum;
mod r#struct;

use darling::FromDeriveInput as _;
use es_fluent_derive_core::{
    options::{r#enum::EnumOpts, r#struct::StructOpts},
    validation,
};
use syn::{Data, DeriveInput, parse_macro_input};

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_es_fluent(input).into()
}

fn expand_es_fluent(input: DeriveInput) -> proc_macro2::TokenStream {
    let tokens = match &input.data {
        Data::Enum(data) => {
            let opts = match EnumOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors(),
            };

            // Validate namespace if provided
            if let Some(ns) = opts.attr_args().namespace()
                && let Err(err) = validation::validate_namespace(ns, Some(opts.ident().span()))
            {
                err.abort();
            }

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

            // Validate namespace if provided
            if let Some(ns) = opts.attr_args().namespace()
                && let Err(err) = validation::validate_namespace(ns, Some(opts.ident().span()))
            {
                err.abort();
            }

            r#struct::process_struct(&opts, data)
        },
        _ => panic!("Unsupported data type"),
    };

    tokens
}

#[cfg(test)]
mod tests {
    use super::expand_es_fluent;
    use std::sync::{LazyLock, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};
    use syn::parse_quote;

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn with_manifest_dir<T>(namespaces: &[&str], f: impl FnOnce() -> T) -> T {
        let _guard = ENV_LOCK.lock().expect("lock poisoned");
        let previous = std::env::var_os("CARGO_MANIFEST_DIR");

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let manifest_dir = std::env::temp_dir().join(format!(
            "es-fluent-derive-expand-{pid}-{unique}",
            pid = std::process::id()
        ));

        std::fs::create_dir_all(&manifest_dir).expect("create temp manifest dir");
        let namespaces_value = namespaces
            .iter()
            .map(|ns| format!("\"{ns}\""))
            .collect::<Vec<_>>()
            .join(", ");
        std::fs::write(
            manifest_dir.join("i18n.toml"),
            format!(
                "fallback_language = \"en-US\"\nassets_dir = \"i18n\"\nnamespaces = [{namespaces_value}]\n"
            ),
        )
        .expect("write i18n.toml");

        // SAFETY: tests serialize environment updates with a global lock.
        unsafe { std::env::set_var("CARGO_MANIFEST_DIR", &manifest_dir) };

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        match previous {
            Some(prev) => {
                // SAFETY: tests serialize environment updates with a global lock.
                unsafe { std::env::set_var("CARGO_MANIFEST_DIR", prev) };
            },
            None => {
                // SAFETY: tests serialize environment updates with a global lock.
                unsafe { std::env::remove_var("CARGO_MANIFEST_DIR") };
            },
        }
        let _ = std::fs::remove_dir_all(&manifest_dir);

        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    #[test]
    fn expand_es_fluent_generates_tokens_for_enum_and_struct() {
        let enum_input: syn::DeriveInput = parse_quote! {
            enum Status {
                Ready,
            }
        };
        let enum_tokens = expand_es_fluent(enum_input).to_string();
        assert!(enum_tokens.contains("impl"));

        let struct_input: syn::DeriveInput = parse_quote! {
            struct User {
                id: u64
            }
        };
        let struct_tokens = expand_es_fluent(struct_input).to_string();
        assert!(struct_tokens.contains("impl"));
    }

    #[test]
    fn expand_es_fluent_returns_compile_errors_for_attribute_parse_failures() {
        let enum_input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = 123)]
            enum BadEnum {
                A
            }
        };
        let enum_tokens = expand_es_fluent(enum_input).to_string();
        assert!(enum_tokens.contains("compile_error"));

        let struct_input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = 123)]
            struct BadStruct {
                a: i32
            }
        };
        let struct_tokens = expand_es_fluent(struct_input).to_string();
        assert!(struct_tokens.contains("compile_error"));
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
}

use super::test_context::{
    SourceDeclaration, attributes_enable_test_only_derive, attributes_require_test,
    collect_source_evidence, literal_include_path,
};
use super::*;
use std::collections::HashSet;
use std::ffi::OsStr;

#[test]
fn literal_include_path_rejects_dynamic_and_missing_inputs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let current_file = temp.path().join("lib.rs");
    let included_file = temp.path().join("included.rs");
    std::fs::write(&included_file, "struct Included;").expect("write included source");

    let literal: syn::ItemMacro = syn::parse_quote!(include!("included.rs"););
    let dynamic: syn::ItemMacro = syn::parse_quote!(include!(concat!("included", ".rs")););
    let missing: syn::ItemMacro = syn::parse_quote!(include!("missing.rs"););

    assert_eq!(
        literal_include_path(&literal.mac, &current_file),
        Some(
            included_file
                .canonicalize()
                .expect("canonical included path")
        )
    );
    assert_eq!(literal_include_path(&dynamic.mac, &current_file), None);
    assert_eq!(literal_include_path(&missing.mac, &current_file), None);
}

#[test]
fn literal_includes_preserve_expression_and_nested_module_test_context() {
    let temp = tempfile::tempdir().expect("tempdir");
    let nested_dir = temp.path().join("nested");
    std::fs::create_dir_all(&nested_dir).expect("create nested include directory");

    let root = temp.path().join("lib.rs");
    let expression = temp.path().join("expression.rs");
    std::fs::write(
        &root,
        "#[test]\nfn expression_case() { let _value = include!(\"expression.rs\"); }\n#[cfg(test)]\nmod nested_case { include!(\"nested/items.rs\"); }\n",
    )
    .expect("write root source");
    std::fs::write(
        &expression,
        "{ struct ExpressionTarget; ExpressionTarget }\n",
    )
    .expect("write included expression");
    std::fs::write(nested_dir.join("items.rs"), "mod messages;\n").expect("write included items");
    let nested_target = nested_dir.join("messages.rs");
    std::fs::write(&nested_target, "struct NestedTarget;\n").expect("write nested module");

    for (path, marker) in [
        (expression, "ExpressionTarget"),
        (nested_target, "NestedTarget"),
    ] {
        let target = SourceDeclaration {
            path: path.canonicalize().expect("canonical target"),
            marked_source: std::fs::read_to_string(&path).expect("read target source"),
            marker_ident: marker.to_string(),
        };
        let mut visited = HashSet::new();
        let mut evidence = Vec::new();
        collect_source_evidence(
            &root,
            temp.path(),
            false,
            &target,
            Some(FallbackValidationDerive::EsFluent),
            &mut visited,
            &mut evidence,
        );
        assert_eq!(evidence, vec![true], "{marker} should inherit test context");
    }
}

#[test]
fn rustdoc_synthetic_crate_bypasses_strict_fallback_coverage() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    let id = FluentMessageId::try_new("doctest_only").expect("valid message id");

    temp_env::with_vars(
        [
            ("CARGO_MANIFEST_DIR", Some(temp.path().as_os_str())),
            ("CARGO_PKG_NAME", Some(OsStr::new("test-package"))),
            (INVENTORY_RUNNER_ENV, None),
            (FALLBACK_CATALOG_ENV, None),
            ("UNSTABLE_RUSTDOC_TEST_PATH", Some(OsStr::new("doctest.rs"))),
        ],
        || {
            assert_eq!(
                fallback_validation(&syn::parse_quote!(
                    struct DoctestOnly;
                ))
                .diagnostic(None, &id, "DoctestOnly"),
                None
            );
        },
    );
}

#[test]
fn cfg_predicates_only_exempt_items_that_require_test() {
    let test_only: syn::DeriveInput = syn::parse_quote! {
        #[cfg(all(unix, test))]
        struct TestOnly;
    };
    let maybe_test: syn::DeriveInput = syn::parse_quote! {
        #[cfg(any(unix, test))]
        struct MaybeTest;
    };
    let double_negative: syn::DeriveInput = syn::parse_quote! {
        #[cfg(not(not(test)))]
        struct DoubleNegative;
    };

    assert!(attributes_require_test(&test_only.attrs));
    assert!(!attributes_require_test(&maybe_test.attrs));
    assert!(attributes_require_test(&double_negative.attrs));
}

#[test]
fn cfg_attr_exemption_is_specific_to_the_test_only_derive() {
    let input: syn::DeriveInput = syn::parse_quote! {
        #[cfg_attr(test, derive(es_fluent::EsFluent))]
        #[cfg_attr(any(test, feature = "demo"), derive(es_fluent::EsFluentLabel))]
        struct TestOnly;
    };

    assert!(attributes_enable_test_only_derive(
        &input.attrs,
        Some(FallbackValidationDerive::EsFluent)
    ));
    assert!(!attributes_enable_test_only_derive(
        &input.attrs,
        Some(FallbackValidationDerive::EsFluentLabel)
    ));
    assert!(!attributes_enable_test_only_derive(
        &input.attrs,
        Some(FallbackValidationDerive::EsFluentVariants)
    ));
}

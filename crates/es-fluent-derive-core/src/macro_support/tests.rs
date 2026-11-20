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
            (BUILD_MANIFEST_ENV, None),
            (BUILD_MANIFEST_HASH_ENV, None),
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

/// Writes `ftl` as the fallback resource of `package` and returns the
/// collected build manifest path.
fn write_build_manifest(root: &std::path::Path, package: &str, ftl: &str) -> std::path::PathBuf {
    std::fs::create_dir_all(root.join("i18n/en")).expect("create locale dir");
    std::fs::write(
        root.join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(root.join(format!("i18n/en/{package}.ftl")), ftl).expect("write FTL");
    let layout = es_fluent_toml::ResolvedI18nLayout::from_manifest_dir(root).expect("layout");
    let manifest = es_fluent_build::manifest::BuildManifest::collect(&layout, package, true)
        .expect("collect manifest");
    let path = root.join("build-manifest.json");
    std::fs::write(&path, manifest.to_bytes()).expect("write manifest");
    path
}

#[test]
fn package_validation_is_reused_while_the_manifest_hash_is_unchanged() {
    let temp = tempfile::tempdir().expect("tempdir");
    let manifest = write_build_manifest(temp.path(), "cached-package", "hello = Hello\n");
    let id = FluentMessageId::try_new("hello").expect("valid message id");
    let input: syn::DeriveInput = syn::parse_quote!(
        struct Hello;
    );
    let diagnostic_with = |hash: Option<&str>| {
        temp_env::with_vars(
            [
                ("CARGO_MANIFEST_DIR", Some(temp.path().as_os_str())),
                ("CARGO_PKG_NAME", Some(OsStr::new("cached-package"))),
                (INVENTORY_RUNNER_ENV, None),
                ("UNSTABLE_RUSTDOC_TEST_PATH", None),
                (BUILD_MANIFEST_ENV, Some(manifest.as_os_str())),
                (BUILD_MANIFEST_HASH_ENV, hash.map(OsStr::new)),
            ],
            || fallback_validation(&input).diagnostic(None, &id, "Hello"),
        )
    };

    assert_eq!(diagnostic_with(Some("first")), None);
    write_build_manifest(temp.path(), "cached-package", "");
    assert_eq!(
        diagnostic_with(Some("first")),
        None,
        "an unchanged hash reuses the package state"
    );
    assert!(
        diagnostic_with(Some("second")).is_some(),
        "a new hash rereads the manifest"
    );

    write_build_manifest(temp.path(), "cached-package", "hello = Hello\n");
    assert_eq!(
        diagnostic_with(None),
        None,
        "without a hash the manifest is read for every expansion"
    );
}

#[test]
fn package_inputs_come_from_the_manifest_instead_of_package_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let package_dir = temp.path().join("package");
    let manifest = write_build_manifest(&package_dir, "manifest-package", "hello = Hello\n");
    let moved = temp.path().join("manifest.json");
    std::fs::rename(&manifest, &moved).expect("move manifest");
    std::fs::remove_dir_all(&package_dir).expect("remove package files");
    let id = FluentMessageId::try_new("hello").expect("valid message id");

    temp_env::with_vars(
        [
            ("CARGO_MANIFEST_DIR", Some(package_dir.as_os_str())),
            ("CARGO_PKG_NAME", Some(OsStr::new("manifest-package"))),
            (INVENTORY_RUNNER_ENV, None),
            ("UNSTABLE_RUSTDOC_TEST_PATH", None),
            (BUILD_MANIFEST_ENV, Some(moved.as_os_str())),
            (BUILD_MANIFEST_HASH_ENV, None),
        ],
        || {
            let inputs = PackageInputs::current().expect("inputs");
            let config = inputs.config().expect("config");
            assert_eq!(config.fallback_language_id(), "en");
            let resources = inputs
                .resources(&config, "manifest-package")
                .expect("resources");
            assert_eq!(resources.languages.len(), 1);
            assert_eq!(
                inputs.languages(&config).expect("languages"),
                resources.languages
            );
            assert_eq!(
                current_package_config()
                    .expect("config")
                    .map(|config| config.fallback_language_id()),
                Some("en".to_string())
            );
            assert_eq!(
                fallback_validation(&syn::parse_quote!(
                    struct Hello;
                ))
                .diagnostic(None, &id, "Hello"),
                None
            );
        },
    );
}

#[test]
fn mismatched_manifest_format_is_an_actionable_diagnostic() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    let manifest = temp.path().join("manifest.json");
    std::fs::write(&manifest, br#"{"format":0}"#).expect("write manifest");
    let id = FluentMessageId::try_new("hello").expect("valid message id");

    let diagnostic = temp_env::with_vars(
        [
            ("CARGO_MANIFEST_DIR", Some(temp.path().as_os_str())),
            ("CARGO_PKG_NAME", Some(OsStr::new("format-package"))),
            (INVENTORY_RUNNER_ENV, None),
            ("UNSTABLE_RUSTDOC_TEST_PATH", None),
            (BUILD_MANIFEST_ENV, Some(manifest.as_os_str())),
            (BUILD_MANIFEST_HASH_ENV, None),
        ],
        || {
            fallback_validation(&syn::parse_quote!(
                struct Hello;
            ))
            .diagnostic(None, &id, "Hello")
        },
    )
    .expect("diagnostic");

    assert!(
        diagnostic.contains("use matching es-fluent and es-fluent-build versions"),
        "unexpected diagnostic: {diagnostic}"
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

#[test]
fn registry_source_file_is_manifest_relative() {
    use super::source_file::manifest_relative_path;
    use std::path::Path;

    let manifest_dir = Path::new("/workspace/examples/readme");
    let cwd = Path::new("/workspace");

    assert_eq!(
        manifest_relative_path(
            Path::new("examples/readme/src/namespaces/user/profile.rs"),
            cwd,
            manifest_dir,
        )
        .as_deref(),
        Some("src/namespaces/user/profile.rs")
    );
    assert_eq!(
        manifest_relative_path(
            Path::new("/workspace/examples/readme/src/lib.rs"),
            Path::new("/elsewhere"),
            manifest_dir,
        )
        .as_deref(),
        Some("src/lib.rs")
    );
    assert_eq!(
        manifest_relative_path(
            Path::new("/workspace/target/debug/build/out/generated.rs"),
            cwd,
            manifest_dir,
        ),
        None
    );
    assert_eq!(
        manifest_relative_path(manifest_dir, cwd, manifest_dir),
        None
    );
}

#[test]
fn registry_source_file_falls_back_to_file_macro_without_local_file() {
    let tokens = registry_source_file_tokens(proc_macro2::Span::call_site());
    assert_eq!(tokens.to_string(), quote!(file!()).to_string());
}

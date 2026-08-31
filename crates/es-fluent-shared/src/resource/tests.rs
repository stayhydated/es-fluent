use super::*;
use std::collections::HashSet;

use crate::fluent::FluentDomain;
use crate::namespace::NamespacePathError;
use crate::namespace::ResolvedNamespace;

#[test]
fn fallback_catalog_encodes_only_resolvable_messages() {
    let domain = FluentDomain::try_new("demo").expect("domain");
    let mut catalog = FallbackCatalog::default();
    catalog
        .insert_source(
            &domain,
            "hello = Hello\nattr-only =\n    .label = Label\n-term = Term\n".to_string(),
        )
        .expect("catalog");
    let encoded = catalog.encode();

    assert!(fallback_catalog_contains(&encoded, "demo", "hello"));
    assert!(!fallback_catalog_contains(&encoded, "demo", "attr-only"));
    assert!(!fallback_catalog_contains(&encoded, "demo", "term"));
    assert!(!fallback_catalog_contains(&encoded, "other", "hello"));
    assert!(fallback_catalog_contains(b"demo\thello", "demo", "hello"));
}

#[test]
fn fallback_catalog_rejects_message_term_collisions() {
    let domain = FluentDomain::try_new("demo").expect("domain");
    let mut catalog = FallbackCatalog::default();
    let error = catalog
        .insert_source(&domain, "hello = Hello\n-hello = Term\n".to_string())
        .expect_err("collision");

    assert!(error.to_string().contains("duplicate Fluent entry 'hello'"));
}

#[test]
fn resource_key_explicit_constructors_preserve_key_and_domain() {
    let dynamic = ResourceKey::try_new("demo/ui").expect("dynamic key");
    let static_key = ResourceKey::from_static_path("demo/errors");

    assert_eq!(dynamic.as_str(), "demo/ui");
    assert_eq!(dynamic.domain(), "demo");
    assert_eq!(dynamic.domain_name().as_str(), "demo");
    assert_eq!(dynamic.as_ref(), "demo/ui");
    assert_eq!(static_key.to_string(), "demo/errors");
}

#[test]
fn resource_key_rejects_noncanonical_key_shapes() {
    assert_eq!(
        ResourceKey::try_new("../demo").expect_err("parent segment"),
        ResourceKeyError(NamespacePathError::CurrentOrParentSegment)
    );
    assert_eq!(
        ResourceKey::try_new("demo.ftl").expect_err("file suffix"),
        ResourceKeyError(NamespacePathError::FileExtension)
    );
    assert_eq!(
        ResourceKey::try_new("demo//ui").expect_err("empty segment"),
        ResourceKeyError(NamespacePathError::EmptySegment)
    );
}

#[test]
fn locale_relative_ftl_path_validates_canonical_resource_paths() {
    let path = LocaleRelativeFtlPath::try_new("demo/ui.ftl").expect("path");
    assert_eq!(path.as_str(), "demo/ui.ftl");
    assert_eq!(path.to_string(), "demo/ui.ftl");
    assert_eq!(path, "demo/ui.ftl");
    assert_eq!(
        LocaleRelativeFtlPath::from_static_path("demo/static.ftl").as_str(),
        "demo/static.ftl"
    );

    assert_eq!(
        LocaleRelativeFtlPath::try_new("").expect_err("empty path"),
        LocaleRelativeFtlPathError::Empty
    );
    assert_eq!(
        LocaleRelativeFtlPath::try_new("/demo.ftl").expect_err("absolute path"),
        LocaleRelativeFtlPathError::Absolute
    );
    assert_eq!(
        LocaleRelativeFtlPath::try_new("demo\\ui.ftl").expect_err("backslash path"),
        LocaleRelativeFtlPathError::Backslash
    );
    assert_eq!(
        LocaleRelativeFtlPath::try_new("demo/ui").expect_err("missing suffix"),
        LocaleRelativeFtlPathError::MissingFtlSuffix
    );
    assert!(matches!(
        LocaleRelativeFtlPath::try_new("demo/../ui.ftl"),
        Err(LocaleRelativeFtlPathError::InvalidStem(
            NamespacePathError::CurrentOrParentSegment
        ))
    ));
}

#[test]
fn module_resource_spec_try_new_reports_invalid_parts() {
    assert!(matches!(
        ModuleResourceSpec::try_new("../demo", "demo.ftl", true),
        Err(ResourcePlanError::InvalidResourceKey {
            key,
            details: ResourceKeyError(NamespacePathError::CurrentOrParentSegment),
        }) if key == "../demo"
    ));
    assert!(matches!(
        ModuleResourceSpec::try_new("demo", "demo", true),
        Err(ResourcePlanError::InvalidResourcePath {
            path,
            details: LocaleRelativeFtlPathError::MissingFtlSuffix,
        }) if path == "demo"
    ));
}

#[test]
fn resource_plan_for_handles_base_and_namespaced_resources() {
    let base_plan = resource_plan_for("demo", &[]);
    assert_eq!(base_plan.len(), 1);
    assert_eq!(base_plan[0].key.as_str(), "demo");
    assert_eq!(base_plan[0].locale_relative_path, "demo.ftl");
    let en_us = "en-US".parse().expect("language id");
    assert_eq!(base_plan[0].locale_path(&en_us), "en-US/demo.ftl");
    assert!(base_plan[0].required);

    let namespaced_plan = resource_plan_for("demo", &["ui", "ui", "errors"]);
    let keys: Vec<_> = namespaced_plan
        .iter()
        .map(|spec| spec.key.as_str())
        .collect();
    assert_eq!(keys, vec!["demo", "demo/ui", "demo/errors"]);
    assert!(!namespaced_plan[0].required);
    assert_eq!(namespaced_plan[0].locale_relative_path, "demo.ftl");
    assert!(namespaced_plan[1].required);
    assert!(
        namespaced_plan
            .iter()
            .all(|spec| spec.locale_relative_path.ends_with(".ftl"))
    );
}

#[test]
fn resource_plan_api_exposes_specs_and_sparse_plans() {
    let plan = ResourcePlan::for_domain("demo", &["ui"]).expect("resource plan");
    assert_eq!(plan.specs()[0], ModuleResourceSpec::base("demo", false));
    assert_eq!(
        plan.specs()[1].key,
        ResourceKey::from_static_path("demo/ui")
    );

    let namespace = ResolvedNamespace::new("errors/forms").expect("namespace");
    let sparse = ResourcePlan::sparse_for_domain("demo", true, &[namespace], false);
    let specs = sparse.into_specs();

    assert_eq!(
        specs,
        vec![
            ModuleResourceSpec::base("demo", false),
            ModuleResourceSpec::new(
                ResourceKey::from_static_path("demo/errors/forms"),
                LocaleRelativeFtlPath::from_static_path("demo/errors/forms.ftl"),
                true
            ),
        ]
    );
}

#[test]
fn sparse_from_assets_discovers_canonical_language_resource_plans() {
    let temp = tempfile::tempdir().expect("tempdir");
    let assets = temp.path();
    std::fs::create_dir_all(assets.join("en-US/demo")).expect("create en assets");
    std::fs::create_dir_all(assets.join("fr/demo/forms")).expect("create fr assets");
    std::fs::write(assets.join("en-US/demo.ftl"), "hello = Hello").expect("write en base");
    std::fs::write(assets.join("en-US/demo/ui.ftl"), "title = UI").expect("write en ui");
    std::fs::write(assets.join("fr/demo/forms/button.ftl"), "button = Bouton")
        .expect("write fr namespace");
    std::fs::write(assets.join("en-US/other-domain.ftl"), "other = Other")
        .expect("write host-provided domain");
    std::fs::create_dir_all(assets.join("fr/other-domain")).expect("create other domain");
    std::fs::write(assets.join("fr/other-domain/ui.ftl"), "other-ui = Autre")
        .expect("write host-provided namespace");
    std::fs::write(assets.join("fr/demo/ignore.txt"), "ignored").expect("write ignored");

    let plans = ResourcePlan::sparse_from_assets("demo", assets).expect("plans");

    assert_eq!(
        plans
            .languages()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        vec!["en-US", "fr"]
    );
    assert_eq!(
        plans
            .namespaces()
            .iter()
            .map(ResolvedNamespace::as_str)
            .collect::<Vec<_>>(),
        vec!["forms/button", "ui"]
    );
    let specs_by_language = plans
        .resource_specs_by_language()
        .iter()
        .map(|(language, specs)| (language.to_string(), specs.clone()))
        .collect::<Vec<_>>();
    assert_eq!(
        specs_by_language,
        vec![
            (
                "en-US".to_string(),
                vec![
                    ModuleResourceSpec::base("demo", false),
                    ModuleResourceSpec::namespaced(
                        "demo",
                        &ResolvedNamespace::new("ui").expect("ui namespace"),
                        true
                    ),
                ]
            ),
            (
                "fr".to_string(),
                vec![ModuleResourceSpec::namespaced(
                    "demo",
                    &ResolvedNamespace::new("forms/button").expect("forms namespace"),
                    true
                ),]
            ),
        ]
    );
}

#[test]
fn sparse_from_assets_requires_base_when_no_namespaces_exist() {
    let temp = tempfile::tempdir().expect("tempdir");
    let assets = temp.path();
    std::fs::create_dir_all(assets.join("en")).expect("create en assets");
    std::fs::write(assets.join("en/demo.ftl"), "hello = Hello").expect("write en base");

    let plans = ResourcePlan::sparse_from_assets("demo", assets).expect("plans");

    let specs_by_language = plans
        .resource_specs_by_language()
        .iter()
        .map(|(language, specs)| (language.to_string(), specs.clone()))
        .collect::<Vec<_>>();
    assert_eq!(
        specs_by_language,
        vec![(
            "en".to_string(),
            vec![ModuleResourceSpec::base("demo", true)]
        )]
    );
}

#[test]
fn sparse_from_assets_rejects_noncanonical_locale_directories() {
    let temp = tempfile::tempdir().expect("tempdir");
    let assets = temp.path();
    std::fs::create_dir_all(assets.join("en-us")).expect("create invalid locale");
    std::fs::write(assets.join("en-us/demo.ftl"), "hello = Hello").expect("write base");

    let error = ResourcePlan::sparse_from_assets("demo", assets).expect_err("invalid locale");

    assert!(matches!(
        error,
        SparseAssetResourcePlanError::InvalidLocaleDirectory { ref raw_name, .. }
            if raw_name == "en-us"
    ));
    assert!(
        error
            .to_string()
            .contains("must use canonical BCP-47 form 'en-US'")
    );
}

#[test]
fn sparse_from_assets_rejects_invalid_namespaces() {
    let temp = tempfile::tempdir().expect("tempdir");
    let assets = temp.path();
    std::fs::create_dir_all(assets.join("en/demo")).expect("create en assets");
    std::fs::write(assets.join("en/demo/bad.ftl.ftl"), "bad = Bad")
        .expect("write invalid namespace");

    let error = ResourcePlan::sparse_from_assets("demo", assets).expect_err("invalid namespace");

    assert!(matches!(
        error,
        SparseAssetResourcePlanError::InvalidNamespace {
            namespace,
            domain,
            details: NamespacePathError::FileExtension,
        } if namespace == "bad.ftl" && domain == "demo"
    ));
}

#[test]
#[should_panic(expected = "resource_plan_for received invalid namespace")]
fn resource_plan_for_rejects_invalid_namespaces() {
    let _ = resource_plan_for("demo", &["../outside"]);
}

#[test]
fn try_resource_plan_for_reports_invalid_namespaces() {
    let err =
        try_resource_plan_for("demo", &["../outside"]).expect_err("invalid namespace should fail");

    assert_eq!(
        err,
        ResourcePlanError::InvalidNamespace {
            namespace: "../outside".to_string(),
            details: NamespacePathError::CurrentOrParentSegment
        }
    );
}

#[test]
fn try_resource_plan_for_reports_invalid_domain_without_panicking() {
    let err = try_resource_plan_for("../demo", &[]).expect_err("invalid domain should fail");

    assert!(matches!(
        err,
        ResourcePlanError::InvalidDomain { domain, .. } if domain == "../demo"
    ));
}

#[test]
fn resource_plan_uses_resolved_namespace_keys() {
    let plan = resource_plan_for("demo", &["ui/button"]);

    assert_eq!(plan[1].key, ResourceKey::from_static_path("demo/ui/button"));
    assert_eq!(plan[1].locale_relative_path, "demo/ui/button.ftl");
}

#[test]
fn required_and_optional_keys_reflect_plan_membership() {
    let plan = vec![
        ModuleResourceSpec::new(
            ResourceKey::from_static_path("demo"),
            LocaleRelativeFtlPath::from_static_path("demo.ftl"),
            true,
        ),
        ModuleResourceSpec::new(
            ResourceKey::from_static_path("demo/optional"),
            LocaleRelativeFtlPath::from_static_path("demo/optional.ftl"),
            false,
        ),
    ];

    let required = required_resource_keys_from_plan(&plan);
    let optional = optional_resource_keys_from_plan(&plan);

    assert!(required.contains(&ResourceKey::from_static_path("demo")));
    assert!(!required.contains(&ResourceKey::from_static_path("demo/optional")));
    assert!(optional.contains(&ResourceKey::from_static_path("demo/optional")));
    assert!(!optional.contains(&ResourceKey::from_static_path("demo")));

    let loaded = HashSet::from([ResourceKey::from_static_path("demo")]);
    assert!(locale_is_ready(&required, &loaded));

    let unloaded = HashSet::new();
    assert!(!locale_is_ready(&required, &unloaded));
}

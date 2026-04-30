use super::*;
use unic_langid::langid;

static VALID_MODULE: ModuleData = ModuleData {
    name: "demo-module",
    domain: "demo-domain",
    supported_languages: &[langid!("en"), langid!("fr")],
    namespaces: &["ui", "errors"],
};
static DUPLICATE_MODULES: [&ModuleData; 2] = [
    &ModuleData {
        name: "dup-name",
        domain: "dup-domain-a",
        supported_languages: &[langid!("en"), langid!("en")],
        namespaces: &["ui", "ui"],
    },
    &ModuleData {
        name: "dup-name",
        domain: "dup-domain-a",
        supported_languages: &[],
        namespaces: &["../bad"],
    },
];

#[test]
fn module_data_resource_plan_uses_canonical_namespaced_paths() {
    let plan = VALID_MODULE.resource_plan();
    let keys: Vec<_> = plan.iter().map(|spec| spec.key.as_str()).collect();
    let paths: Vec<_> = plan
        .iter()
        .map(|spec| spec.locale_relative_path.as_str())
        .collect();

    assert_eq!(
        keys,
        vec!["demo-domain", "demo-domain/ui", "demo-domain/errors"]
    );
    assert_eq!(
        paths,
        vec![
            "demo-domain.ftl",
            "demo-domain/ui.ftl",
            "demo-domain/errors.ftl",
        ]
    );
    assert!(!plan[0].required);
    assert!(plan[1..].iter().all(|spec| spec.required));
}

#[test]
fn validate_module_registry_accepts_valid_metadata() {
    assert!(validate_module_registry([&VALID_MODULE]).is_ok());
}

#[test]
fn validate_module_registry_reports_duplicates_and_invalid_namespaces() {
    let errors =
        validate_module_registry(DUPLICATE_MODULES).expect_err("invalid registry should fail");

    assert!(errors.contains(&ModuleRegistryError::DuplicateModuleName {
        name: "dup-name".to_string(),
    }));
    assert!(errors.contains(&ModuleRegistryError::DuplicateDomain {
        domain: "dup-domain-a".to_string(),
    }));
    assert!(
        errors.contains(&ModuleRegistryError::DuplicateSupportedLanguage {
            module: "dup-name".to_string(),
            language: langid!("en"),
        })
    );
    assert!(errors.contains(&ModuleRegistryError::DuplicateNamespace {
        module: "dup-name".to_string(),
        namespace: "ui".to_string(),
    }));
    assert!(errors.iter().any(|error| matches!(
        error,
        ModuleRegistryError::InvalidNamespace { module, namespace, .. }
            if module == "dup-name" && namespace == "../bad"
    )));
}

#[test]
fn module_registry_error_messages_are_descriptive() {
    let cases = [
        (
            ModuleRegistryError::EmptyModuleName,
            "module name must not be empty",
        ),
        (
            ModuleRegistryError::EmptyDomain {
                module: "demo".to_string(),
            },
            "module 'demo' has an empty domain",
        ),
        (
            ModuleRegistryError::DuplicateModuleName {
                name: "demo".to_string(),
            },
            "duplicate module name 'demo'",
        ),
        (
            ModuleRegistryError::DuplicateDomain {
                domain: "demo".to_string(),
            },
            "duplicate module domain 'demo'",
        ),
        (
            ModuleRegistryError::DuplicateSupportedLanguage {
                module: "demo".to_string(),
                language: langid!("en"),
            },
            "module 'demo' declares duplicate language 'en'",
        ),
        (
            ModuleRegistryError::DuplicateNamespace {
                module: "demo".to_string(),
                namespace: "ui".to_string(),
            },
            "module 'demo' declares duplicate namespace 'ui'",
        ),
        (
            ModuleRegistryError::InvalidNamespace {
                module: "demo".to_string(),
                namespace: "../ui".to_string(),
                details: "must not contain '..' segments",
            },
            "module 'demo' has invalid namespace '../ui': must not contain '..' segments",
        ),
    ];

    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
    }
}

#[test]
fn static_module_descriptor_returns_original_module_data() {
    let descriptor = StaticModuleDescriptor::new(&VALID_MODULE);
    assert_eq!(descriptor.data(), &VALID_MODULE);
}

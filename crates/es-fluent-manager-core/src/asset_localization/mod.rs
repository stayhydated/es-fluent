//! Shared module metadata and discovery contracts.

mod loading;
mod module;
mod resource;

pub use loading::{
    LocaleLoadReport, ResourceLoadError, ResourceLoadStatus, build_locale_load_report,
    clear_locale_resource, collect_available_languages, collect_locale_resources,
    load_locale_resources, parse_and_store_locale_resource_content, parse_fluent_resource_bytes,
    parse_fluent_resource_content, record_failed_locale_resource, record_locale_resource_error,
    record_missing_locale_resource, store_locale_resource,
};
pub use module::{
    I18nModuleDescriptor, ModuleData, ModuleRegistryError, StaticModuleDescriptor,
    validate_module_registry,
};
pub use resource::{
    ModuleResourceSpec, ResourceKey, locale_is_ready, optional_resource_keys_from_plan,
    required_resource_keys_from_plan, resource_plan_for,
};

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_bundle::FluentResource;
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;
    use unic_langid::{LanguageIdentifier, langid};

    static SUPPORTED: &[LanguageIdentifier] = &[langid!("en-US"), langid!("fr")];
    static NAMESPACES: &[&str] = &["ui", "errors"];
    static DATA: ModuleData = ModuleData {
        name: "test-module",
        domain: "test-domain",
        supported_languages: SUPPORTED,
        namespaces: NAMESPACES,
    };

    #[test]
    fn static_descriptor_new_and_data_round_trip() {
        let module = StaticModuleDescriptor::new(&DATA);
        let data = module.data();

        assert_eq!(data.name, "test-module");
        assert_eq!(data.domain, "test-domain");
        assert_eq!(data.supported_languages, SUPPORTED);
        assert_eq!(data.namespaces, NAMESPACES);
    }

    #[test]
    fn resource_key_helpers_return_expected_shapes() {
        let key = ResourceKey::new("app/ui");
        assert_eq!(key.as_str(), "app/ui");
        assert_eq!(key.domain(), "app");
        assert_eq!(key.to_string(), "app/ui");
    }

    #[test]
    fn resource_plan_without_namespaces_requires_base_file() {
        let plan = resource_plan_for("app", &[]);
        assert_eq!(
            plan,
            vec![ModuleResourceSpec {
                key: ResourceKey::new("app"),
                locale_relative_path: "app.ftl".to_string(),
                required: true
            }]
        );
    }

    #[test]
    fn resource_plan_with_namespaces_requires_namespace_files_and_keeps_base_optional() {
        let plan = resource_plan_for("app", &["ui", "errors"]);
        assert_eq!(
            plan,
            vec![
                ModuleResourceSpec {
                    key: ResourceKey::new("app"),
                    locale_relative_path: "app.ftl".to_string(),
                    required: false
                },
                ModuleResourceSpec {
                    key: ResourceKey::new("app/ui"),
                    locale_relative_path: "app/ui.ftl".to_string(),
                    required: true
                },
                ModuleResourceSpec {
                    key: ResourceKey::new("app/errors"),
                    locale_relative_path: "app/errors.ftl".to_string(),
                    required: true
                }
            ]
        );
        assert_eq!(plan[0].locale_path(&langid!("en-US")), "en-US/app.ftl");
        assert_eq!(plan[1].locale_path(&langid!("en-US")), "en-US/app/ui.ftl");
    }

    #[test]
    fn resource_plan_with_nested_namespaces_preserves_relative_paths() {
        let plan = resource_plan_for("app", &["ui/button"]);
        assert_eq!(
            plan,
            vec![
                ModuleResourceSpec {
                    key: ResourceKey::new("app"),
                    locale_relative_path: "app.ftl".to_string(),
                    required: false
                },
                ModuleResourceSpec {
                    key: ResourceKey::new("app/ui/button"),
                    locale_relative_path: "app/ui/button.ftl".to_string(),
                    required: true
                }
            ]
        );
    }

    #[test]
    fn resource_plan_deduplicates_duplicate_namespaces() {
        let plan = resource_plan_for("app", &["ui", "ui"]);
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].key, ResourceKey::new("app"));
        assert_eq!(plan[1].key, ResourceKey::new("app/ui"));
    }

    #[test]
    fn locale_is_ready_requires_all_required_keys() {
        let plan = resource_plan_for("app", &["ui", "errors"]);
        let required = required_resource_keys_from_plan(&plan);
        let optional = optional_resource_keys_from_plan(&plan);

        assert_eq!(optional, HashSet::from([ResourceKey::new("app")]));

        let ready_loaded =
            HashSet::from([ResourceKey::new("app/ui"), ResourceKey::new("app/errors")]);
        assert!(locale_is_ready(&required, &ready_loaded));

        let missing_required = HashSet::from([ResourceKey::new("app/ui")]);
        assert!(!locale_is_ready(&required, &missing_required));
    }

    #[test]
    fn locale_load_report_tracks_errors_and_readiness() {
        let plan = resource_plan_for("app", &["ui"]);
        let mut report = LocaleLoadReport::from_plan(&plan);

        report.mark_loaded(ResourceKey::new("app/ui"));

        assert!(report.is_ready());
        assert_eq!(
            report.required_keys(),
            &HashSet::from([ResourceKey::new("app/ui")])
        );
        assert_eq!(
            report.optional_keys(),
            &HashSet::from([ResourceKey::new("app")])
        );
        assert!(report.loaded_keys().contains(&ResourceKey::new("app/ui")));
        assert_eq!(report.missing_required_keys(), HashSet::new());
    }

    #[test]
    fn validate_module_registry_rejects_duplicates_and_invalid_namespaces() {
        static DUP_LANGUAGE: &[LanguageIdentifier] = &[langid!("en"), langid!("en")];
        static INVALID_NAMESPACES: &[&str] = &[
            "ui",
            "ui",
            "",
            "errors.ftl",
            "bad//path",
            r"bad\path",
            "../escape",
            " ui ",
        ];
        static BAD_DATA: ModuleData = ModuleData {
            name: "test-module",
            domain: "test-domain",
            supported_languages: DUP_LANGUAGE,
            namespaces: INVALID_NAMESPACES,
        };
        static DUP_DOMAIN: ModuleData = ModuleData {
            name: "other-module",
            domain: "test-domain",
            supported_languages: SUPPORTED,
            namespaces: &[],
        };

        let errs = validate_module_registry([&DATA, &BAD_DATA, &DUP_DOMAIN])
            .expect_err("validation should fail");
        assert!(errs.iter().any(|err| matches!(
            err,
            ModuleRegistryError::DuplicateModuleName { name } if name == "test-module"
        )));
        assert!(errs.iter().any(|err| matches!(
            err,
            ModuleRegistryError::DuplicateDomain { domain } if domain == "test-domain"
        )));
        assert!(errs.iter().any(|err| matches!(
            err,
            ModuleRegistryError::DuplicateSupportedLanguage { module, .. } if module == "test-module"
        )));
        assert!(errs.iter().any(|err| matches!(
            err,
            ModuleRegistryError::DuplicateNamespace { module, namespace } if module == "test-module" && namespace == "ui"
        )));
        assert!(errs.iter().any(|err| matches!(
            err,
            ModuleRegistryError::InvalidNamespace { module, namespace, details }
                if module == "test-module"
                    && namespace == "bad//path"
                    && details == &"namespace path must not contain empty segments"
        )));
        assert!(errs.iter().any(|err| matches!(
            err,
            ModuleRegistryError::InvalidNamespace { module, namespace, details }
                if module == "test-module"
                    && namespace == r"bad\path"
                    && details == &"namespace must use '/' as path separator"
        )));
        assert!(errs.iter().any(|err| matches!(
            err,
            ModuleRegistryError::InvalidNamespace { module, namespace, details }
                if module == "test-module"
                    && namespace == "../escape"
                    && details == &"namespace path must not contain '.' or '..' segments"
        )));
        assert!(errs.iter().any(|err| matches!(
            err,
            ModuleRegistryError::InvalidNamespace { module, namespace, details }
                if module == "test-module"
                    && namespace == " ui "
                    && details == &"namespace must not have leading or trailing whitespace"
        )));
    }

    #[test]
    fn validate_module_registry_accepts_path_based_namespaces() {
        static PATH_NAMESPACES: &[&str] = &["ui/button", "errors/forms"];
        static PATH_DATA: ModuleData = ModuleData {
            name: "path-module",
            domain: "path-domain",
            supported_languages: SUPPORTED,
            namespaces: PATH_NAMESPACES,
        };

        validate_module_registry([&PATH_DATA]).expect("path-based namespaces should be valid");
    }

    #[test]
    fn module_data_resource_plan_delegates_to_shared_builder() {
        let plan = DATA.resource_plan();
        let direct = resource_plan_for(DATA.domain, DATA.namespaces);
        assert_eq!(plan, direct);
    }

    #[test]
    fn parse_fluent_resource_content_reports_parse_errors() {
        let spec = ModuleResourceSpec {
            key: ResourceKey::new("app/ui"),
            locale_relative_path: "app/ui.ftl".to_string(),
            required: true,
        };

        let err = parse_fluent_resource_content(&spec, "broken = {".to_string())
            .expect_err("invalid fluent should fail");
        assert!(matches!(
            err,
            ResourceLoadError::Parse { required: true, .. }
        ));
    }

    #[test]
    fn load_locale_resources_centralizes_report_bookkeeping() {
        let plan = resource_plan_for("app", &["ui"]);
        let (resources, report) = load_locale_resources(&plan, |spec| {
            if spec.key == ResourceKey::new("app/ui") {
                ResourceLoadStatus::Loaded(
                    FluentResource::try_new("hello = Hello".to_string())
                        .map(Arc::new)
                        .expect("valid resource"),
                )
            } else {
                ResourceLoadStatus::Missing
            }
        });

        assert_eq!(resources.len(), 1);
        assert!(report.is_ready());
        assert_eq!(report.errors().len(), 1);
        assert!(matches!(
            report.errors()[0],
            ResourceLoadError::Missing {
                required: false,
                ..
            }
        ));
    }

    #[test]
    fn locale_state_helpers_track_reports_resources_and_languages() {
        let lang = langid!("en");
        let spec = ModuleResourceSpec {
            key: ResourceKey::new("app/ui"),
            locale_relative_path: "app/ui.ftl".to_string(),
            required: true,
        };

        let mut specs = HashMap::new();
        specs.insert((lang.clone(), spec.key.clone()), spec.clone());
        let mut loaded = HashMap::new();
        let mut errors = HashMap::new();

        parse_and_store_locale_resource_content(
            &mut loaded,
            &mut errors,
            &lang,
            &spec,
            "hello = Hello".to_string(),
        )
        .expect("store resource");

        let report = build_locale_load_report(&specs, &loaded, &errors, &lang);
        assert!(report.is_ready());
        assert_eq!(collect_locale_resources(&loaded, &lang).len(), 1);
        assert_eq!(collect_available_languages(&specs), vec![lang.clone()]);

        let err =
            record_failed_locale_resource(&mut loaded, &mut errors, &lang, &spec, "watcher error");
        assert!(err.is_required());

        let report = build_locale_load_report(&specs, &loaded, &errors, &lang);
        assert!(!report.is_ready());

        clear_locale_resource(&mut loaded, &mut errors, &lang, &spec.key);
        assert!(collect_locale_resources(&loaded, &lang).is_empty());
    }
}

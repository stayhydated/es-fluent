use super::*;

#[test]
fn loaded_dioxus_asset_i18n_localizes_selected_language() {
    let i18n = DioxusAssetI18n::new_with_loaded_modules(
        vec![loaded_module()],
        langid!("en"),
        LanguageSelectionPolicy::BestEffort,
    )
    .expect("initial language should load");

    assert_eq!(
        i18n.localize(static_key("test-app", "hello"), None),
        Some("Hello".to_string())
    );
    assert_eq!(i18n.requested_language(), langid!("en"));
    assert_eq!(
        i18n.localize(static_key("test-app", "hello"), None),
        Some("Hello".to_string())
    );
    assert!(i18n == i18n.clone());
    i18n.select_language(langid!("en"))
        .expect("selecting the active language should be a no-op");
    let mut looked_up = None;
    i18n.with_lookup(&mut |lookup| {
        looked_up = lookup(static_key("test-app", "hello"), None);
    });
    assert_eq!(looked_up, Some("Hello".to_string()));
    assert_eq!(i18n.localize_message(&TestMessage), "Hello");
}

/// A runtime utility module, like a language-name provider: it follows the
/// manager's selected locale but declares no locales of its own.
mod follower {
    use es_fluent::registry::StaticFluentMessageKey;
    use es_fluent_manager_core::{
        FluentArgumentMap, I18nModule, I18nModuleDescriptor, I18nModuleRegistration,
        LocalizationError, Localizer, ModuleData, ModuleDomain,
    };
    use unic_langid::LanguageIdentifier;

    pub(super) static DATA: ModuleData = ModuleData {
        name: "test-follower",
        owner: es_fluent_manager_core::__macro::static_domain("test-follower"),
        supported_languages: &[],
        domains: &[ModuleDomain {
            domain: es_fluent_manager_core::__macro::static_domain("test-follower"),
            namespaces: &[],
        }],
    };

    struct FollowerModule;
    static MODULE: FollowerModule = FollowerModule;
    inventory::submit! { &MODULE as &dyn I18nModuleRegistration }

    impl I18nModuleDescriptor for FollowerModule {
        fn data(&self) -> &'static ModuleData {
            &DATA
        }
    }

    impl I18nModule for FollowerModule {
        fn create_localizer(
            &self,
            lang: &LanguageIdentifier,
        ) -> Result<Box<dyn Localizer>, LocalizationError> {
            Ok(Box::new(SelectedLocale(lang.to_string())))
        }

        fn contributes_to_language_selection(&self) -> bool {
            false
        }
    }

    /// Answers `selected-locale` with the locale its localizer was created for.
    struct SelectedLocale(String);

    impl Localizer for SelectedLocale {
        fn localize<'a>(
            &self,
            key: StaticFluentMessageKey,
            _: Option<&FluentArgumentMap<'a>>,
        ) -> Option<String> {
            (key.owner() == DATA.owner
                && DATA.owns_domain(key.domain())
                && key.id().as_str() == "selected-locale")
                .then(|| self.0.clone())
        }
    }
}

#[test]
fn loaded_dioxus_asset_i18n_localizes_runtime_follower_messages() {
    let i18n = DioxusAssetI18n::new_with_loaded_modules(
        vec![
            loaded_module_for_language(langid!("en"), "hello = Hello"),
            loaded_module_for_language(langid!("fr"), "hello = Bonjour"),
        ],
        langid!("en"),
        LanguageSelectionPolicy::BestEffort,
    )
    .expect("initial language should load");

    // Discovered runtime followers serve the manager's locale without owning it.
    for _ in 0..2 {
        assert_eq!(
            i18n.localize(static_key("test-follower", "selected-locale"), None),
            Some("en".to_string())
        );
    }
    i18n.select_language(langid!("fr"))
        .expect("the application module supports fr");
    let mut looked_up = None;
    i18n.with_lookup(&mut |lookup| {
        looked_up = lookup(static_key("test-follower", "selected-locale"), None);
    });
    assert_eq!(looked_up, Some("fr".to_string()));
    assert_eq!(
        i18n.localize(static_key("test-follower", "other-message"), None),
        None
    );
    assert_eq!(
        i18n.localize(static_key("other-owner", "selected-locale"), None),
        None
    );
}

#[test]
fn runtime_followers_do_not_make_unsupported_locales_selectable() {
    let i18n = DioxusAssetI18n::new_with_loaded_modules(
        vec![loaded_module()],
        langid!("en"),
        LanguageSelectionPolicy::BestEffort,
    )
    .expect("initial language should load");

    assert!(i18n.select_language(langid!("de")).is_err());
    assert_eq!(i18n.requested_language(), langid!("en"));
    assert_eq!(
        i18n.localize(static_key("test-follower", "selected-locale"), None),
        Some("en".to_string())
    );
}

#[test]
fn loaded_dioxus_asset_i18n_reports_initial_language_errors() {
    let error = match DioxusAssetI18n::new_with_loaded_modules(
        vec![loaded_module()],
        langid!("de"),
        LanguageSelectionPolicy::BestEffort,
    ) {
        Ok(_) => panic!("unsupported language should fail"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        DioxusAssetLoadError::LanguageSelection { .. }
    ));
    assert!(error.resource_errors().is_empty());
    assert!(
        error
            .to_string()
            .contains("failed to select the requested language")
    );
    assert!(error.source().is_some());
}

#[test]
fn dioxus_asset_load_error_reports_discovery_details() {
    let error = DioxusAssetLoadError::ModuleDiscovery(Arc::from([]));

    assert!(error.resource_errors().is_empty());
    assert!(
        error
            .to_string()
            .contains("failed strict i18n module discovery")
    );
    assert!(error.source().is_none());
}

#[test]
fn localizer_uses_language_fallbacks() {
    let i18n = DioxusAssetI18n::new_with_loaded_modules(
        vec![loaded_fallback_module()],
        langid!("en-US"),
        LanguageSelectionPolicy::BestEffort,
    )
    .expect("fallback language should load");

    assert_eq!(i18n.requested_language(), langid!("en-US"));
    assert_eq!(
        i18n.localize(static_key("fallback-app", "fallback"), None),
        Some("English fallback".to_string())
    );
}

#[test]
fn strict_selection_rejects_partial_module_failures() {
    let i18n = DioxusAssetI18n::new_with_loaded_modules(
        vec![
            loaded_module_for_language(langid!("en"), "hello = Hello"),
            loaded_module_for_language(langid!("fr"), "hello = Bonjour"),
        ],
        langid!("en"),
        LanguageSelectionPolicy::BestEffort,
    )
    .expect("best effort should accept one selected module");

    assert!(i18n.select_language_strict(langid!("en")).is_err());
    i18n.select_language(langid!("fr"))
        .expect("best effort should switch to fr");
    assert_eq!(i18n.requested_language(), langid!("fr"));
}

#[test]
fn bundle_assembly_errors_are_returned_for_initial_locale() {
    let error = es_fluent_manager_core::I18nModule::create_localizer(
        &duplicate_resource_module(),
        &langid!("en"),
    )
    .err()
    .expect("duplicate messages should fail the initial bundle");

    assert!(!matches!(error, LocalizationError::LanguageNotSupported(_)));
}

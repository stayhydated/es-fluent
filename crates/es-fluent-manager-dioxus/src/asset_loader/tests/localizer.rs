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

#[test]
fn loaded_dioxus_asset_i18n_localizes_runtime_follower_messages() {
    let _ = es_fluent_lang::force_link();
    let i18n = DioxusAssetI18n::new_with_loaded_modules(
        vec![loaded_module()],
        langid!("en"),
        LanguageSelectionPolicy::BestEffort,
    )
    .expect("initial language should load");

    assert_eq!(
        i18n.localize(static_key("es-fluent-lang", "es-fluent-lang-en"), None),
        Some("English".to_string())
    );
    assert_eq!(
        i18n.localize(static_key("es-fluent-lang", "es-fluent-lang-en"), None,),
        Some("English".to_string())
    );
    let mut looked_up = None;
    i18n.with_lookup(&mut |lookup| {
        looked_up = lookup(static_key("es-fluent-lang", "es-fluent-lang-en"), None);
    });
    assert_eq!(looked_up, Some("English".to_string()));
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
    let error = duplicate_resource_module()
        .create_localizer()
        .select_language(&langid!("en"))
        .expect_err("duplicate messages should fail the initial bundle");

    assert!(!matches!(error, LocalizationError::LanguageNotSupported(_)));
}

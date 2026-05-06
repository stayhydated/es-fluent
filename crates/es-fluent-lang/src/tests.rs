use super::*;
use unic_langid::langid;

#[test]
fn parse_message_language_extracts_language_identifier() {
    assert_eq!(
        parse_message_language("es-fluent-lang-fr-FR"),
        Some(langid!("fr-FR"))
    );
    assert_eq!(parse_message_language("missing-prefix"), None);
    assert_eq!(parse_message_language("es-fluent-lang-invalid!"), None);
}

#[test]
fn formatter_candidates_include_requested_chain_and_defaults() {
    let candidates = formatter_candidates(&langid!("fr-CA"));

    assert!(candidates.contains(&langid!("fr-CA")));
    assert!(candidates.contains(&langid!("fr")));
    assert!(candidates.contains(&langid!("en")));
    assert!(candidates.contains(&langid!("en-001")));
}

#[test]
fn language_module_creates_localizer_and_selects_language() {
    let module = EsFluentLanguageModule;
    assert_eq!(module.data().name, "es-fluent-lang");
    assert!(!I18nModule::contributes_to_language_selection(&module));

    let localizer = I18nModule::create_localizer(&module);
    localizer
        .select_language(&langid!("en-US"))
        .expect("language selection should succeed");

    #[cfg(not(feature = "localized-langs"))]
    assert_eq!(
        localizer.localize("es-fluent-lang-fr", None),
        Some(expected_french_name())
    );

    #[cfg(feature = "localized-langs")]
    assert_eq!(
        localizer.localize("es-fluent-lang-fr", None),
        Some("French".to_string())
    );
}

#[cfg(not(feature = "localized-langs"))]
#[test]
fn autonym_mode_returns_native_language_names_regardless_of_selected_locale() {
    let module = EsFluentLanguageModule;
    let localizer = I18nModule::create_localizer(&module);

    localizer
        .select_language(&langid!("en-US"))
        .expect("language selection should succeed");

    assert_eq!(
        localizer.localize("es-fluent-lang-en", None),
        Some("English".to_string())
    );
    assert_eq!(
        localizer.localize("es-fluent-lang-fr", None),
        Some(expected_french_name())
    );
    assert_eq!(
        localizer.localize("es-fluent-lang-ja", None),
        Some(expected_japanese_name())
    );

    localizer
        .select_language(&langid!("ja-JP"))
        .expect("language selection should succeed");

    assert_eq!(
        localizer.localize("es-fluent-lang-fr", None),
        Some(expected_french_name())
    );
    assert_eq!(
        localizer.localize("es-fluent-lang-ja", None),
        Some(expected_japanese_name())
    );
}

#[cfg(feature = "localized-langs")]
#[test]
fn localized_mode_returns_translated_language_names() {
    let module = EsFluentLanguageModule;
    let localizer = I18nModule::create_localizer(&module);

    localizer
        .select_language(&langid!("en"))
        .expect("English language selection should succeed");
    assert_eq!(
        localizer.localize("es-fluent-lang-fr", None),
        Some("French".to_string())
    );
    assert_eq!(
        localizer.localize("es-fluent-lang-de", None),
        Some("German".to_string())
    );
    assert_eq!(
        localizer.localize("es-fluent-lang-ja", None),
        Some("Japanese".to_string())
    );

    localizer
        .select_language(&langid!("fr"))
        .expect("French language selection should succeed");
    assert_eq!(
        localizer.localize("es-fluent-lang-fr", None),
        Some(expected_french_name())
    );
    assert_eq!(
        localizer.localize("es-fluent-lang-de", None),
        Some("allemand".to_string())
    );
    assert_eq!(
        localizer.localize("es-fluent-lang-en", None),
        Some("anglais".to_string())
    );
}

#[cfg(feature = "localized-langs")]
#[test]
fn localized_mode_falls_back_to_parent_display_locale() {
    let module = EsFluentLanguageModule;
    let localizer = I18nModule::create_localizer(&module);

    localizer
        .select_language(&langid!("fr-FR"))
        .expect("fr-FR should fall back to fr");
    assert_eq!(
        localizer.localize("es-fluent-lang-en", None),
        Some("anglais".to_string())
    );
}

#[cfg(feature = "localized-langs")]
#[test]
fn localized_mode_ignores_unused_args() {
    let module = EsFluentLanguageModule;
    let localizer = I18nModule::create_localizer(&module);
    let mut args = HashMap::new();
    args.insert("unused", FluentValue::from("value"));

    localizer
        .select_language(&langid!("en"))
        .expect("language selection should succeed");
    assert_eq!(
        localizer.localize("es-fluent-lang-fr", Some(&args)),
        Some("French".to_string())
    );
}

#[test]
fn uses_standard_module_registration() {
    let registration = inventory::iter::<&'static dyn I18nModuleRegistration>()
        .find(|registration| registration.data().domain == "es-fluent-lang")
        .expect("es-fluent-lang module registration should be present");
    let localizer = registration
        .create_localizer()
        .expect("es-fluent-lang should provide a localizer");
    localizer
        .select_language(&langid!("en-US"))
        .expect("language selection should succeed");

    assert_eq!(
        localizer.localize("es-fluent-lang-en", None),
        Some("English".to_string())
    );
}

#[test]
fn force_link_reports_linked_resources() {
    assert!(force_link() > 0);
}

fn expected_french_name() -> String {
    "français".to_string()
}

#[cfg(not(feature = "localized-langs"))]
fn expected_japanese_name() -> String {
    "日本語".to_string()
}

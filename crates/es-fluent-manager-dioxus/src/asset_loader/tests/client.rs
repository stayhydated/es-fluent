use super::*;

#[cfg(feature = "client")]
#[test]
#[serial]
fn asset_i18n_context_localizes_through_provider_hook() {
    let i18n = DioxusAssetI18n::new_with_loaded_modules(
        vec![loaded_module()],
        langid!("en"),
        LanguageSelectionPolicy::BestEffort,
    )
    .expect("initial language should load");
    let mut dom =
        VirtualDom::new_with_props(AssetContextMessage, AssetContextMessageProps { i18n });

    dom.rebuild_in_place();

    assert!(dioxus_ssr::render(&dom).contains("Hello"));
}

#[cfg(feature = "client")]
#[test]
#[serial]
fn asset_i18n_handle_methods_update_tracked_language_and_lookup() {
    let i18n = DioxusAssetI18n::new_with_loaded_modules(
        vec![loaded_multilingual_module()],
        langid!("en"),
        LanguageSelectionPolicy::BestEffort,
    )
    .expect("initial language should load");
    let mut dom =
        VirtualDom::new_with_props(AssetHandleExercise, AssetHandleExerciseProps { i18n });

    dom.rebuild_in_place();

    let rendered = dioxus_ssr::render(&dom);
    assert!(rendered.contains("en|en|fr|Hello|Hello|Hello"));
}

#[cfg(feature = "client")]
#[test]
#[serial]
fn use_i18n_reports_missing_context() {
    let mut dom = VirtualDom::new(MissingAssetContextMessage);

    dom.rebuild_in_place();

    assert!(dioxus_ssr::render(&dom).contains("missing"));
}

#[cfg(feature = "client")]
#[test]
fn log_asset_provider_load_error_once_is_idempotent() {
    let logged = std::rc::Rc::new(std::cell::Cell::new(false));
    let error = DioxusAssetLoadError::language_selection(
        LocalizationError::LanguageNotSupported(langid!("de")),
        &[],
    );

    log_asset_provider_load_error_once(&error, &logged);
    assert!(logged.get());
    log_asset_provider_load_error_once(&error, &logged);
    assert!(logged.get());
}

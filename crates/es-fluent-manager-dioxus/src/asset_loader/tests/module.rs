use super::*;

#[test]
fn dioxus_i18n_asset_modules_debug_equality_and_slice_are_stable() {
    let modules = DioxusI18nAssetModules::new(ASSET_MODULES);
    let same = DioxusI18nAssetModules::new(ASSET_MODULES);
    let different = DioxusI18nAssetModules::new(INVALID_ASSET_MODULES);
    let resource = DioxusI18nAssetResource::new(
        langid!("en"),
        "asset-test",
        "asset-test.ftl",
        true,
        ASSET_RESOURCES[0].asset,
    );
    let module = DioxusI18nAssetModule::new(&ASSET_DATA, &[]);

    assert_eq!(modules, same);
    assert_ne!(modules, different);
    assert_eq!(modules.as_slice().len(), 1);
    assert_eq!(format!("{modules:?}"), "DioxusI18nAssetModules { len: 1 }");
    assert_eq!(resource.spec(), ASSET_RESOURCES[0].spec());
    assert_eq!(module.resources.len(), 0);
}

#[test]
fn load_modules_reads_assets_and_selects_languages() {
    let modules = DioxusI18nAssetModules::new(ASSET_MODULES);
    let i18n = futures::executor::block_on(DioxusAssetI18n::load_modules(
        modules,
        langid!("en"),
        LanguageSelectionPolicy::BestEffort,
    ))
    .expect("asset module should load");

    assert_eq!(
        i18n.localize(static_key("asset-test", "asset-hello"), None),
        Some("Hello from asset".to_string())
    );
    i18n.select_language(langid!("fr"))
        .expect("asset i18n should select fr");
    assert_eq!(
        i18n.localize(static_key("asset-test", "asset-hello"), None),
        Some("Bonjour from asset".to_string())
    );
}

#[test]
fn load_modules_collects_parse_errors_for_language_selection_failures() {
    let modules = DioxusI18nAssetModules::new(INVALID_ASSET_MODULES);
    let error = match futures::executor::block_on(DioxusAssetI18n::load_modules(
        modules,
        langid!("en"),
        LanguageSelectionPolicy::BestEffort,
    )) {
        Ok(_) => panic!("invalid FTL should prevent locale readiness"),
        Err(error) => error,
    };

    assert_eq!(error.resource_errors().len(), 1);
}

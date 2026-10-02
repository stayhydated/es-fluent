use crate::{DioxusAssetI18n, DioxusAssetLoadError, DioxusI18nAssetModules};
use es_fluent::{
    FluentArgs, FluentLocalizer, FluentLocalizerLookup, FluentMessage,
    registry::StaticFluentMessageKey,
};
use es_fluent_manager_core::{LanguageSelectionPolicy, LocalizationError};
use unic_langid::LanguageIdentifier;

/// SSR localization runtime backed by Dioxus asset loading.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SsrI18nRuntime {
    modules: DioxusI18nAssetModules,
}

impl SsrI18nRuntime {
    pub const fn new(modules: DioxusI18nAssetModules) -> Self {
        Self { modules }
    }

    pub const fn discovered() -> Self {
        Self::new(DioxusI18nAssetModules::discovered())
    }

    pub async fn request<L: Into<LanguageIdentifier>>(
        &self,
        language: L,
    ) -> Result<SsrI18n, DioxusAssetLoadError> {
        self.request_with_policy(language, LanguageSelectionPolicy::BestEffort)
            .await
    }

    pub async fn request_strict<L: Into<LanguageIdentifier>>(
        &self,
        language: L,
    ) -> Result<SsrI18n, DioxusAssetLoadError> {
        self.request_with_policy(language, LanguageSelectionPolicy::Strict)
            .await
    }

    pub async fn request_with_policy<L: Into<LanguageIdentifier>>(
        &self,
        language: L,
        selection_policy: LanguageSelectionPolicy,
    ) -> Result<SsrI18n, DioxusAssetLoadError> {
        let i18n = DioxusAssetI18n::load_modules(self.modules, language, selection_policy).await?;
        Ok(SsrI18n { i18n })
    }

    pub fn request_blocking<L: Into<LanguageIdentifier>>(
        &self,
        language: L,
    ) -> Result<SsrI18n, DioxusAssetLoadError> {
        futures::executor::block_on(self.request(language))
    }

    pub fn request_strict_blocking<L: Into<LanguageIdentifier>>(
        &self,
        language: L,
    ) -> Result<SsrI18n, DioxusAssetLoadError> {
        futures::executor::block_on(self.request_strict(language))
    }
}

impl Default for SsrI18nRuntime {
    fn default() -> Self {
        Self::discovered()
    }
}

/// Request-scoped Dioxus SSR localization state.
#[derive(Clone, Eq, PartialEq)]
pub struct SsrI18n {
    i18n: DioxusAssetI18n,
}

impl SsrI18n {
    pub fn requested_language(&self) -> LanguageIdentifier {
        self.i18n.requested_language()
    }

    pub fn select_language<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        self.i18n.select_language(lang)
    }

    pub fn select_language_strict<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        self.i18n.select_language_strict(lang)
    }

    pub fn localize_message<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        self.i18n.localize_message(message)
    }

    #[cfg(feature = "client")]
    pub fn provide_context(&self) -> crate::DioxusAssetI18nHandle {
        crate::use_provide_asset_i18n(self.i18n.clone())
    }
}

impl FluentLocalizer for SsrI18n {
    fn localize<'a>(
        &self,
        key: StaticFluentMessageKey,
        args: Option<&FluentArgs<'a>>,
    ) -> Option<String> {
        FluentLocalizer::localize(&self.i18n, key, args)
    }

    fn with_lookup(&self, f: &mut dyn FnMut(&mut FluentLocalizerLookup<'_>)) {
        FluentLocalizer::with_lookup(&self.i18n, f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DioxusI18nAssetModule, DioxusI18nAssetResource};
    use dioxus::prelude::manganis;
    use dioxus_core::{Element, VirtualDom};
    use dioxus_core_macro::{Props, component, rsx};
    use es_fluent::FluentLocalizerExt as _;
    use es_fluent_manager_core::{ModuleData, ModuleDomain};
    use unic_langid::{LanguageIdentifier, langid};

    fn static_key(id: &'static str) -> StaticFluentMessageKey {
        es_fluent::registry::__macro::static_message_key(
            "asset-test",
            es_fluent::registry::__macro::static_domain("asset-test"),
            es_fluent::registry::__macro::static_entry_id(id),
        )
    }

    static SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en"), langid!("fr")];
    static MODULE_DATA: ModuleData = ModuleData {
        name: "asset-test",
        owner: es_fluent_manager_core::__macro::static_domain("asset-test"),
        supported_languages: SUPPORTED_LANGUAGES,
        domains: &[ModuleDomain {
            domain: es_fluent_manager_core::__macro::static_domain("asset-test"),
            namespaces: &[],
        }],
    };
    static RESOURCES: &[DioxusI18nAssetResource] = &[
        DioxusI18nAssetResource::new(
            langid!("en"),
            "asset-test",
            "asset-test.ftl",
            true,
            dioxus::prelude::asset!("/tests/fixtures/dioxus_i18n/en/asset-test.ftl"),
        ),
        DioxusI18nAssetResource::new(
            langid!("fr"),
            "asset-test",
            "asset-test.ftl",
            true,
            dioxus::prelude::asset!("/tests/fixtures/dioxus_i18n/fr/asset-test.ftl"),
        ),
    ];
    static MODULE: DioxusI18nAssetModule = DioxusI18nAssetModule::new(&MODULE_DATA, RESOURCES);
    static MODULES: &[&DioxusI18nAssetModule] = &[&MODULE];

    struct TestMessage;

    impl FluentMessage for TestMessage {
        fn to_fluent_string_with(
            &self,
            localize: &mut es_fluent::FluentMessageLookup<'_>,
        ) -> String {
            localize(static_key("asset-hello"), None)
        }
    }

    #[allow(non_snake_case)]
    #[component]
    fn SsrMessage(i18n: SsrI18n) -> Element {
        let message = i18n.localize_message(&TestMessage);
        rsx! { "{message}" }
    }

    fn runtime() -> SsrI18nRuntime {
        SsrI18nRuntime::new(DioxusI18nAssetModules::new(MODULES))
    }

    #[test]
    fn ssr_runtime_requests_and_switches_languages() {
        let runtime = runtime();
        assert_eq!(
            runtime,
            SsrI18nRuntime::new(DioxusI18nAssetModules::new(MODULES))
        );

        let i18n = futures::executor::block_on(runtime.request(langid!("en")))
            .expect("SSR request should load assets");
        assert_eq!(i18n.requested_language(), langid!("en"));
        assert_eq!(
            i18n.localize(static_key("asset-hello"), None),
            Some("Hello from asset".to_string())
        );
        assert_eq!(i18n.localize_message(&TestMessage), "Hello from asset");

        i18n.select_language(langid!("fr"))
            .expect("SSR request should switch language");
        assert_eq!(i18n.requested_language(), langid!("fr"));
        assert_eq!(
            i18n.localize(static_key("asset-hello"), None),
            Some("Bonjour from asset".to_string())
        );

        i18n.select_language_strict(langid!("en"))
            .expect("strict SSR language switch should work");
        let mut looked_up = None;
        i18n.with_lookup(&mut |lookup| {
            looked_up = lookup(static_key("asset-hello"), None);
        });
        assert_eq!(looked_up, Some("Hello from asset".to_string()));
    }

    #[test]
    fn ssr_runtime_blocking_and_policy_requests_report_errors() {
        let runtime = runtime();
        let strict = futures::executor::block_on(
            runtime.request_with_policy(langid!("en"), LanguageSelectionPolicy::Strict),
        )
        .expect("strict policy should load all modules");
        assert_eq!(strict.localize_message(&TestMessage), "Hello from asset");

        let blocking = runtime
            .request_blocking(langid!("fr"))
            .expect("blocking request should load assets");
        assert_eq!(
            blocking.localize_message(&TestMessage),
            "Bonjour from asset"
        );

        assert!(runtime.request_strict_blocking(langid!("de")).is_err());
        assert!(futures::executor::block_on(runtime.request_strict(langid!("de"))).is_err());
    }

    #[test]
    fn ssr_requests_render_independent_locales_through_dioxus() {
        let runtime = runtime();
        let english = runtime
            .request_blocking(langid!("en"))
            .expect("English SSR request should load assets");
        let french = runtime
            .request_blocking(langid!("fr"))
            .expect("French SSR request should load assets");

        assert_eq!(
            english.try_localize_message(&TestMessage),
            Some("Hello from asset".to_string())
        );
        let mut english_dom =
            VirtualDom::new_with_props(SsrMessage, SsrMessageProps { i18n: english });
        let mut french_dom =
            VirtualDom::new_with_props(SsrMessage, SsrMessageProps { i18n: french });
        english_dom.rebuild_in_place();
        french_dom.rebuild_in_place();

        assert!(dioxus_ssr::render(&english_dom).contains("Hello from asset"));
        assert!(dioxus_ssr::render(&french_dom).contains("Bonjour from asset"));
    }
}

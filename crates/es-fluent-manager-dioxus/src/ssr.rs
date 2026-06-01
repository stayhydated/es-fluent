use crate::{DioxusAssetI18n, DioxusAssetLoadError, DioxusI18nAssetModules};
use dioxus_core::{Element, VirtualDom};
use dioxus_ssr::Renderer;
use es_fluent::{FluentLocalizer, FluentMessage, FluentValue};
use es_fluent_manager_core::{LanguageSelectionPolicy, LocalizationError};
use std::collections::HashMap;
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

    pub fn rebuild_and_render(&self, dom: &mut VirtualDom) -> String {
        dom.rebuild_in_place();
        dioxus_ssr::render(dom)
    }

    pub fn rebuild_and_pre_render(&self, dom: &mut VirtualDom) -> String {
        dom.rebuild_in_place();
        dioxus_ssr::pre_render(dom)
    }

    pub fn render(&self, dom: &VirtualDom) -> String {
        dioxus_ssr::render(dom)
    }

    pub fn pre_render(&self, dom: &VirtualDom) -> String {
        dioxus_ssr::pre_render(dom)
    }

    pub fn render_with(&self, renderer: &mut Renderer, dom: &VirtualDom) -> String {
        renderer.render(dom)
    }

    pub fn render_element(&self, element: Element) -> String {
        dioxus_ssr::render_element(element)
    }
}

impl FluentLocalizer for SsrI18n {
    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        FluentLocalizer::localize(&self.i18n, id, args)
    }

    fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        FluentLocalizer::localize_in_domain(&self.i18n, domain, id, args)
    }

    fn with_lookup(
        &self,
        f: &mut dyn FnMut(
            &mut dyn for<'a> FnMut(
                &str,
                &str,
                Option<&HashMap<&str, FluentValue<'a>>>,
            ) -> Option<String>,
        ),
    ) {
        FluentLocalizer::with_lookup(&self.i18n, f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DioxusI18nAssetModule, DioxusI18nAssetResource};
    use dioxus::prelude::manganis;
    use dioxus_core::VirtualDom;
    use dioxus_core_macro::rsx;
    use es_fluent_manager_core::ModuleData;
    use unic_langid::{LanguageIdentifier, langid};

    static SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en"), langid!("fr")];
    static MODULE_DATA: ModuleData = ModuleData {
        name: "asset-test",
        domain: "asset-test",
        supported_languages: SUPPORTED_LANGUAGES,
        namespaces: &[],
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
            localize: &mut dyn for<'a> FnMut(
                es_fluent::registry::StaticFluentDomain,
                es_fluent::registry::StaticFluentEntryId,
                Option<&es_fluent::FluentArgs<'a>>,
            ) -> String,
        ) -> String {
            localize(
                es_fluent::registry::StaticFluentDomain::new_unchecked("asset-test"),
                es_fluent::registry::StaticFluentEntryId::new_unchecked("asset-hello"),
                None,
            )
        }
    }

    #[allow(non_snake_case)]
    fn SsrMessage() -> Element {
        rsx! { "SSR" }
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
            i18n.localize("asset-hello", None),
            Some("Hello from asset".to_string())
        );
        assert_eq!(i18n.localize_message(&TestMessage), "Hello from asset");

        i18n.select_language(langid!("fr"))
            .expect("SSR request should switch language");
        assert_eq!(i18n.requested_language(), langid!("fr"));
        assert_eq!(
            i18n.localize_in_domain("asset-test", "asset-hello", None),
            Some("Bonjour from asset".to_string())
        );

        i18n.select_language_strict(langid!("en"))
            .expect("strict SSR language switch should work");
        let mut looked_up = None;
        i18n.with_lookup(&mut |lookup| {
            looked_up = lookup("asset-test", "asset-hello", None);
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
    fn ssr_render_helpers_delegate_to_dioxus_ssr() {
        let i18n = runtime()
            .request_blocking(langid!("en"))
            .expect("SSR request should load assets");
        let mut dom = VirtualDom::new(SsrMessage);

        assert!(i18n.rebuild_and_render(&mut dom).contains("SSR"));
        assert!(i18n.render(&dom).contains("SSR"));
        assert!(i18n.pre_render(&dom).contains("SSR"));

        let mut renderer = Renderer::new();
        assert!(i18n.render_with(&mut renderer, &dom).contains("SSR"));
        assert!(i18n.rebuild_and_pre_render(&mut dom).contains("SSR"));
        assert!(i18n.render_element(rsx! { "Element" }).contains("Element"));
    }
}

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

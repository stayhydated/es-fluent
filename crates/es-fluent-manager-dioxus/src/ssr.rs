use crate::{DioxusInitError, ManagedI18n, ModuleDiscoveryErrors};
use dioxus_core::{Element, VirtualDom};
use dioxus_ssr::Renderer;
use es_fluent::{FluentLocalizer, FluentMessage, FluentValue};
use es_fluent_manager_core::{
    DiscoveredRuntimeI18nModules, FluentManager, LanguageSelectionPolicy, LocalizationError,
};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use unic_langid::LanguageIdentifier;

/// SSR localization runtime with cached module discovery.
///
/// Construct this once during process startup, then create one [`SsrI18n`] per
/// request. The request object owns its own manager state, so locale selection is
/// isolated without relying on context-free localization hooks.
#[derive(Clone, Debug, Default)]
pub struct SsrI18nRuntime {
    modules: Arc<OnceLock<Result<DiscoveredRuntimeI18nModules, ModuleDiscoveryErrors>>>,
}

impl SsrI18nRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request<L: Into<LanguageIdentifier>>(
        &self,
        language: L,
    ) -> Result<SsrI18n, DioxusInitError> {
        self.request_with_policy(language, LanguageSelectionPolicy::BestEffort)
    }

    pub fn request_strict<L: Into<LanguageIdentifier>>(
        &self,
        language: L,
    ) -> Result<SsrI18n, DioxusInitError> {
        SsrI18n::new_with_cached_modules_strict(cached_discovered_modules(&self.modules)?, language)
    }

    pub fn request_with_policy<L: Into<LanguageIdentifier>>(
        &self,
        language: L,
        selection_policy: LanguageSelectionPolicy,
    ) -> Result<SsrI18n, DioxusInitError> {
        SsrI18n::new_with_cached_modules_with_policy(
            cached_discovered_modules(&self.modules)?,
            language,
            selection_policy,
        )
    }
}

/// Request-scoped Dioxus SSR localization state.
#[derive(Clone, Eq, PartialEq)]
pub struct SsrI18n {
    managed: ManagedI18n,
}

impl SsrI18n {
    pub(crate) fn new_with_cached_modules_strict<L: Into<LanguageIdentifier>>(
        modules: &DiscoveredRuntimeI18nModules,
        lang: L,
    ) -> Result<Self, DioxusInitError> {
        let managed = ManagedI18n::new_with_cached_modules_strict(modules, lang)?;
        Ok(Self { managed })
    }

    pub(crate) fn new_with_cached_modules_with_policy<L: Into<LanguageIdentifier>>(
        modules: &DiscoveredRuntimeI18nModules,
        lang: L,
        selection_policy: LanguageSelectionPolicy,
    ) -> Result<Self, DioxusInitError> {
        let managed =
            ManagedI18n::new_with_cached_modules_with_policy(modules, lang, selection_policy)?;
        Ok(Self { managed })
    }

    pub fn requested_language(&self) -> LanguageIdentifier {
        self.managed.requested_language()
    }

    pub fn select_language<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        self.managed.select_language(lang)
    }

    pub fn select_language_strict<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        self.managed.select_language_strict(lang)
    }

    pub fn localize_message<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        self.managed.localize_message(message)
    }

    #[cfg(feature = "client")]
    pub fn provide_context(&self) -> Result<crate::DioxusI18n, DioxusInitError> {
        crate::use_provide_i18n(self.managed.clone())
    }

    /// Rebuilds and renders a Dioxus virtual DOM.
    ///
    /// This helper does not install i18n context automatically; pass
    /// `SsrI18n` as a prop or call `SsrI18n::provide_context` from a
    /// component when using hook-based lookup.
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
        FluentLocalizer::localize(&self.managed, id, args)
    }

    fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        FluentLocalizer::localize_in_domain(&self.managed, domain, id, args)
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
        FluentLocalizer::with_lookup(&self.managed, f);
    }
}

fn cached_discovered_modules(
    modules: &OnceLock<Result<DiscoveredRuntimeI18nModules, ModuleDiscoveryErrors>>,
) -> Result<&DiscoveredRuntimeI18nModules, DioxusInitError> {
    let modules = modules.get_or_init(|| {
        FluentManager::try_discover_runtime_modules().map_err(ModuleDiscoveryErrors::from)
    });

    match modules {
        Ok(modules) => Ok(modules),
        Err(errors) => Err(DioxusInitError::ModuleDiscovery(errors.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use es_fluent_manager_core::{ModuleDiscoveryError, ModuleRegistrationKind};

    #[test]
    fn cached_discovered_modules_returns_cached_discovery_errors() {
        let modules = OnceLock::new();
        modules
            .set(Err(ModuleDiscoveryErrors::from(vec![
                ModuleDiscoveryError::DuplicateModuleRegistration {
                    name: "app".to_string(),
                    domain: "app".to_string(),
                    kind: ModuleRegistrationKind::RuntimeLocalizer,
                    count: 2,
                },
            ])))
            .expect("test should set cache once");

        let err = cached_discovered_modules(&modules)
            .expect_err("cached discovery errors should be returned");

        match err {
            DioxusInitError::ModuleDiscovery(errors) => {
                assert_eq!(errors.as_slice().len(), 1);
            },
            other => panic!("expected module discovery error, got {other:?}"),
        }
    }
}

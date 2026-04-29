use crate::{DioxusInitError, ManagedI18n, ModuleDiscoveryErrors};
use dioxus_core::{Element, VirtualDom};
use dioxus_ssr::Renderer;
use es_fluent::{FluentMessage, FluentValue};
use es_fluent_manager_core::{DiscoveredRuntimeI18nModules, FluentManager, LocalizationError};
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
        SsrI18n::new_with_cached_modules(cached_discovered_modules(&self.modules)?, language)
    }
}

/// Request-scoped Dioxus SSR localization state.
pub struct SsrI18n {
    managed: ManagedI18n,
}

impl SsrI18n {
    pub(crate) fn new_with_cached_modules<L: Into<LanguageIdentifier>>(
        modules: &DiscoveredRuntimeI18nModules,
        lang: L,
    ) -> Result<Self, DioxusInitError> {
        let managed = ManagedI18n::new_with_cached_modules(modules, lang)?;
        Ok(Self { managed })
    }

    pub fn managed(&self) -> &ManagedI18n {
        &self.managed
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

    pub fn localize<'a>(
        &self,
        id: impl AsRef<str>,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        self.managed.localize(id, args)
    }

    pub fn localize_in_domain<'a>(
        &self,
        domain: impl AsRef<str>,
        id: impl AsRef<str>,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        self.managed.localize_in_domain(domain, id, args)
    }

    pub fn localize_message<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        self.managed.localize_message(message)
    }

    pub fn localize_message_silent<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        self.managed.localize_message_silent(message)
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

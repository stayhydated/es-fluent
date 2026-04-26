use crate::{DioxusGlobalLocalizerError, DioxusInitError, ManagedI18n, bridge::install_ssr_bridge};
use dioxus_core::{Element, VirtualDom};
use dioxus_ssr::Renderer;
use es_fluent::{CustomLocalizerLookup, FluentValue, GlobalLocalizationError};
use es_fluent_manager_core::FluentManager;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

thread_local! {
    static CURRENT_MANAGER_STACK: RefCell<Vec<Arc<FluentManager>>> = const { RefCell::new(Vec::new()) };
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SsrI18nRuntime;

impl SsrI18nRuntime {
    pub fn install() -> Result<Self, DioxusGlobalLocalizerError> {
        install_process_global_bridge()?;
        Ok(Self)
    }

    pub fn request<L: Into<LanguageIdentifier>>(
        &self,
        language: L,
    ) -> Result<SsrI18n, DioxusInitError> {
        install_process_global_bridge().map_err(DioxusInitError::global_localizer)?;
        SsrI18n::new_with_discovered_modules(language)
    }
}

/// Request-scoped Dioxus SSR localization state.
///
/// Construct this through [`SsrI18nRuntime::request`] after installing the SSR
/// runtime once during process startup. `SsrI18n` is synchronous by design. Do
/// do not hold [`SsrI18n::with_sync_thread_local_manager`] scopes across `.await`,
/// spawned tasks, streaming callbacks, or higher-level fullstack server
/// boundaries.
pub struct SsrI18n {
    managed: ManagedI18n,
}

impl SsrI18n {
    pub(crate) fn new_with_discovered_modules<L: Into<LanguageIdentifier>>(
        lang: L,
    ) -> Result<Self, DioxusInitError> {
        let managed = ManagedI18n::new_with_discovered_modules(lang)?;
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
    ) -> Result<(), GlobalLocalizationError> {
        self.managed.select_language(lang)
    }

    pub fn select_language_strict<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), GlobalLocalizationError> {
        self.managed.select_language_strict(lang)
    }

    /// Runs a synchronous callback while this request's manager is installed.
    pub fn with_sync_thread_local_manager<R>(
        &self,
        f: impl FnOnce() -> R,
    ) -> Result<R, DioxusGlobalLocalizerError> {
        install_process_global_bridge()?;
        let _scope = CurrentManagerScope::new(Arc::clone(self.managed.manager()));
        Ok(f())
    }

    /// Rebuilds the virtual DOM and serializes it while this request's manager
    /// is installed.
    pub fn rebuild_and_render(
        &self,
        dom: &mut VirtualDom,
    ) -> Result<String, DioxusGlobalLocalizerError> {
        self.with_sync_thread_local_manager(|| {
            dom.rebuild_in_place();
            dioxus_ssr::render(dom)
        })
    }

    /// Rebuilds the virtual DOM and pre-renders it while this request's manager
    /// is installed.
    pub fn rebuild_and_pre_render(
        &self,
        dom: &mut VirtualDom,
    ) -> Result<String, DioxusGlobalLocalizerError> {
        self.with_sync_thread_local_manager(|| {
            dom.rebuild_in_place();
            dioxus_ssr::pre_render(dom)
        })
    }

    /// Serializes an already rebuilt virtual DOM while this request's manager is
    /// installed.
    pub fn render(&self, dom: &VirtualDom) -> Result<String, DioxusGlobalLocalizerError> {
        self.with_sync_thread_local_manager(|| dioxus_ssr::render(dom))
    }

    /// Pre-renders an already rebuilt virtual DOM while this request's manager
    /// is installed.
    pub fn pre_render(&self, dom: &VirtualDom) -> Result<String, DioxusGlobalLocalizerError> {
        self.with_sync_thread_local_manager(|| dioxus_ssr::pre_render(dom))
    }

    pub fn render_with(
        &self,
        renderer: &mut Renderer,
        dom: &VirtualDom,
    ) -> Result<String, DioxusGlobalLocalizerError> {
        self.with_sync_thread_local_manager(|| renderer.render(dom))
    }

    pub fn render_element(&self, element: Element) -> Result<String, DioxusGlobalLocalizerError> {
        self.with_sync_thread_local_manager(|| dioxus_ssr::render_element(element))
    }
}

struct CurrentManagerScope {
    manager: Arc<FluentManager>,
}

impl CurrentManagerScope {
    fn new(manager: Arc<FluentManager>) -> Self {
        CURRENT_MANAGER_STACK.with(|stack| stack.borrow_mut().push(Arc::clone(&manager)));
        Self { manager }
    }
}

impl Drop for CurrentManagerScope {
    fn drop(&mut self) {
        CURRENT_MANAGER_STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            let popped = stack.pop();

            if !matches!(popped, Some(manager) if Arc::ptr_eq(&manager, &self.manager)) {
                tracing::error!("SSR i18n manager stack corruption detected");
                stack.clear();
            }
        });
    }
}

fn install_process_global_bridge() -> Result<(), DioxusGlobalLocalizerError> {
    install_ssr_bridge()
}

pub(crate) fn localize_current_ssr_manager<'a>(
    domain: Option<&str>,
    id: &str,
    args: Option<&HashMap<&str, FluentValue<'a>>>,
) -> CustomLocalizerLookup {
    CURRENT_MANAGER_STACK.with(|stack| {
        let manager = match stack.borrow().last() {
            Some(manager) => Arc::clone(manager),
            None => {
                match domain {
                    Some(domain) => tracing::error!(
                        domain,
                        message_id = id,
                        "SSR Fluent localization used outside an SsrI18n scope"
                    ),
                    None => tracing::error!(
                        message_id = id,
                        "SSR Fluent localization used outside an SsrI18n scope"
                    ),
                }
                return CustomLocalizerLookup::Missing;
            },
        };

        let message = match domain {
            Some(domain) => manager.localize_in_domain(domain, id, args),
            None => manager.localize(id, args),
        };

        match message {
            Some(message) => CustomLocalizerLookup::Found(message),
            None => {
                match domain {
                    Some(domain) => {
                        tracing::warn!(domain, message_id = id, "missing Fluent message")
                    },
                    None => tracing::warn!(message_id = id, "missing Fluent message"),
                }
                CustomLocalizerLookup::Missing
            },
        }
    })
}

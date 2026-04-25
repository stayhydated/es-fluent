use crate::{
    DioxusGlobalLocalizerError, DioxusInitError, GlobalLocalizerMode, ManagedI18n,
    bridge::install_ssr_bridge,
};
use dioxus_core::{Element, VirtualDom};
use dioxus_ssr::Renderer;
use es_fluent::{FluentValue, GlobalLocalizationError};
use es_fluent_manager_core::FluentManager;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

thread_local! {
    static CURRENT_MANAGER_STACK: RefCell<Vec<Arc<FluentManager>>> = const { RefCell::new(Vec::new()) };
}

pub struct SsrI18n {
    managed: ManagedI18n,
}

impl SsrI18n {
    pub fn new_with_discovered_modules<L: Into<LanguageIdentifier>>(lang: L) -> Self {
        Self::try_new_with_discovered_modules(lang)
            .unwrap_or_else(|error| panic!("failed to initialize Dioxus SSR i18n manager: {error}"))
    }

    pub fn try_new_with_discovered_modules<L: Into<LanguageIdentifier>>(
        lang: L,
    ) -> Result<Self, DioxusInitError> {
        Self::try_new_with_discovered_modules_and_mode(lang, GlobalLocalizerMode::ErrorIfAlreadySet)
    }

    pub fn try_new_with_discovered_modules_and_mode<L: Into<LanguageIdentifier>>(
        lang: L,
        mode: GlobalLocalizerMode,
    ) -> Result<Self, DioxusInitError> {
        let managed = ManagedI18n::try_new_with_discovered_modules(lang)?;
        install_global_localizer(mode).map_err(DioxusInitError::global_localizer)?;

        Ok(Self { managed })
    }

    pub fn install_global_localizer(
        mode: GlobalLocalizerMode,
    ) -> Result<(), DioxusGlobalLocalizerError> {
        install_global_localizer(mode)
    }

    pub fn managed(&self) -> &ManagedI18n {
        &self.managed
    }

    pub fn active_language(&self) -> LanguageIdentifier {
        self.managed.active_language()
    }

    pub fn requested_language(&self) -> LanguageIdentifier {
        self.active_language()
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

    pub fn with_manager<R>(&self, f: impl FnOnce() -> R) -> R {
        let _scope = CurrentManagerScope::new(self.managed.manager());
        f()
    }

    /// Rebuilds the virtual DOM and serializes it while this request's manager
    /// is installed.
    ///
    /// Use this for the common SSR path. Components that call
    /// `to_fluent_string()` usually localize during the Dioxus rebuild pass, so
    /// both rebuilding and rendering need the request-scoped manager.
    pub fn rebuild_and_render(&self, dom: &mut VirtualDom) -> String {
        self.with_manager(|| {
            dom.rebuild_in_place();
            dioxus_ssr::render(dom)
        })
    }

    /// Rebuilds the virtual DOM and pre-renders it while this request's manager
    /// is installed.
    pub fn rebuild_and_pre_render(&self, dom: &mut VirtualDom) -> String {
        self.with_manager(|| {
            dom.rebuild_in_place();
            dioxus_ssr::pre_render(dom)
        })
    }

    /// Serializes an already rebuilt virtual DOM while this request's manager is
    /// installed.
    ///
    /// If localization happens during the rebuild pass, call
    /// [`Self::rebuild_and_render`] or rebuild inside [`Self::with_manager`]
    /// before using this lower-level method.
    pub fn render(&self, dom: &VirtualDom) -> String {
        self.with_manager(|| dioxus_ssr::render(dom))
    }

    /// Pre-renders an already rebuilt virtual DOM while this request's manager
    /// is installed.
    pub fn pre_render(&self, dom: &VirtualDom) -> String {
        self.with_manager(|| dioxus_ssr::pre_render(dom))
    }

    pub fn render_with(&self, renderer: &mut Renderer, dom: &VirtualDom) -> String {
        self.with_manager(|| renderer.render(dom))
    }

    pub fn render_element(&self, element: Element) -> String {
        self.with_manager(|| dioxus_ssr::render_element(element))
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
            let popped = stack.borrow_mut().pop();
            debug_assert!(popped.is_some(), "SSR manager stack underflow");
            if let Some(popped) = popped {
                debug_assert!(
                    Arc::ptr_eq(&popped, &self.manager),
                    "SSR manager stack popped a different manager than this scope pushed"
                );
            }
        });
    }
}

pub fn install_global_localizer(
    mode: GlobalLocalizerMode,
) -> Result<(), DioxusGlobalLocalizerError> {
    install_ssr_bridge(
        mode,
        move |domain: Option<&str>, id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>| {
            CURRENT_MANAGER_STACK.with(|stack| {
                let manager = Arc::clone(stack.borrow().last()?);
                let message = match domain {
                    Some(domain) => manager.localize_in_domain(domain, id, args),
                    None => manager.localize(id, args),
                };

                match message {
                    Some(message) => Some(message),
                    None => {
                        match domain {
                            Some(domain) => {
                                tracing::warn!(domain, message_id = id, "missing Fluent message");
                            },
                            None => {
                                tracing::warn!(message_id = id, "missing Fluent message");
                            },
                        }
                        Some(id.to_string())
                    },
                }
            })
        },
    )
}

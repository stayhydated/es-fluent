use crate::{
    DioxusGlobalLocalizerError, DioxusInitError, GlobalBridgePolicy, ManagedI18n,
    bridge::{install_ssr_bridge, install_ssr_bridge_scoped},
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

/// Request-scoped Dioxus SSR localization state.
///
/// `SsrI18n` is synchronous by design. Do not hold
/// [`SsrI18n::with_sync_thread_local_manager`] scopes across `.await`, spawned
/// tasks, streaming callbacks, or higher-level fullstack server boundaries. The
/// active manager is stored in thread-local state and is only valid for the
/// synchronous render call that owns the scope.
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
        Self::try_new_with_discovered_modules_and_policy(lang, GlobalBridgePolicy::InstallOnce)
    }

    pub fn try_new_with_discovered_modules_and_policy<L: Into<LanguageIdentifier>>(
        lang: L,
        policy: GlobalBridgePolicy,
    ) -> Result<Self, DioxusInitError> {
        let managed = ManagedI18n::try_new_with_discovered_modules(lang)?;
        install_process_global_bridge(policy).map_err(DioxusInitError::global_localizer)?;

        Ok(Self { managed })
    }

    pub fn install_process_global_bridge(
        policy: GlobalBridgePolicy,
    ) -> Result<(), DioxusGlobalLocalizerError> {
        self::install_process_global_bridge(policy)
    }

    pub fn install_process_global_bridge_scoped(
        policy: GlobalBridgePolicy,
    ) -> Result<crate::DioxusGlobalBridgeGuard, DioxusGlobalLocalizerError> {
        self::install_process_global_bridge_scoped(policy)
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
    ///
    /// Do not keep this scope alive across `.await`, spawned tasks, streaming
    /// callbacks, or fullstack server boundaries. The manager stack is
    /// thread-local and is only safe for synchronous SSR work.
    pub fn with_sync_thread_local_manager<R>(&self, f: impl FnOnce() -> R) -> R {
        let _scope = CurrentManagerScope::new(self.managed.raw_manager_untracked());
        f()
    }

    /// Rebuilds the virtual DOM and serializes it while this request's manager
    /// is installed.
    ///
    /// Use this for the common SSR path. Components that call
    /// `to_fluent_string()` usually localize during the Dioxus rebuild pass, so
    /// both rebuilding and rendering need the request-scoped manager.
    pub fn rebuild_and_render(&self, dom: &mut VirtualDom) -> String {
        self.with_sync_thread_local_manager(|| {
            dom.rebuild_in_place();
            dioxus_ssr::render(dom)
        })
    }

    /// Rebuilds the virtual DOM and pre-renders it while this request's manager
    /// is installed.
    pub fn rebuild_and_pre_render(&self, dom: &mut VirtualDom) -> String {
        self.with_sync_thread_local_manager(|| {
            dom.rebuild_in_place();
            dioxus_ssr::pre_render(dom)
        })
    }

    /// Serializes an already rebuilt virtual DOM while this request's manager is
    /// installed.
    ///
    /// If localization happens during the rebuild pass, call
    /// [`Self::rebuild_and_render`] or rebuild inside
    /// [`Self::with_sync_thread_local_manager`]
    /// before using this lower-level method.
    pub fn render(&self, dom: &VirtualDom) -> String {
        self.with_sync_thread_local_manager(|| dioxus_ssr::render(dom))
    }

    /// Pre-renders an already rebuilt virtual DOM while this request's manager
    /// is installed.
    pub fn pre_render(&self, dom: &VirtualDom) -> String {
        self.with_sync_thread_local_manager(|| dioxus_ssr::pre_render(dom))
    }

    pub fn render_with(&self, renderer: &mut Renderer, dom: &VirtualDom) -> String {
        self.with_sync_thread_local_manager(|| renderer.render(dom))
    }

    pub fn render_element(&self, element: Element) -> String {
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

pub fn install_process_global_bridge(
    policy: GlobalBridgePolicy,
) -> Result<(), DioxusGlobalLocalizerError> {
    install_ssr_bridge(
        policy,
        move |domain: Option<&str>, id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>| {
            CURRENT_MANAGER_STACK.with(|stack| {
                let manager = match stack.borrow().last() {
                    Some(manager) => Arc::clone(manager),
                    None => {
                        match domain {
                            Some(domain) => {
                                tracing::error!(
                                    domain,
                                    message_id = id,
                                    "SSR Fluent localization used outside an SsrI18n scope"
                                );
                            },
                            None => {
                                tracing::error!(
                                    message_id = id,
                                    "SSR Fluent localization used outside an SsrI18n scope"
                                );
                            },
                        }
                        return Some(id.to_string());
                    },
                };
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

pub fn install_process_global_bridge_scoped(
    policy: GlobalBridgePolicy,
) -> Result<crate::DioxusGlobalBridgeGuard, DioxusGlobalLocalizerError> {
    install_ssr_bridge_scoped(
        policy,
        move |domain: Option<&str>, id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>| {
            CURRENT_MANAGER_STACK.with(|stack| {
                let manager = match stack.borrow().last() {
                    Some(manager) => Arc::clone(manager),
                    None => {
                        match domain {
                            Some(domain) => {
                                tracing::error!(
                                    domain,
                                    message_id = id,
                                    "SSR Fluent localization used outside an SsrI18n scope"
                                );
                            },
                            None => {
                                tracing::error!(
                                    message_id = id,
                                    "SSR Fluent localization used outside an SsrI18n scope"
                                );
                            },
                        }
                        return Some(id.to_string());
                    },
                };
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

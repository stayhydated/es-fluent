use crate::{DioxusInitError, GlobalLocalizerMode, ManagedI18n};
use dioxus_core::{Element, VirtualDom};
use dioxus_ssr::Renderer;
use es_fluent::{
    FluentValue, GlobalLocalizationError, replace_custom_localizer_with_domain,
    try_set_custom_localizer_with_domain,
};
use es_fluent_manager_core::FluentManager;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use unic_langid::LanguageIdentifier;

thread_local! {
    static CURRENT_MANAGER_STACK: RefCell<Vec<Arc<FluentManager>>> = const { RefCell::new(Vec::new()) };
}

static THREAD_LOCAL_BRIDGE_INSTALLED: AtomicBool = AtomicBool::new(false);
static THREAD_LOCAL_BRIDGE_INSTALL_LOCK: Mutex<()> = Mutex::new(());

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
        install_global_localizer(mode).map_err(DioxusInitError::GlobalLocalizer)?;

        Ok(Self { managed })
    }

    pub fn install_global_localizer(
        mode: GlobalLocalizerMode,
    ) -> Result<(), GlobalLocalizationError> {
        install_global_localizer(mode)
    }

    pub fn managed(&self) -> &ManagedI18n {
        &self.managed
    }

    pub fn active_language(&self) -> LanguageIdentifier {
        self.managed.active_language()
    }

    pub fn select_language<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), GlobalLocalizationError> {
        self.managed.select_language(lang)
    }

    pub fn with_manager<R>(&self, f: impl FnOnce() -> R) -> R {
        let _scope = CurrentManagerScope::new(self.managed.manager());
        f()
    }

    pub fn render(&self, dom: &VirtualDom) -> String {
        self.with_manager(|| dioxus_ssr::render(dom))
    }

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

pub fn install_global_localizer(mode: GlobalLocalizerMode) -> Result<(), GlobalLocalizationError> {
    if THREAD_LOCAL_BRIDGE_INSTALLED.load(Ordering::Acquire) {
        return Ok(());
    }

    let _guard = THREAD_LOCAL_BRIDGE_INSTALL_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if THREAD_LOCAL_BRIDGE_INSTALLED.load(Ordering::Acquire) {
        return Ok(());
    }

    install_thread_local_bridge(mode)?;
    THREAD_LOCAL_BRIDGE_INSTALLED.store(true, Ordering::Release);
    Ok(())
}

fn install_thread_local_bridge(mode: GlobalLocalizerMode) -> Result<(), GlobalLocalizationError> {
    let bridge =
        move |domain: Option<&str>, id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>| {
            CURRENT_MANAGER_STACK.with(|stack| {
                stack.borrow().last().and_then(|manager| match domain {
                    Some(domain) => manager.localize_in_domain(domain, id, args),
                    None => manager.localize(id, args),
                })
            })
        };

    match mode {
        GlobalLocalizerMode::ErrorIfAlreadySet => try_set_custom_localizer_with_domain(bridge),
        GlobalLocalizerMode::ReplaceExisting => {
            tracing::debug!(
                "replacing the process-global Fluent custom localizer with the Dioxus SSR bridge"
            );
            replace_custom_localizer_with_domain(bridge);
            Ok(())
        },
    }
}

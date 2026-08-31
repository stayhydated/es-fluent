#[cfg(all(target_arch = "wasm32", debug_assertions))]
use super::hot_reload::dioxus_i18n_hot_reload_message_matches;
use super::{
    error::DioxusAssetLoadError, localizer::DioxusAssetI18n, module::DioxusI18nAssetModules,
};

use dioxus_core::{Element, VNode};
use dioxus_core_macro::{Props, component, rsx};
use dioxus_signals::{ReadableExt as _, Signal, WritableExt as _};

use es_fluent::{
    FluentArgs, FluentLocalizer, FluentLocalizerLookup, FluentMessage,
    registry::StaticFluentMessageKey,
};
use es_fluent_manager_core::{LanguageSelectionPolicy, LocalizationError};

#[cfg(all(target_arch = "wasm32", debug_assertions))]
use std::sync::Arc;
use unic_langid::LanguageIdentifier;
#[cfg(all(target_arch = "wasm32", debug_assertions))]
use wasm_bindgen::{JsCast as _, closure::Closure};

#[cfg(feature = "client")]
#[derive(Clone)]
pub enum DioxusAssetI18nLoadState {
    Loading,
    Ready(DioxusAssetI18n),
    Failed(DioxusAssetLoadError),
}

#[cfg(feature = "client")]
#[derive(Clone)]
struct DioxusAssetI18nLoadConfig {
    modules: DioxusI18nAssetModules,
    initial_language: LanguageIdentifier,
    selection_policy: LanguageSelectionPolicy,
}

#[cfg(feature = "client")]
fn use_dioxus_i18n_asset_reload_revision(modules: DioxusI18nAssetModules) -> Signal<u64> {
    let revision = dioxus_hooks::use_signal(|| 0_u64);

    #[cfg(all(target_arch = "wasm32", debug_assertions))]
    {
        let watched_assets =
            dioxus_core::use_hook(move || watched_dioxus_i18n_asset_paths(modules));
        let revision_for_listener = revision;
        let _listener = dioxus_core::use_hook(move || {
            start_dioxus_i18n_asset_hot_reload_listener(
                watched_assets.clone(),
                revision_for_listener,
            )
            .map(std::rc::Rc::new)
        });
    }

    #[cfg(not(all(target_arch = "wasm32", debug_assertions)))]
    {
        let _ = modules;
    }

    revision
}

#[cfg(all(feature = "client", target_arch = "wasm32", debug_assertions))]
struct DioxusAssetHotReloadListener {
    _websocket: web_sys::WebSocket,
    _onmessage: Closure<dyn FnMut(web_sys::MessageEvent)>,
}

#[cfg(all(feature = "client", target_arch = "wasm32", debug_assertions))]
fn start_dioxus_i18n_asset_hot_reload_listener(
    watched_assets: Arc<[String]>,
    mut revision: Signal<u64>,
) -> Option<DioxusAssetHotReloadListener> {
    if watched_assets.is_empty() {
        return None;
    }

    let window = web_sys::window()?;
    let location = window.location();
    let protocol = match location.protocol().ok().as_deref() {
        Some("https:") => "wss:",
        _ => "ws:",
    };
    let host = location.host().ok()?;
    let websocket = web_sys::WebSocket::new(&format!("{protocol}//{host}/_dioxus")).ok()?;
    let onmessage =
        Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
            let Some(message) = event.data().as_string() else {
                return;
            };

            if !dioxus_i18n_hot_reload_message_matches(&message, &watched_assets) {
                return;
            }

            let mut revision = revision.write();
            *revision = revision.wrapping_add(1);
        });

    websocket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

    Some(DioxusAssetHotReloadListener {
        _websocket: websocket,
        _onmessage: onmessage,
    })
}

#[cfg(all(feature = "client", target_arch = "wasm32", debug_assertions))]
fn watched_dioxus_i18n_asset_paths(modules: DioxusI18nAssetModules) -> Arc<[String]> {
    modules
        .as_slice()
        .iter()
        .flat_map(|module| module.resources.iter())
        .map(|resource| resource.asset.bundled().bundled_path().to_string())
        .collect::<Vec<_>>()
        .into()
}

#[cfg(feature = "client")]
pub fn use_init_asset_i18n_modules<L>(
    modules: DioxusI18nAssetModules,
    initial_language: L,
    selection_policy: LanguageSelectionPolicy,
) -> DioxusAssetI18nLoadState
where
    L: Into<LanguageIdentifier> + 'static,
{
    let initial_language = initial_language.into();
    let config = dioxus_core::use_hook(move || DioxusAssetI18nLoadConfig {
        modules,
        initial_language,
        selection_policy,
    });
    let reload_revision = use_dioxus_i18n_asset_reload_revision(config.modules);
    let resource = dioxus_hooks::use_resource(move || {
        let config = config.clone();
        let reload_revision = *reload_revision.read();
        async move {
            DioxusAssetI18n::load_modules_with_cache_bust(
                config.modules,
                config.initial_language.clone(),
                config.selection_policy,
                (reload_revision != 0).then_some(reload_revision),
            )
            .await
        }
    });

    match resource.read_unchecked().as_ref() {
        Some(Ok(i18n)) => DioxusAssetI18nLoadState::Ready(i18n.clone()),
        Some(Err(error)) => DioxusAssetI18nLoadState::Failed(error.clone()),
        None => DioxusAssetI18nLoadState::Loading,
    }
}

#[cfg(feature = "client")]
pub fn use_init_asset_i18n<L>(
    initial_language: L,
    selection_policy: LanguageSelectionPolicy,
) -> DioxusAssetI18nLoadState
where
    L: Into<LanguageIdentifier> + 'static,
{
    use_init_asset_i18n_modules(
        DioxusI18nAssetModules::discovered(),
        initial_language,
        selection_policy,
    )
}

#[cfg(feature = "client")]
#[derive(Clone)]
struct DioxusAssetI18nContext {
    i18n: Signal<DioxusAssetI18n>,
    tracked: Signal<LanguageIdentifier>,
    selection_policy: Signal<LanguageSelectionPolicy>,
}

#[cfg(feature = "client")]
impl DioxusAssetI18nContext {
    fn i18n(&self) -> DioxusAssetI18n {
        self.i18n.read().clone()
    }

    fn current(&self) -> LanguageIdentifier {
        self.tracked.read().clone()
    }

    fn peek(&self) -> LanguageIdentifier {
        self.tracked.peek().clone()
    }

    fn update(&self, value: LanguageIdentifier) {
        let mut tracked = self.tracked;
        *tracked.write() = value;
    }

    fn update_selection_policy(&self, selection_policy: LanguageSelectionPolicy) {
        if *self.selection_policy.peek() == selection_policy {
            return;
        }

        let mut current = self.selection_policy;
        *current.write() = selection_policy;
    }

    fn replace_i18n(&self, i18n: DioxusAssetI18n) {
        let unchanged = { self.i18n.peek().eq(&i18n) };
        if unchanged {
            return;
        }

        let requested_language = self.peek();
        if i18n.requested_language() != requested_language
            && let Err(error) = i18n.select_language_with_policy(
                requested_language.clone(),
                *self.selection_policy.peek(),
            )
        {
            tracing::warn!(
                "Reloaded Dioxus asset i18n could not preserve requested locale '{}': {}",
                requested_language,
                error
            );
        }

        let selected_language = i18n.requested_language();
        let mut current = self.i18n;
        *current.write() = i18n;

        if selected_language != requested_language {
            self.update(selected_language);
        }
    }
}

#[cfg(feature = "client")]
#[derive(Clone)]
pub struct DioxusAssetI18nHandle {
    context: DioxusAssetI18nContext,
}

#[cfg(feature = "client")]
impl DioxusAssetI18nHandle {
    pub fn requested_language(&self) -> LanguageIdentifier {
        self.context.current()
    }

    pub fn peek_requested_language(&self) -> LanguageIdentifier {
        self.context.peek()
    }

    pub fn select_language<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        let i18n = self.context.i18n();
        i18n.select_language(lang)?;
        self.context.update(i18n.requested_language());
        Ok(())
    }

    pub fn select_language_strict<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        let i18n = self.context.i18n();
        i18n.select_language_strict(lang)?;
        self.context.update(i18n.requested_language());
        Ok(())
    }

    pub fn localize_message<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        let _ = self.context.current();
        self.context.i18n().localize_message(message)
    }
}

#[cfg(feature = "client")]
impl FluentLocalizer for DioxusAssetI18nHandle {
    fn localize<'a>(
        &self,
        key: StaticFluentMessageKey,
        args: Option<&FluentArgs<'a>>,
    ) -> Option<String> {
        let _ = self.context.current();
        let i18n = self.context.i18n();
        FluentLocalizer::localize(&i18n, key, args)
    }

    fn with_lookup(&self, f: &mut dyn FnMut(&mut FluentLocalizerLookup<'_>)) {
        let _ = self.context.current();
        let i18n = self.context.i18n();
        FluentLocalizer::with_lookup(&i18n, f);
    }
}

#[cfg(feature = "client")]
pub fn use_provide_asset_i18n(i18n: DioxusAssetI18n) -> DioxusAssetI18nHandle {
    use_provide_asset_i18n_with_policy(i18n, LanguageSelectionPolicy::BestEffort)
}

#[cfg(feature = "client")]
fn use_provide_asset_i18n_with_policy(
    i18n: DioxusAssetI18n,
    selection_policy: LanguageSelectionPolicy,
) -> DioxusAssetI18nHandle {
    let fallback_language = i18n.requested_language();
    let initial_i18n = i18n.clone();
    let context = dioxus_hooks::use_context_provider(move || DioxusAssetI18nContext {
        tracked: Signal::new(fallback_language),
        i18n: Signal::new(initial_i18n),
        selection_policy: Signal::new(selection_policy),
    });
    context.update_selection_policy(selection_policy);
    context.replace_i18n(i18n);
    DioxusAssetI18nHandle { context }
}

#[cfg(feature = "client")]
pub fn try_use_i18n() -> Option<DioxusAssetI18nHandle> {
    dioxus_hooks::try_use_context::<DioxusAssetI18nContext>()
        .map(|context| DioxusAssetI18nHandle { context })
}

#[cfg(feature = "client")]
pub fn use_i18n() -> Result<DioxusAssetI18nHandle, crate::DioxusAssetI18nContextError> {
    try_use_i18n().ok_or(crate::DioxusAssetI18nContextError::MissingContext)
}

#[cfg(feature = "client")]
pub fn try_consume_asset_i18n() -> Option<DioxusAssetI18nHandle> {
    dioxus_core::try_consume_context::<DioxusAssetI18nContext>()
        .map(|context| DioxusAssetI18nHandle { context })
}

#[cfg(feature = "client")]
pub fn consume_asset_i18n() -> Result<DioxusAssetI18nHandle, crate::DioxusAssetI18nContextError> {
    try_consume_asset_i18n().ok_or(crate::DioxusAssetI18nContextError::MissingContext)
}

#[cfg(feature = "client")]
#[allow(non_snake_case)]
#[component]
pub fn DioxusAssetI18nProvider(
    #[props(default = DioxusI18nAssetModules::discovered())] modules: DioxusI18nAssetModules,
    initial_language: LanguageIdentifier,
    #[props(default = LanguageSelectionPolicy::BestEffort)]
    selection_policy: LanguageSelectionPolicy,
    #[props(default)] loading: Option<Element>,
    #[props(default)] fallback: Option<Element>,
    children: Element,
) -> Element {
    let state = use_init_asset_i18n_modules(modules, initial_language, selection_policy);
    let load_failure_logged =
        dioxus_core::use_hook(|| std::rc::Rc::new(std::cell::Cell::new(false)));

    match state {
        DioxusAssetI18nLoadState::Loading => loading.unwrap_or_else(VNode::empty),
        DioxusAssetI18nLoadState::Ready(i18n) => rsx! {
            DioxusAssetI18nReadyProvider {
                i18n,
                selection_policy,
                {children}
            }
        },
        DioxusAssetI18nLoadState::Failed(error) => {
            log_asset_provider_load_error_once(&error, &load_failure_logged);
            fallback.unwrap_or_else(VNode::empty)
        },
    }
}

#[cfg(feature = "client")]
#[allow(non_snake_case)]
#[component]
pub fn DioxusAssetI18nReadyProvider(
    i18n: DioxusAssetI18n,
    #[props(default = LanguageSelectionPolicy::BestEffort)]
    selection_policy: LanguageSelectionPolicy,
    children: Element,
) -> Element {
    let _ = use_provide_asset_i18n_with_policy(i18n, selection_policy);
    children
}

#[cfg(feature = "client")]
pub(super) fn log_asset_provider_load_error_once(
    error: &DioxusAssetLoadError,
    logged: &std::rc::Rc<std::cell::Cell<bool>>,
) {
    if logged.get() {
        return;
    }

    tracing::error!(
        error = %error,
        "Dioxus asset i18n provider initialization failed; rendering fallback if configured, otherwise rendering no children",
    );
    logged.set(true);
}

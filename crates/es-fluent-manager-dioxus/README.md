[![Docs](https://docs.rs/es-fluent-manager-dioxus/badge.svg)](https://docs.rs/es-fluent-manager-dioxus/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-dioxus.svg)](https://crates.io/crates/es-fluent-manager-dioxus)

# es-fluent-manager-dioxus

Experimental [Dioxus](https://dioxuslabs.com/) integration for `es-fluent`.

Use this crate when a Dioxus app needs `es-fluent` module discovery plus a Dioxus-owned localization runtime. Most non-Dioxus applications should use [`es-fluent-manager-embedded`](../es-fluent-manager-embedded/README.md) or [`es-fluent-manager-bevy`](../es-fluent-manager-bevy/README.md) instead.

## Features

Enable the runtime surfaces your crate actually uses:

```toml
# Client apps
es-fluent-manager-dioxus = { version = "0.7", features = ["client"] }

# Server-side rendering
es-fluent-manager-dioxus = { version = "0.7", features = ["ssr"] }
```

The crate has no default runtime feature. `define_i18n_module!` is always re-exported.

- `client`: Dioxus provider, hook/context runtime, and signal-backed locale state for interactive rendering.
- `ssr`: synchronous Dioxus SSR runtime with request-scoped localization.

## Client

```toml
[dependencies]
dioxus = "0.7"
es-fluent = { version = "0.15", features = ["derive"] }
es-fluent-manager-dioxus = { version = "0.7", features = ["client"] }
es-fluent-manager-dioxus-derive = "0.7" # optional direct ToFluentString subscriptions
unic-langid = "0.9"
```

Register assets once from the crate with `i18n.toml`:

```ignore
es_fluent_manager_dioxus::define_i18n_module!();
```

Initialize the Dioxus runtime with the provider component, then localize through `use_i18n()` in descendants:

```rs
use dioxus::prelude::*;
use es_fluent::EsFluent;
use es_fluent_manager_dioxus::{DioxusClientBridgeMode, I18nProvider, use_i18n};
use unic_langid::langid;

fn app() -> Element {
    let fallback = rsx! { "Localization failed to initialize" };

    rsx! {
        I18nProvider {
            initial_language: langid!("en-US"),
            bridge_mode: DioxusClientBridgeMode::Disabled,
            fallback: Some(fallback),
            LocaleButton {}
        }
    }
}

#[derive(Clone, Copy, EsFluent)]
#[fluent(namespace = "ui")]
enum UiMessage {
    Hello,
}

#[component]
fn LocaleButton() -> Element {
    let i18n = match use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { "Failed to initialize i18n: {error}" },
    };
    let label = i18n.localize_in_domain_or_id("my-app", "ui_message-Hello", None);

    rsx! {
        button {
            onclick: move |_| {
                if let Err(error) = i18n.select_language(langid!("fr")) {
                    eprintln!("locale switch failed: {error}");
                }
            },
            "{label}"
        }
    }
}
```

This quick start uses context-bound lookup and disables the process-global bridge. That is the safest default for multi-root apps, tests, hot reload, and hosts that may already install an `es-fluent` global localizer. Use direct `message.to_fluent_string()` rendering only when the global bridge tradeoff is acceptable.

`I18nProvider` is a thin provider component over `use_init_i18n_with_bridge_mode(...)`. It logs initialization failures. If `fallback: Option<Element>` is supplied, the provider renders that fallback on initialization failure. Without a fallback it preserves the previous permissive behavior and keeps rendering `children` with a failed i18n context so hook order stays stable. Use `I18nProviderStrict` when children should not render after initialization failure; it renders `fallback` when supplied and otherwise renders an empty vnode. Use `use_init_i18n(...)` directly when the root component needs the `DioxusI18n` handle immediately, or use `use_provide_i18n(...)` when a caller has already constructed a `ManagedI18n`. These hooks return `Result` so components can render initialization or bridge-installation failures. `I18nProvider` and `use_provide_i18n*` initialize once per component instance; changing the initial language, provided manager, or `bridge_mode` after the first render does not replace the installed context. Use `select_language(...)` to change locale at runtime.

After a `ManagedI18n` is handed to the Dioxus provider, route locale switches through `DioxusI18n::select_language(...)` or `DioxusI18n::select_language_strict(...)` so the Dioxus signal is updated with the manager state. `ManagedI18n` equality is identity equality over the shared manager and requested-language state, not semantic equality over modules or locale values.

Client localization should be context-bound through `DioxusI18n` by default:

- `localize(...)` returns `Option<String>` from the current `ManagedI18n`.
- `localize_in_domain(...)` returns `Option<String>` from the current `ManagedI18n` plus an explicit domain.
- `localize_or_id(...)` and `localize_in_domain_or_id(...)` are explicit fallback helpers for UIs that intentionally render message IDs on misses.
- `localize_or_id_silent(...)` and `localize_in_domain_or_id_silent(...)` provide the same fallback without logging a warning.
- `DioxusI18n::to_fluent_string_via_global_bridge(...)` is available when passing the context handle explicitly is clearer, but it still delegates typed-message formatting to the process-global `es-fluent` localizer after subscribing to the Dioxus signal.
- `#[i18n_subscription]` from `es-fluent-manager-dioxus-derive` lets a component keep direct `message.to_fluent_string()` calls while subscribing that component to locale changes; failed subscription attempts are logged.
- `use_i18n_subscription()` is the hook-level version for code that should not use the attribute macro.
- `try_use_i18n_subscription()` does the same when a provider may be absent, returning `Ok(None)` for missing context.
- `requested_language()` returns the requested language, not necessarily the locale used by every message after fallback.
- `select_language(...)` records the requested language and updates the Dioxus signal used by render code.
- `select_language_strict(...)` requires every discovered module to support the requested locale.
- `try_use_i18n()` and `try_consume_i18n()` follow Dioxus optional-context naming; `use_i18n_optional()` remains as a compatibility alias.
- `consume_i18n()` reads the context from event handlers, async tasks, or other places where the Dioxus runtime is active but hooks cannot be called.

Language-selection methods are fallible so UI event handlers can decide how to surface failures.

The client runtime installs the `es-fluent` custom localizer bridge automatically when a ready `ManagedI18n` context is provided. The default bridge mode is strict:

- Reinstalling the same client manager is idempotent.
- A second distinct client owner is rejected with `DioxusGlobalLocalizerError::OwnerConflict`.
- SSR and client ownership conflict intentionally.
- External replacement of the global custom localizer is reported as `DioxusGlobalLocalizerError::ExternalReplacement`.

`I18nProvider` and `use_init_i18n_with_bridge_mode(...)` accept `DioxusClientBridgeMode`:

- `Strict` installs the process-global bridge and returns initialization errors on conflicts. Use it when components call direct `message.to_fluent_string()` and you want conflicts to fail visibly. Do not use it casually in multi-root apps or hosts that may already own the global localizer.
- `BestEffort` keeps the Dioxus context usable if the process-global bridge cannot be installed; raw `DioxusI18n` lookup methods still work. Treat this as a compatibility mode, not the normal path for direct typed rendering, because direct `ToFluentString` calls may not use this Dioxus manager.
- `Disabled` never installs the process-global bridge. Prefer this when every localization call is explicitly context-bound through `DioxusI18n`, especially in tests, hot reload, embedded widgets, or larger apps with another global localizer owner.

Typed `ToFluentString` rendering needs the process-global bridge to point at the Dioxus manager. Prefer `Strict` for direct `message.to_fluent_string()` rendering. Prefer `Disabled` only when using `localize(...)` or `localize_in_domain(...)` directly. `DioxusI18n::to_fluent_string(...)` is kept as a deprecated alias for `to_fluent_string_via_global_bridge(...)`; the explicit name is preferred because this path is a subscription helper plus global-localizer delegation, not a direct context-bound lookup.

If `use_init_i18n(...)` or `use_provide_i18n(...)` cannot initialize or install the client bridge, it still provides a failed context to keep hook order stable. Descendants can call `use_i18n_optional()` to distinguish a missing provider from a failed provider. Pass a visible `fallback` to `I18nProvider` or `I18nProviderStrict` when a blank app or children rendering under a failed context would be misleading.

## SSR

```toml
[dependencies]
dioxus = { version = "0.7", default-features = false, features = ["ssr"] }
es-fluent-manager-dioxus = { version = "0.7", features = ["ssr"] }
```

Install the SSR runtime once during startup, then create request-scoped `SsrI18n` values from it:

```rs
use dioxus::prelude::*;
use es_fluent_manager_dioxus::ssr::SsrI18nRuntime;
use unic_langid::langid;

fn app() -> Element {
    rsx! { div { "hello" } }
}

fn render() -> Result<String, Box<dyn std::error::Error>> {
    let runtime = SsrI18nRuntime::install()?;
    let i18n = runtime.request(langid!("en-US"))?;

    let mut dom = VirtualDom::new(app);
    Ok(i18n.rebuild_and_render(&mut dom)?)
}
```

`SsrI18n` scopes localization to the synchronous render call through a thread-local manager stack. Render and scope methods revalidate that the SSR bridge still owns the global custom localizer before pushing request state. Do not hold `with_sync_thread_local_manager(...)` across `.await`, spawned tasks, streaming callbacks, or fullstack server boundaries.

Safe synchronous usage keeps all localization inside the scope:

```rs
let html = i18n.with_sync_thread_local_manager(|| {
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
})?;
```

Do not create a future inside the scope and await it later:

```rs
let future = i18n.with_sync_thread_local_manager(|| async {
    load_request_data().await;
    dioxus_ssr::render_element(rsx! { "hello" })
})?;

let html = future.await; // the thread-local manager scope has already ended
```

`SsrI18n` values are constructed through `SsrI18nRuntime::request(...)`, which revalidates that the SSR bridge still owns the global custom localizer before creating request state. Each request currently creates a fresh `ManagedI18n`; benchmark this path for high-volume SSR workloads before assuming manager construction is free.

`SsrI18n::managed()` is public because each `SsrI18n` is request-scoped. The client `DioxusI18n` handle does not expose `ManagedI18n`, because direct client language changes would bypass the Dioxus signal that drives rerendering.

If SSR localization is called while the SSR bridge is installed but no request scope is active, the bridge marks the lookup as missing and prevents fallthrough to unrelated global localization state. The string-returning `es-fluent` global helpers still render their normal message-id fallback.

## Process-global bridge lifecycle

The Dioxus client and SSR runtimes install an `es-fluent` custom localizer in process-global state. Treat that bridge as an application-lifetime resource:

- Install one Dioxus bridge owner per process.
- Client and SSR bridge ownership are mutually exclusive in the same process.
- Reinstalling the same client manager or SSR runtime is idempotent.
- Installing a different client manager, crossing client/SSR ownership, or replacing the global localizer externally is reported as an error.
- There is no public uninstall or reset API outside crate tests.
- Hot reload, test harnesses, and multi-root apps can hit these owner checks; handle the returned `Result` rather than assuming bridge installation always succeeds.
- Client-only apps that do not call typed `ToFluentString` localization can opt out with `DioxusClientBridgeMode::Disabled`.
- Use `current_dioxus_global_localizer_owner()` and `is_dioxus_bridge_current()` for read-only diagnostics when startup or hot-reload code needs to report the active Dioxus bridge owner.

[![Docs](https://docs.rs/es-fluent-manager-dioxus/badge.svg)](https://docs.rs/es-fluent-manager-dioxus/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-dioxus.svg)](https://crates.io/crates/es-fluent-manager-dioxus)

# es-fluent-manager-dioxus

[Dioxus](https://dioxuslabs.com/) integration for `es-fluent`.

Use this crate when a Dioxus app needs `es-fluent` module discovery plus Dioxus-owned localization state. Most non-Dioxus applications should use [`es-fluent-manager-embedded`](../es-fluent-manager-embedded/README.md) or [`es-fluent-manager-bevy`](../es-fluent-manager-bevy/README.md) instead.

## Features

Enable the runtime surface your crate uses:

```toml
# Client apps
es-fluent-manager-dioxus = { version = "0.7", features = ["client"] }

# Server-side rendering
es-fluent-manager-dioxus = { version = "0.7", features = ["ssr"] }
```

The crate has no default runtime feature. `define_i18n_module!` is always re-exported.

- `client`: Dioxus provider, hook/context runtime, and signal-backed locale state for interactive rendering.
- `ssr`: request-scoped Dioxus SSR runtime with cached module discovery.

## Client

```ignore
use dioxus::prelude::*;
use es_fluent::{EsFluent, EsFluentLabel, FluentLabel as _};
use es_fluent_manager_dioxus::{I18nProvider, use_i18n};
use unic_langid::langid;

es_fluent_manager_dioxus::define_i18n_module!();

fn app() -> Element {
    rsx! {
        I18nProvider {
            initial_language: langid!("en"),
            LocaleButton {}
        }
    }
}

#[derive(Clone, Copy, EsFluent, EsFluentLabel)]
#[fluent(namespace = "ui")]
#[fluent_label(origin)]
enum UiMessage {
    Hello,
}

#[component]
fn LocaleButton() -> Element {
    let i18n = match use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { "Failed to initialize i18n: {error}" },
    };
    let label = i18n.localize_message(&UiMessage::Hello);
    let title = UiMessage::localize_label(&i18n);

    rsx! {
        button {
            onclick: move |_| {
                if let Err(error) = i18n.select_language(langid!("fr-FR")) {
                    eprintln!("locale switch failed: {error}");
                }
            },
            "{title}: {label}"
        }
    }
}
```

Client apps localize through the `DioxusI18n` context provided by `I18nProvider`, `use_init_i18n(...)`, or `use_provide_i18n(...)`.

- `localize_message(...)` renders `#[derive(EsFluent)]` messages through the Dioxus context and is the preferred typed lookup path.
- `DioxusI18n` implements `FluentLocalizer`, so `#[derive(EsFluentLabel)]` values can call `MyType::localize_label(&i18n)` in client components.
- Raw string-ID lookup is intentionally not exposed as a client convenience API; use typed messages and labels in application code.
- `requested_language()` returns the requested language, not necessarily the locale used by every message after fallback.
- `select_language(...)` records the requested language and updates the Dioxus signal used by render code.
- `select_language_strict(...)` requires every discovered module to support the requested locale.
- `try_use_i18n()` and `try_consume_i18n()` follow Dioxus optional-context naming.
- `consume_i18n()` reads the context from event handlers, async tasks, or other places where the Dioxus runtime is active but hooks cannot be called.

`I18nProvider` is a thin provider component over `use_init_i18n(...)`. It logs initialization failures once per provider instance. If `fallback: Option<Element>` is supplied, the provider renders that fallback on initialization failure. Without a fallback it renders children with a failed i18n context, so descendants that call `use_i18n()` receive the same initialization error. `I18nProviderStrict` is the fail-closed rendering variant: it renders fallback when one is supplied and otherwise renders no children. It uses the same best-effort initial language selection as `I18nProvider`; strictness here does not mean strict locale selection.

`I18nProvider` and `use_provide_i18n(...)` initialize once per component instance. Changing the initial language or provided manager after the first render does not replace the installed context. Use `select_language(...)` to change locale at runtime. After a `ManagedI18n` is handed to the provider, route locale switches through `DioxusI18n::select_language(...)` or `DioxusI18n::select_language_strict(...)` so the Dioxus signal stays aligned with manager state.

Dioxus localizes through explicit component or request context. Keeping lookup context-bound avoids cross-root, hot-reload, test, and SSR request leakage.

## SSR

```ignore
use dioxus::prelude::*;
use es_fluent::EsFluent;
use es_fluent_manager_dioxus::ssr::{SsrI18n, SsrI18nRuntime};
use unic_langid::langid;

es_fluent_manager_dioxus::define_i18n_module!();

#[derive(Clone, Copy, EsFluent)]
#[fluent(namespace = "site")]
enum SiteMessage {
    Title,
}

#[component]
fn App(i18n: SsrI18n) -> Element {
    let title = i18n.localize_message(&SiteMessage::Title);
    rsx! { div { "{title}" } }
}

fn render(runtime: &SsrI18nRuntime) -> Result<String, Box<dyn std::error::Error>> {
    let i18n = runtime.request(langid!("en"))?;
    let mut dom = VirtualDom::new_with_props(
        App,
        AppProps {
            i18n: i18n.clone(),
        },
    );

    Ok(i18n.rebuild_and_render(&mut dom))
}
```

Create one `SsrI18nRuntime` during startup, then create one `SsrI18n` per request. The runtime caches the first validated module-discovery result for its lifetime, including discovery or validation failures; construct a new runtime to retry after a failed discovery. Each request creates fresh manager/localizer state so request languages remain isolated.

SSR components should receive a cloned `SsrI18n` as a prop or through app-owned context and call `localize_message(...)` or `MyType::localize_label(&i18n)`. If SSR components use the Dioxus hook API, call `i18n.provide_context()?` from an app-owned provider component.

Executable Dioxus documentation lives in `examples/dioxus-client-example` and `examples/dioxus-ssr-example` because the client and SSR runtimes are feature-split and validated separately in CI.

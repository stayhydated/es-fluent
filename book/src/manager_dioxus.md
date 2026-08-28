# Dioxus manager

Use `es-fluent-manager-dioxus` for Dioxus client rendering,
request-scoped SSR, or applications that use both.

## Choose features

~~~toml
[dependencies]
dioxus = "0.7"
es-fluent = "0.18"

# Client rendering:
es-fluent-manager-dioxus = { version = "0.7", features = ["client"] }

# SSR only:
# es-fluent-manager-dioxus = { version = "0.7", features = ["ssr"] }

# Client and SSR:
# es-fluent-manager-dioxus = { version = "0.7", features = ["client", "ssr"] }
~~~

The crate has no default runtime feature. The module macro remains available
for all feature combinations. Set `missing_message_policy = "fallback-str"` in
the owning package's `i18n.toml` when client and request-scoped SSR lookup should
render snake_case source names after locale fallback is exhausted.
Fallback-locale values are compile-time checked by default through
`es-fluent-build`.

## Register Dioxus assets

Call the macro from a library-reachable module. The configured
`assets_dir` must be inside the package root because Dioxus
`asset!` owns the resource loading.

~~~rust
// src/i18n.rs
es_fluent_manager_dioxus::define_i18n_module!();
~~~

## Provide client context

~~~rust
use dioxus::prelude::*;
use es_fluent::EsFluent;
use es_fluent_manager_dioxus::{DioxusAssetI18nProvider, use_i18n};
use unic_langid::langid;

#[derive(Clone, Copy, EsFluent)]
enum UiMessage {
    Hello,
}

fn app() -> Element {
    rsx! {
        DioxusAssetI18nProvider {
            initial_language: langid!("en"),
            Greeting {}
        }
    }
}

#[component]
fn Greeting() -> Element {
    let i18n = match use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { "Missing i18n context: {error}" },
    };

    rsx! { p { "{i18n.localize_message(&UiMessage::Hello)}" } }
}
~~~

The provider loads discovered asset modules asynchronously, owns its loading
and failure UI, and publishes a signal-backed context after loading. Descendant
components use `use_i18n()`; event handlers can switch locales through
the returned handle.

During debug WASM runs served by `dx serve`, Dioxus asset hot reload
updates subscribed components when generated FTL changes.

## Create SSR request state

Create one runtime, then request one locale context per render:

~~~rust
use es_fluent_manager_dioxus::ssr::SsrI18nRuntime;
use unic_langid::langid;

async fn request_i18n() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = SsrI18nRuntime::discovered();
    let i18n = runtime.request(langid!("en")).await?;
    // Pass the i18n value into the request's component tree.
    let _ = i18n;
    Ok(())
}
~~~

`request(...)` and `request_strict(...)` are asynchronous
because asset reads are asynchronous. Blocking variants are available for
static generation. Render helpers do not install context automatically; pass
`SsrI18n` as a prop or provide it from the request's component tree.

Enable both `client` and `ssr` if SSR components use the
Dioxus hook API. Use an explicit module set only when the application should
load a subset of discovered translations.

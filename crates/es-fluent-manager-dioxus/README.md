[![Docs](https://docs.rs/es-fluent-manager-dioxus/badge.svg)](https://docs.rs/es-fluent-manager-dioxus/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-dioxus.svg)](https://crates.io/crates/es-fluent-manager-dioxus)

# es-fluent-manager-dioxus

[Dioxus](https://dioxuslabs.com/) integration for `es-fluent`.

Use this crate when a Dioxus app needs typed `es-fluent` localization loaded
through Dioxus assets. Most non-Dioxus applications should use
[`es-fluent-manager-embedded`](../es-fluent-manager-embedded/README.md) or
[`es-fluent-manager-bevy`](../es-fluent-manager-bevy/README.md) instead.

## Features

Enable the runtime surface your crate uses:

```toml
# Client apps
es-fluent-manager-dioxus = { version = "0.7", features = ["client"] }

# Server-side rendering
es-fluent-manager-dioxus = { version = "0.7", features = ["ssr"] }

# Fullstack or static rendering that uses both paths
es-fluent-manager-dioxus = { version = "0.7", features = ["client", "ssr"] }
```

The crate has no default runtime feature. `define_i18n_module!` is always
re-exported and emits Dioxus `asset!` registrations plus a
`dioxus_i18n_asset_modules()` helper for the current crate.

- `client`: Dioxus provider, hook/context runtime, async asset loading, and signal-backed locale state for interactive rendering.
- `ssr`: request-scoped SSR helpers backed by the same Dioxus asset module set.

## Define the Module

Prefer a library-reachable module, usually `src/i18n.rs` declared from
`src/lib.rs`, so `cargo es-fluent generate` can discover localizable types from
the library target:

```ignore
// a i18n.toml file must exist in the root of the crate
es_fluent_manager_dioxus::define_i18n_module!();
```

The macro scans the configured `assets_dir` and generates
`dioxus_i18n_asset_module()`, `dioxus_i18n_asset_modules()`,
`load_dioxus_i18n_assets(...)`, and `load_dioxus_i18n_assets_with_policy(...)`.
Dioxus `asset!` requires the
configured `assets_dir` to be inside the package root. Keep FTL files in
`assets/locales` or another package-local source asset directory, not under
Dioxus `public`, unless you intentionally want to publish raw translation files
as static web output.

## Client

```ignore
use dioxus::prelude::*;
use es_fluent::{EsFluent, EsFluentLabel, FluentLabel as _};
use es_fluent_manager_dioxus::{DioxusAssetI18nProvider, use_asset_i18n};
use unic_langid::langid;

use crate::i18n::dioxus_i18n_asset_modules;

fn app() -> Element {
    rsx! {
        DioxusAssetI18nProvider {
            modules: dioxus_i18n_asset_modules(),
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
    let i18n = match use_asset_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { "Missing i18n context: {error}" },
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

`DioxusAssetI18nProvider` loads the generated asset module set with a Dioxus
resource. It renders `loading` while assets are being read, renders `fallback`
on load failure, and otherwise provides a `DioxusAssetI18nHandle` through
`use_asset_i18n()`, `try_use_asset_i18n()`, `consume_asset_i18n()`, or
`try_consume_asset_i18n()`.
Dioxus app translations are loaded only through the generated asset modules.
Runtime follower modules that do not count as locale support, such as
`es-fluent-lang` language labels, are discovered automatically and follow the
selected asset-backed locale.
When a Dioxus app needs asset translations from multiple crates, create a
package-local static slice of `dioxus_i18n_asset_module()` references and pass
`DioxusI18nAssetModules::new(...)` to the provider.
In debug WASM builds served by `dx serve`, changed FTL assets are reloaded from
Dioxus asset hot-reload messages and the provider updates subscribed
components while preserving the requested locale when possible.

- `localize_message(...)` renders `#[derive(EsFluent)]` messages through the Dioxus context and is the preferred typed lookup path.
- `DioxusAssetI18nHandle` implements `FluentLocalizer`, so `#[derive(EsFluentLabel)]` values can call `MyType::localize_label(&i18n)` in client components.
- `requested_language()` returns the requested language, not necessarily the locale used by every message after fallback.
- `select_language(...)` records the requested language and updates the Dioxus signal used by render code.
- `select_language_strict(...)` requires every generated module to support the requested locale.
- `use_init_asset_i18n_modules(...)` returns `DioxusAssetI18nLoadState` for applications that want to own the loading UI and pass an explicit `LanguageSelectionPolicy`.

Dioxus localizes through explicit component or request context. Keeping lookup
context-bound avoids cross-root, hot-reload, test, and SSR request leakage.

## SSR

```ignore
use dioxus::prelude::*;
use es_fluent::EsFluent;
use es_fluent_manager_dioxus::ssr::{SsrI18n, SsrI18nRuntime};
use unic_langid::langid;

use crate::i18n::dioxus_i18n_asset_modules;

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

async fn render() -> Result<String, Box<dyn std::error::Error>> {
    let runtime = SsrI18nRuntime::new(dioxus_i18n_asset_modules());
    let i18n = runtime.request(langid!("en")).await?;
    let mut dom = VirtualDom::new_with_props(
        App,
        AppProps {
            i18n: i18n.clone(),
        },
    );

    Ok(i18n.rebuild_and_render(&mut dom))
}
```

Create one `SsrI18nRuntime` with the generated module handle, then create one
`SsrI18n` per request. `request(...)` and `request_strict(...)` are async
because Dioxus asset reads are async; `request_blocking(...)` and
`request_strict_blocking(...)` are available for static generation or other
synchronous SSR entry points.

The render helpers do not install context automatically; pass `SsrI18n` as a
prop or call `provide_context()` from a component when using hook-based lookup.

Executable Dioxus documentation lives in `web`, which uses the same generated
asset module handle for browser and server checks.

# Public Facades

Use this reference to choose the crate or integration surface before writing code.

## Selection Table

| Need | Use | Notes |
| --- | --- | --- |
| Define typed messages, labels, variants, choices | `es-fluent` | Default application dependency. Re-exports derive and runtime traits. |
| Generate/check/format/sync FTL | `cargo es-fluent` from `es-fluent-cli` | Use from crate or workspace root. Inventory comes from library targets. |
| Track locale asset rebuilds from `build.rs` | `es-fluent-build` in `[build-dependencies]` | Call `es_fluent_build::track_i18n_assets()` when manager macros scan locale assets at compile time. |
| General Rust runtime, CLI, TUI, desktop, GPUI-style apps | `es-fluent-manager-embedded` | Embeds FTL files and returns explicit `EmbeddedI18n` handles. |
| Dioxus client UI | `es-fluent-manager-dioxus` with `client` | Use `define_i18n_module!`, pass generated `dioxus_i18n_asset_modules()` to `DioxusAssetI18nProvider`, aggregate multiple crates with `dioxus_i18n_asset_module()` when needed, and localize through `use_i18n()`. |
| Dioxus SSR | `es-fluent-manager-dioxus` with `ssr` | Create `SsrI18nRuntime::new(dioxus_i18n_asset_modules())`, then one `SsrI18n` per request. |
| Bevy ECS/assets | `es-fluent-manager-bevy` | Add `I18nPlugin`, use `FluentText<T>`, `BevyFluentText`, and `BevyI18n`. |
| Typed language picker | `es-fluent-lang` | Use `#[es_fluent_language]` on an empty enum discovered from locale folders. |

## Version Lines

Current public documentation uses:

```toml
[dependencies]
es-fluent = "0.16"
unic-langid = "0.9"
es-fluent-manager-embedded = "0.16"

# Dioxus
es-fluent-manager-dioxus = { version = "0.7", features = ["client"] }
es-fluent-manager-dioxus = { version = "0.7", features = ["ssr"] }
es-fluent-manager-dioxus = { version = "0.7", features = ["client", "ssr"] }

# Bevy
es-fluent-manager-bevy = "0.18.13"

[build-dependencies]
es-fluent-build = "0.16"
```

## Common Setup

Prefer CLI scaffolding:

```sh
cargo es-fluent init --update-cargo-toml
```

Useful manager variants:

```sh
cargo es-fluent init --manager dioxus --dioxus-runtime client --update-cargo-toml
cargo es-fluent init --manager dioxus --dioxus-runtime ssr --update-cargo-toml
cargo es-fluent init --manager bevy --update-cargo-toml
```

The standard module should be library-reachable:

```rust
// src/i18n.rs
pub use es_fluent_manager_embedded::{
    EmbeddedI18n as I18n, EmbeddedInitError, LocalizationError,
};

es_fluent_manager_embedded::define_i18n_module!();
```

```rust
// src/lib.rs
pub mod i18n;
```

## Embedded Manager

Use for ordinary Rust applications:

```rust
use es_fluent::EsFluent;
use es_fluent_manager_embedded::EmbeddedI18n;
use unic_langid::langid;

#[derive(EsFluent)]
struct Greeting<'a> {
    name: &'a str,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let i18n = EmbeddedI18n::try_new_with_language(langid!("en"))?;
    let text = i18n.localize_message(&Greeting { name: "Ada" });
    println!("{text}");
    Ok(())
}
```

Use `try_new_with_language_strict(...)` or `select_language_strict(...)` when every discovered module must support the selected locale.

## Dioxus Manager

Client apps localize through Dioxus context:

```rust
use dioxus::prelude::*;
use es_fluent::{EsFluent, EsFluentLabel, FluentLabel as _};
use es_fluent_manager_dioxus::{DioxusAssetI18nProvider, use_i18n};
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
    let i18n = match use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { "Missing i18n context: {error}" },
    };
    let label = i18n.localize_message(&UiMessage::Hello);
    let title = UiMessage::localize_label(&i18n);

    rsx! { button { "{title}: {label}" } }
}
```

SSR apps create request-scoped state:

```rust
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
    let mut dom = VirtualDom::new_with_props(App, AppProps { i18n: i18n.clone() });
    Ok(i18n.rebuild_and_render(&mut dom))
}
```

Route locale switches through `DioxusAssetI18nHandle::select_language(...)`.
Dioxus asset loading is async, so `DioxusAssetI18nProvider` owns
loading/failure rendering on the client and `SsrI18nRuntime::request(...)` is
async on the server. Application translations come from the generated Dioxus
asset modules; runtime follower modules such as `es-fluent-lang` language
labels are discovered automatically.
For component libraries with their own Dioxus FTL assets, give the library its
own `i18n.toml` and `define_i18n_module!()`, then pass a static aggregate of
the app and library `dioxus_i18n_asset_module()` references to
`DioxusI18nAssetModules::new(...)`.
During `dx serve` debug WASM runs, changed generated FTL assets refresh the
provider context through Dioxus asset hot reload while preserving the requested
locale when possible.

## Bevy Manager

Use Bevy's plugin and ECS surfaces:

```rust
use bevy::prelude::*;
use es_fluent_manager_bevy::I18nPlugin;
use unic_langid::langid;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(I18nPlugin::with_language(langid!("en")))
        .run();
}
```

If the Bevy asset root is `assets` but translations live in `assets/i18n`, configure the path relative to the asset root:

```rust
use es_fluent_manager_bevy::{I18nPlugin, I18nPluginConfig};

app.add_plugins(I18nPlugin::with_config(
    I18nPluginConfig::new(langid!("en")).with_asset_path("i18n"),
));
```

Prefer `BevyFluentText` for UI messages:

```rust
use bevy::prelude::Component;
use es_fluent::EsFluent;
use es_fluent_manager_bevy::{BevyFluentText, FluentText};

#[derive(BevyFluentText, Clone, Component, EsFluent)]
pub enum UiMessage {
    StartGame,
    Settings,
}

fn spawn_menu(mut commands: Commands) {
    commands.spawn((FluentText::new(UiMessage::StartGame), Text::new("")));
}
```

For direct localization in systems, request `BevyI18n` as a `SystemParam` and call `localize_message(...)`.

## Language Enum

Use `es-fluent-lang` when the UI needs a type-safe supported-language list:

```rust
use es_fluent_lang::es_fluent_language;
use strum::EnumIter;

#[es_fluent_language]
#[derive(Clone, Copy, Debug, Eq, EnumIter, PartialEq)]
pub enum Languages {}
```

The macro scans `i18n.toml` and canonical locale folders, implements `Default` from `fallback_language`, conversion to/from `LanguageIdentifier`, and `FluentMessage` for rendering language labels through the active manager:

```rust
use strum::IntoEnumIterator as _;

for language in Languages::iter() {
    let label = i18n.localize_message(&language);
    println!("{language:?}: {label}");
}

i18n.select_language(Languages::FrFr)?;
```

Use `#[es_fluent_language(mode = "custom")]` when the application ships its own translated language names.

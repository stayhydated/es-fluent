# Public Facades

Use this reference to choose the crate or integration surface before writing code.

## Selection Table

| Need | Use | Notes |
| --- | --- | --- |
| Define typed messages, labels, variants, choices | `es-fluent` | Default application dependency. Re-exports derive and runtime traits. |
| Generate/check/format/sync FTL | `cargo es-fluent` from `es-fluent-cli` | Use from crate or workspace root. Inventory comes from library targets. |
| Track locale asset rebuilds from `build.rs` | `es-fluent-build` in `[build-dependencies]` | Call `es_fluent_build::track_i18n_assets()` when manager macros scan locale assets at compile time. |
| General Rust runtime, CLI, TUI, desktop, GPUI-style apps | `es-fluent-manager-embedded` | Embeds FTL files and returns explicit `EmbeddedI18n` handles. |
| Dioxus client UI | `es-fluent-manager-dioxus` with `client` | Use `I18nProvider`, `use_i18n`, `DioxusI18n::localize_message`. Add `debug-embed` for browser WASM debug builds using `define_i18n_module!`. |
| Dioxus SSR | `es-fluent-manager-dioxus` with `ssr` | Create one `SsrI18nRuntime`, then one `SsrI18n` per request. |
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
use es_fluent_manager_dioxus::{I18nProvider, use_i18n};
use unic_langid::langid;

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

    rsx! { button { "{title}: {label}" } }
}
```

SSR apps create request-scoped state:

```rust
use dioxus::prelude::*;
use es_fluent::EsFluent;
use es_fluent_manager_dioxus::ssr::{SsrI18n, SsrI18nRuntime};
use unic_langid::langid;

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
    let mut dom = VirtualDom::new_with_props(App, AppProps { i18n: i18n.clone() });
    Ok(i18n.rebuild_and_render(&mut dom))
}
```

Dioxus does not use a process-wide localizer. Route locale switches through `DioxusI18n::select_language(...)` or `SsrI18nRuntime::request(...)`.

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
use es_fluent::EsFluent;
use es_fluent_lang::es_fluent_language;
use strum::EnumIter;

#[es_fluent_language]
#[derive(Clone, Copy, Debug, Eq, EsFluent, EnumIter, PartialEq)]
pub enum Languages {}
```

The macro scans `i18n.toml` and canonical locale folders, implements `Default` from `fallback_language`, conversion to/from `LanguageIdentifier`, and can render language labels through the active manager:

```rust
use strum::IntoEnumIterator as _;

for language in Languages::iter() {
    let label = i18n.localize_message(&language);
    println!("{language:?}: {label}");
}

i18n.select_language(Languages::FrFr)?;
```

Use `#[es_fluent_language(custom)]` when the application ships its own translated language names.

# Public Facades

Use this reference to choose the crate or integration surface before writing code.

## Selection Table

| Need | Use | Notes |
| --- | --- | --- |
| Define typed messages, labels, variants, choices | `es-fluent` | Default application dependency. Re-exports derive and runtime traits. |
| Generate/check/format/sync FTL | `cargo es-fluent` from `es-fluent-cli` | Use from an existing crate/workspace root, its `Cargo.toml`, or a path inside a crate. Inventory comes from library targets. |
| Track locale asset rebuilds from `build.rs` | `es-fluent-build` in `[build-dependencies]` | Call `es_fluent_build::track_i18n_assets()` when manager macros scan locale assets at compile time. |
| General Rust runtime, CLI, TUI, desktop, GPUI-style apps | `es-fluent-manager-embedded` | Embeds FTL files and returns explicit `EmbeddedI18n` handles. |
| Dioxus client UI | `es-fluent-manager-dioxus` with `client` | Use `define_i18n_module!`, let `DioxusAssetI18nProvider` load inventory-discovered asset modules, pass `DioxusI18nAssetModules::new(...)` only for explicit subsets, and localize through `use_i18n()`. |
| Dioxus SSR | `es-fluent-manager-dioxus` with `ssr` | Create `SsrI18nRuntime::discovered()`, then one `SsrI18n` per request. |
| Bevy ECS/assets | `es-fluent-manager-bevy` | Add `I18nPlugin`, use `FluentText<T>`, `BevyFluentText`, and `BevyI18n`; generated modules load FTL from the owning crate. |
| Typed language picker | `es-fluent-lang` | Use `#[es_fluent_language]` on an empty enum discovered from locale folders. |

## Version Lines

Current public documentation uses:

```toml
[dependencies]
es-fluent = "0.16"
unic-langid = "0.9"
es-fluent-manager-embedded = "0.16"

# For Dioxus apps, enable only the runtime surface you use.
# es-fluent-manager-dioxus = { version = "0.7", features = ["client"] }
# es-fluent-manager-dioxus = { version = "0.7", features = ["ssr"] }
# es-fluent-manager-dioxus = { version = "0.7", features = ["client", "ssr"] }

# For Bevy integration, use `es-fluent-manager-bevy`.
# es-fluent-manager-bevy = "0.19.0"

[build-dependencies]
es-fluent-build = "0.16"
```

## Common Setup

Create `i18n.toml` next to the crate `Cargo.toml`, create the fallback locale directory, and keep the i18n module library-reachable:

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

Prefer concrete manager `localize_message(...)` methods for application code.
Manager-core lookup and custom `es_fluent::FluentLocalizer` integrations receive
typed `StaticFluentDomain`, `StaticFluentEntryId`, and typed Fluent argument
maps; convert to raw strings only at the final Fluent bundle lookup boundary.

## Dioxus Manager

Client apps localize through Dioxus context:

```rust
use dioxus::prelude::*;
use es_fluent::{EsFluent, EsFluentLabel, FluentLabel as _};
use es_fluent_manager_dioxus::{DioxusAssetI18nProvider, use_i18n};
use unic_langid::langid;

fn app() -> Element {
    rsx! {
        DioxusAssetI18nProvider {
            initial_language: langid!("en"),
            LocaleButton {}
        }
    }
}

#[derive(Clone, Copy, EsFluent, EsFluentLabel)]
#[fluent(namespace = "ui")]
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
    let _ = UiMessage::try_localize_label(&i18n);

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

async fn render() -> Result<String, Box<dyn std::error::Error>> {
    let runtime = SsrI18nRuntime::discovered();
    let i18n = runtime.request(langid!("en")).await?;
    let mut dom = VirtualDom::new_with_props(App, AppProps { i18n: i18n.clone() });
    Ok(i18n.rebuild_and_render(&mut dom))
}
```

Route locale switches through `DioxusAssetI18nHandle::select_language(...)`.
Dioxus asset loading is async, so `DioxusAssetI18nProvider` owns
loading/failure rendering on the client and `SsrI18nRuntime::request(...)` is
async on the server. Application translations come from the generated Dioxus
asset modules registered with inventory; runtime follower modules such as
`es-fluent-lang` language labels are discovered automatically.
For an explicit subset of Dioxus FTL assets, pass a static aggregate of
`dioxus_i18n_asset_module()` references to
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

Generated Bevy module registrations register the owning crate's `.ftl` files
from that crate's configured `assets_dir` as Bevy embedded assets; consuming
apps should not copy dependency-owned domain files into their own asset tree.
`asset_path` is only used for custom metadata-only registrations that do not
provide owner embedded assets. If the Bevy asset root is `assets` but those
custom resources live in `assets/i18n`, configure the path relative to the
asset root:

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

When using `#[locale]` with `BevyFluentText`, mark only named struct fields or
named enum variant fields whose types implement `TryFrom<&LanguageIdentifier>`.

## Language Enum

Use `es-fluent-lang` when the UI needs a type-safe supported-language list:

```rust
use es_fluent_lang::es_fluent_language;
use strum::EnumIter;

#[es_fluent_language]
#[derive(EnumIter)]
pub enum Languages {}
```

The macro scans `i18n.toml` and canonical locale folders, derives `Clone`, `Copy`, `Debug`, `Eq`, `Hash`, and `PartialEq` automatically, implements `Default` from `fallback_language`, conversion to/from `LanguageIdentifier`, and `FluentMessage` for rendering language labels through the active manager:

```rust
use strum::IntoEnumIterator as _;

for language in Languages::iter() {
    let label = i18n.localize_message(&language);
    println!("{language:?}: {label}");
}

i18n.select_language(Languages::FrFr)?;
```

Use `#[es_fluent_language(custom)]` when the application ships its own translated language names.

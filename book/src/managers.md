# Runtime Managers

`es-fluent` is agnostic about how you load translations at runtime. The
ecosystem provides three ready-made manager crates so you don't have to build
your own asset pipeline.

| Manager                      | Best for                 | How it works                                                 |
| ---------------------------- | ------------------------ | ------------------------------------------------------------ |
| `es-fluent-manager-embedded` | CLIs, TUIs, desktop apps | Compiles FTL files into the binary                           |
| `es-fluent-manager-dioxus`   | Dioxus apps              | Uses embedded assets plus Dioxus hooks or request-scoped SSR |
| `es-fluent-manager-bevy`     | Bevy games and apps      | Loads FTL files via Bevy's `AssetServer`                     |

---

## Embedded Manager (`es-fluent-manager-embedded`)

Bundles your translations directly into the binary and returns an explicit manager handle. No external files needed at runtime.

### Features

- **Embedded Assets**: Compiles your FTL files into the binary.
- **Explicit Context**: Keep the manager handle in application state and pass it to code that localizes messages.
- **Thread Safe**: Safe to use from multiple threads after initialization.

### Quick Start

#### 1. Define the Module

In your crate root (`lib.rs` or `main.rs`), tell the manager to scan your assets:

```rust
// a i18n.toml file must exist in the root of the crate
es_fluent_manager_embedded::define_i18n_module!();
```

#### 2. Initialize & Use

In your application entry point:

```rust
use es_fluent::EsFluent;
use es_fluent_manager_embedded::EmbeddedI18n;
use unic_langid::langid;

#[derive(EsFluent)]
enum MyMessage {
    Hello { name: String },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let i18n = EmbeddedI18n::try_new_with_language(langid!("en-US"))?;

    let msg = MyMessage::Hello { name: "World".to_string() };
    println!("{}", i18n.localize_message(&msg));

    Ok(())
}
```

If you have a [Language Enum](language_enum.md), you can pass it directly since it implements `Into<LanguageIdentifier>`:

```rust
let i18n = es_fluent_manager_embedded::EmbeddedI18n::try_new_with_language(Languages::En)?;
```

If the language is not known during startup, create the context first and switch
later with `select_language(...)`:

```rust
let i18n = es_fluent_manager_embedded::EmbeddedI18n::try_new()?;
i18n.select_language(Languages::Fr)?;
```

`select_language(...)` returns an error if no discovered module can serve the
requested locale, or if a supported locale's resources would build a broken
Fluent bundle. When some modules support the requested locale and others do
not, the default switch keeps the supporting modules active. Failed switches
keep the previous ready locale active.

For custom runtime integrations, `es-fluent-manager-core` exposes the same
strict discovery behavior through
`FluentManager::try_new_with_discovered_modules()`. Most applications should
prefer a concrete manager crate instead of wiring the shared context manually.

The embedded manager also uses strict discovery and returns initialization
errors before the manager is returned:

```rust
es_fluent_manager_embedded::EmbeddedI18n::try_new_with_language(Languages::Fr)
    .expect("embedded i18n manager should initialize");
```

`try_new_with_language(...)` only returns the embedded context after the
requested language has been selected successfully.

---

## Dioxus Manager (`es-fluent-manager-dioxus`)

Dioxus integration for `es-fluent`.

Enable the runtime surface your crate uses:

```toml
# Client apps
es-fluent-manager-dioxus = { version = "0.8", features = ["client"] }

# SSR
es-fluent-manager-dioxus = { version = "0.8", features = ["ssr"] }
```

The crate has no default runtime feature. The `define_i18n_module!` macro is always available.

- `client`: Dioxus provider, hook/context runtime, and signal-backed locale state for interactive rendering.
- `ssr`: request-scoped Dioxus SSR runtime with cached module discovery.

### Client Quick Start

```rust
use dioxus::prelude::*;
use es_fluent::EsFluent;
use es_fluent_manager_dioxus::{I18nProvider, use_i18n};
use unic_langid::langid;

es_fluent_manager_dioxus::define_i18n_module!();

fn app() -> Element {
    rsx! {
        I18nProvider {
            initial_language: langid!("en-US"),
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
    let label = i18n.localize_message(&UiMessage::Hello);

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

Client apps should localize through the `DioxusI18n` context provided by `I18nProvider`, `use_init_i18n(...)`, or `use_provide_i18n(...)`. Those hooks initialize once; changing the initial language or provided manager after the first render does not replace the installed context. Use `localize_message(...)` for typed context-bound lookup, use `localize(...)` and `localize_in_domain(...)` for string ID lookup, or use explicit fallback helpers when rendering message IDs on misses is intended. Locale switches use fallible `select_language(...)` or `select_language_strict(...)`; after a manager is handed to the Dioxus provider, route language changes through those `DioxusI18n` methods so the Dioxus signal stays aligned with manager state.

Dioxus localizes through explicit component or request context. Keeping lookup context-bound avoids cross-root, hot-reload, test, and SSR request leakage.

If `use_init_i18n(...)` cannot initialize, it still provides a failed context to keep hook order stable for callers that inspect the returned `Result` directly. `I18nProvider` logs that failure and renders `fallback` when one is supplied; without a fallback it renders children without an initialized i18n context, so descendants that call `use_i18n()` receive an initialization error. `I18nProviderStrict` is the fail-closed variant: it renders fallback when one is supplied and otherwise renders an empty vnode. Descendants can call `try_use_i18n()` to distinguish a missing provider from a failed provider. Event handlers and async tasks can call `consume_i18n()` or `try_consume_i18n()` while the Dioxus runtime is active.

### SSR Quick Start

```rust
use dioxus::prelude::*;
use es_fluent_manager_dioxus::{ManagedI18n, ssr::SsrI18nRuntime};
use unic_langid::langid;

fn app(i18n: ManagedI18n) -> Element {
    let title = i18n.localize_message(&SiteMessage::Title);
    rsx! { div { "{title}" } }
}

let runtime = SsrI18nRuntime::new();
let i18n = runtime
    .request(langid!("en-US"))
    .expect("ssr i18n should initialize");

let mut vdom = VirtualDom::new_with_props(
    app,
    appProps {
        i18n: i18n.managed().clone(),
    },
);
let html = i18n.rebuild_and_render(&mut vdom);
```

Create one `SsrI18nRuntime` during startup, then create one `SsrI18n` per request. The runtime caches validated module discovery. Each request creates fresh manager/localizer state so request languages remain isolated.

SSR components should receive a cloned `ManagedI18n` as a prop or through app-owned context and call `localize_message(...)`

---

## Bevy Manager (`es-fluent-manager-bevy`)

Seamless [Bevy](https://bevyengine.org/) integration for `es-fluent`. This plugin connects type-safe localization with Bevy's ECS and Asset system, allowing `#[derive(EsFluent)]` types to serve as components that automatically update when the language changes.

### Features

- **Asset Loading**: Loads `.ftl` files via Bevy's `AssetServer`.
- **Hot Reloading**: Supports hot-reloading of translations during development.
- **Reactive UI**: The `FluentText` component automatically refreshes text when the locale changes.
- **Bevy-native Context**: Systems can request `BevyI18n` as a `SystemParam` for direct localization.
- **Explicit Context**: Localization comes from Bevy resources instead of a context-free bridge.

### Quick Start

#### 1. Define the Module

In your crate root (`lib.rs` or `main.rs`), tell the manager to scan your assets:

```rust
// a i18n.toml file must exist in the root of the crate
es_fluent_manager_bevy::define_i18n_module!();
```

#### 2. Initialize & Use

Add the plugin to your `App`:

```rust
use bevy::prelude::*;
use es_fluent_manager_bevy::I18nPlugin;
use unic_langid::langid;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(I18nPlugin::with_language(langid!("en-US")))
        .run();
}
```

`I18nPlugin` localizes `FluentText` components through Bevy resources and does
not install a process-wide localization hook.

Plugin startup also uses strict module discovery, so invalid or duplicate i18n
module registrations fail the app boot instead of being normalized silently.
Failed hot reloads or locale switches keep the last accepted locale active
instead of publishing a broken update.

Use `RequestedLanguageId` to read the latest user intent and `ActiveLanguageId`
to read the currently published locale. `LocaleChangedEvent` refers to
`ActiveLanguageId`, not merely the latest request.

For direct localization inside a system, request `BevyI18n` like any other
Bevy system parameter:

```rust
use es_fluent_manager_bevy::BevyI18n;

fn update_title(i18n: BevyI18n) {
    let title = i18n.localize_message(&UiMessage::Settings);
    // apply `title` to your Bevy UI, window, or gameplay state
}
```

#### 3. Define Localizable Components (Recommended)

Prefer the `BevyFluentText` derive macro. It auto-registers your type with `I18nPlugin` via inventory, so you don't have to call any registration functions manually.

If a field depends on the active locale (like the `Languages` enum from [Language Enum](language_enum.md)), mark it with `#[locale]`. The macro will generate `RefreshForLocale` and register the locale-aware systems for you.
`#[locale]` is supported on named struct fields and named enum variant fields, and you can mark more than one named field in the same variant when they all need refresh behavior.

`RefreshForLocale` receives the originally requested locale, not the fallback resource locale. For example, if `en-GB` falls back to `en` assets, locale-aware fields still refresh with `en-GB`.

```rust
use bevy::prelude::Component;
use es_fluent::EsFluent;
use es_fluent_manager_bevy::BevyFluentText;

#[derive(BevyFluentText, Clone, Component, EsFluent)]
pub enum UiMessage {
    StartGame,
    Settings,
    LanguageHint {
        #[locale]
        current_language: Languages,
    },
}
```

#### 4. Using in UI

Use the `FluentText` component wrapper for any type that implements `FluentMessage` (which `#[derive(EsFluent)]` provides).

```rust
use es_fluent_manager_bevy::FluentText;

fn spawn_menu(mut commands: Commands) {
    commands.spawn((
        // This text will automatically update if language changes
        FluentText::new(UiMessage::StartGame),
        Text::new(""),
    ));
}
```

#### Manual Registration (Fallback)

If you cannot derive `BevyFluentText` (e.g., external types), you can still register manually:

```rust
app.register_fluent_text::<UiMessage>();
```

If the type needs locale refresh, implement `RefreshForLocale` and use the locale-aware registration function:

```rust
use es_fluent_manager_bevy::RefreshForLocale;

#[derive(EsFluent, Clone, Component)]
pub enum UiMessage {
    LanguageHint { current_language: Languages },
}

impl RefreshForLocale for UiMessage {
    fn refresh_for_locale(&mut self, lang: &unic_langid::LanguageIdentifier) {
        match self {
            UiMessage::LanguageHint { current_language } => {
                if let Ok(value) = Languages::try_from(lang) {
                    *current_language = value;
                }
            }
        }
    }
}

app.register_fluent_text_from_locale::<UiMessage>();
```

#### Do Nested Types Need `BevyFluentText`?

Only the **component type** wrapped by `FluentText<T>` needs registration. If a nested field (like `KbKeys`) is only used inside a registered component, it does **not** need `BevyFluentText`. When the parent component re-renders, its `EsFluent` implementation formats all fields using the current locale.

You only need `BevyFluentText` for a nested type if you plan to use it directly as `FluentText<ThatType>` or otherwise register it as its own component.

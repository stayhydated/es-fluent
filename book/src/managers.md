# Runtime Managers

`es-fluent` is agnostic about how you load translations at runtime. The
ecosystem provides three ready-made manager crates so you don't have to build
your own asset pipeline.

| Manager                      | Best for                 | How it works                             |
| ---------------------------- | ------------------------ | ---------------------------------------- |
| `es-fluent-manager-embedded` | CLIs, TUIs, desktop apps | Compiles FTL files into the binary       |
| `es-fluent-manager-dioxus`   | Dioxus apps              | Uses embedded assets plus Dioxus hooks   |
| `es-fluent-manager-bevy`     | Bevy games and apps      | Loads FTL files via Bevy's `AssetServer` |

---

## Embedded Manager (`es-fluent-manager-embedded`)

Bundles your translations directly into the binary and exposes a global singleton. No external files needed at runtime.

### Features

- **Embedded Assets**: Compiles your FTL files into the binary.
- **Global Access**: Once initialized, you can call `to_fluent_string()` anywhere in your code without passing context around.
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
use es_fluent::ToFluentString;
use unic_langid::langid;

fn main() {
    // 1. Initialize the global manager with the active language
    es_fluent_manager_embedded::init_with_language(langid!("en-US"));

    // 2. Localize things!
    let msg = MyMessage::Hello { name: "World" };
    println!("{}", msg.to_fluent_string());
}
```

If you have a [Language Enum](language_enum.md), you can pass it directly since it implements `Into<LanguageIdentifier>`:

```rust
es_fluent_manager_embedded::init_with_language(Languages::En);
```

If the language is not known during startup, call `init()` and switch later with
`select_language(...)`:

```rust
es_fluent_manager_embedded::init();
es_fluent_manager_embedded::select_language(Languages::Fr)
    .expect("manager initialized and locale is available");
```

`select_language(...)` returns an error if initialization was skipped, if no
discovered module can serve the requested locale, or if a supported locale's
resources would build a broken Fluent bundle (for example duplicate message
definitions across loaded files). When some modules support the requested
locale and others do not, the default switch keeps the supporting modules
active. Failed switches keep the previous ready locale active.

For custom runtime integrations, `es-fluent-manager-core` exposes the same
strict discovery behavior through
`FluentManager::try_new_with_discovered_modules()`. Most applications should
prefer a concrete manager crate instead of wiring the shared context manually.

The embedded manager also uses strict discovery. `init_with_language(...)`
logs initialization errors, while the fallible entry points return them before
the singleton is published:

```rust
es_fluent_manager_embedded::try_init_with_language(Languages::Fr)
    .expect("embedded i18n manager should initialize");
```

Both `init_with_language(...)` and `try_init_with_language(...)` only publish
the embedded singleton after the requested language has been selected
successfully.

---

## Dioxus Manager (`es-fluent-manager-dioxus`)

Experimental Dioxus integration for `es-fluent`.

The Dioxus manager is split by rendering model:

- `client` is the shared client-side hook/runtime layer for Dioxus web, desktop, and mobile renderers.
- `ssr` is separate and wraps synchronous `dioxus::ssr` rendering with a request-scoped localization bridge.

### Features

- **Embedded Assets**: Uses the same compile-time locale discovery flow as the embedded manager.
- **Reactive Locale State**: `use_i18n_provider_once(...)` exposes locale changes through Dioxus signals so render code can rerun when the language changes.
- **Context-First API**: `DioxusI18n` lookup methods resolve through the provider context and are the recommended component path.
- **Explicit Global Bridge**: `ToFluentString` support is available through `GlobalBridgePolicy`, but it is a compatibility layer for code that cannot receive context.
- **Separate SSR Surface**: `SsrI18n` owns its own request-scoped render context instead of pretending SSR behaves like a client app.

### Quick Start

#### 1. Add Dependencies

The default features enable `define_i18n_module!` and the generic `client` hook runtime. Enable `ssr` for server rendering. Mixed client/SSR feature sets are mainly for examples and tests.

```toml
[dependencies]
dioxus = { version = "0.7", features = ["desktop"] }
es-fluent = { version = "0.15", features = ["derive"] }
es-fluent-manager-dioxus = { version = "0.7", features = ["client"] }
unic-langid = "0.9"
```

#### 2. Define the Module

In your crate root (`lib.rs` or `main.rs`), tell the manager to scan your assets:

```rust
// a i18n.toml file must exist in the root of the crate
es_fluent_manager_dioxus::define_i18n_module!();
```

#### 3. Initialize in the Root Component

```rust
use dioxus::prelude::*;
use es_fluent::EsFluent;
use es_fluent_manager_dioxus::{
    GlobalBridgePolicy, I18nProviderConfig, ProcessGlobalLocalizationExt,
    use_i18n_provider_once,
};
use unic_langid::langid;

#[derive(EsFluent)]
enum UiMessage {
    Hello,
}

fn app() -> Element {
    let i18n = use_i18n_provider_once(
        I18nProviderConfig::new(langid!("en-US"))
            .with_global_bridge(GlobalBridgePolicy::InstallOnce),
    )
    .expect("i18n should initialize");

    rsx! {
        button {
            onclick: move |_| {
                i18n.select_language(langid!("fr"))
                    .expect("locale switch should succeed");
            },
            "{i18n.localize_via_process_global(&UiMessage::Hello)}"
        }
    }
}
```

Prefer `localize_id(...)`, `try_localize_id(...)`, `localize_in_domain(...)`, and `try_localize_in_domain(...)` when a lookup must go directly through the current `DioxusI18n` context.

Use `ProcessGlobalLocalizationExt::localize_via_process_global(...)` or `use_process_global_localized(...)` in render code only when you explicitly install a `GlobalBridgePolicy`. These helpers read the Dioxus signal before delegating to the process-global `ToFluentString` path, so they are reactive but not context-bound if another owner later calls `GlobalBridgePolicy::ReplaceExisting`. Plain `to_fluent_string()` formats through whatever process-global bridge is currently installed, but it does not subscribe the component to locale changes by itself.

`initial_language` is only read once. For prop-driven locale changes, call `i18n.select_language(...)` from an event handler or effect. Selection is best-effort by default, so `requested_language()` records the requested UI language; modules that do not support that locale are skipped. Use `select_language_strict(...)` when every discovered module must accept the locale.

The rendering-friendly lookup helpers return the message id when a translation is missing. Use `try_localize_id(...)`, `try_localize_in_domain(...)`, or the matching `ManagedI18n` methods when strict code needs to distinguish missing messages from translated values.

`use_i18n_provider_once(...)` and `use_provide_initial_i18n(...)` return `Result<DioxusI18n, DioxusInitError>` so components can render or report initialization failures. The provided `ManagedI18n` is a first-render value; do not replace it through props. Call `select_language(...)` on the returned `DioxusI18n` handle instead.

For production event handlers, prefer handling locale switch failures instead of panicking:

```rust
if let Err(error) = i18n.select_language(next) {
    eprintln!("locale switch failed: {error}");
}
```

The client hook bridge installs an `es-fluent` process-global custom localizer only when you opt in with `GlobalBridgePolicy::InstallOnce` or `GlobalBridgePolicy::ReplaceExisting`. `GlobalBridgePolicy::Disabled` is the context-only path. `InstallOnce` rejects a distinct Dioxus owner and rejects switching between the client and SSR bridges. `ReplaceExisting` is the only policy that changes bridge ownership, and should be reserved for serial tests or tightly controlled single-owner applications. The scoped guard APIs restore the previous process-global localizer on drop when no later owner replaced the guarded bridge. The bridge verifies the generation currently installed in `es-fluent` before reusing cached Dioxus ownership, so external replacement cannot be silently mistaken for same-owner reuse.

Manual client setup must call `ManagedI18n::install_client_process_global_bridge(...)`. SSR uses `SsrI18n::install_process_global_bridge(...)` instead so derived formatting resolves through the synchronous request-scoped bridge.

While the Dioxus bridge owns the global localizer, missing Dioxus messages fall back to their message id instead of falling through to an unrelated global `es-fluent` context. `ManagedI18n::raw_manager_untracked()` is available as an integration escape hatch, but using it to select languages bypasses the tracked `requested_language()` value and Dioxus rerender signal.

### SSR

The `ssr` feature is separate because there is no long-lived client signal:

```rust
use dioxus::prelude::*;
use es_fluent_manager_dioxus::ssr::SsrI18n;
use unic_langid::langid;

fn app() -> Element {
    rsx! { div { "hello" } }
}

let mut vdom = VirtualDom::new(app);
let i18n = SsrI18n::try_new_with_discovered_modules(langid!("en-US"))
    .expect("ssr i18n should initialize");

let html = i18n.rebuild_and_render(&mut vdom);
```

`SsrI18n` currently targets synchronous `dioxus::ssr` rendering helpers. It does not yet wrap the higher-level `dioxus-server` fullstack router pipeline. The default constructor installs the thread-local bridge idempotently, so SSR servers can construct request-scoped `SsrI18n` values repeatedly. If you prefer an explicit startup step, call `SsrI18n::install_process_global_bridge(...)` once before serving requests. Use `rebuild_and_render(...)` for the common path where localization can happen during the Dioxus rebuild pass. The lower-level `render(&VirtualDom)` method only scopes the final SSR serialization step and assumes the virtual DOM was already rebuilt inside `with_sync_thread_local_manager(...)`.

Do not hold `with_sync_thread_local_manager(...)` scopes across `.await`, spawned tasks, streaming render callbacks, or fullstack server boundaries. The manager scope is thread-local and synchronous. If SSR localization is called outside an `SsrI18n` scope, the bridge returns the message id instead of falling back to unrelated global localization state.

When client and SSR features are enabled in the same binary, only one bridge may own the process-global custom localizer at a time. A second owner receives `DioxusGlobalLocalizerError::OwnerConflict` unless it uses `GlobalBridgePolicy::ReplaceExisting` deliberately.

---

## Bevy Manager (`es-fluent-manager-bevy`)

Seamless [Bevy](https://bevyengine.org/) integration for `es-fluent`. This plugin connects type-safe localization with Bevy's ECS and Asset system, allowing `#[derive(EsFluent)]` types to serve as components that automatically update when the language changes.

### Features

- **Asset Loading**: Loads `.ftl` files via Bevy's `AssetServer`.
- **Hot Reloading**: Supports hot-reloading of translations during development.
- **Reactive UI**: The `FluentText` component automatically refreshes text when the locale changes.
- **Global Hook Ownership**: Can either let Bevy own `es-fluent`'s process-global localization bridge or fail fast when another integration already installed one.

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

`I18nPlugin` still installs the bridge that makes `#[derive(EsFluent)]` work
inside Bevy, but it now defaults to
`GlobalLocalizerMode::ErrorIfAlreadySet`. That keeps startup fail-fast if
another integration already owns the process-global localization bridge.

If your Bevy app intentionally owns that hook and should override any previous
registration, opt in explicitly:

```rust
use es_fluent_manager_bevy::{GlobalLocalizerMode, I18nPlugin};

App::new().add_plugins(
    I18nPlugin::with_language(langid!("en-US"))
        .with_global_localizer_mode(GlobalLocalizerMode::ReplaceExisting),
);
```

Plugin startup also uses strict module discovery, so invalid or duplicate i18n
module registrations fail the app boot instead of being normalized silently.
Failed hot reloads or locale switches keep the last accepted locale active
instead of publishing a broken update.

Use `RequestedLanguageId` to read the latest user intent and `ActiveLanguageId`
to read the currently published locale. `LocaleChangedEvent` refers to
`ActiveLanguageId`, not merely the latest request.

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

Use the `FluentText` component wrapper for any type that implements `ToFluentString` (which `#[derive(EsFluent)]` provides).

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

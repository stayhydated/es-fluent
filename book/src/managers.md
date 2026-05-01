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

Prefer a library-reachable module, usually `src/i18n.rs` declared from
`src/lib.rs`, so `cargo es-fluent generate` can discover localizable types from
the library target:

```rust
// a i18n.toml file must exist in the root of the crate
es_fluent_manager_embedded::define_i18n_module!();
```

Putting the module macro only in `src/main.rs` is runtime-only. It is safe only
when derived message types are still reachable from a library target, or when
you accept that binary-only derived types are not discovered by the CLI.

#### 2. Initialize & Use

In your application entry point:

```rust
use es_fluent::{EsFluent, EsFluentLabel};
use es_fluent_manager_embedded::EmbeddedI18n;
use unic_langid::langid;

#[derive(EsFluent, EsFluentLabel)]
#[fluent_label(origin)]
enum MyMessage {
    Hello { name: String },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let i18n = EmbeddedI18n::try_new_with_language(langid!("en"))?;

    let msg = MyMessage::Hello { name: "World".to_string() };
    println!("{}", i18n.localize_message(&msg));

    Ok(())
}
```

For types that derive `EsFluentLabel`, pass the same explicit context to
`localize_label(...)`:

```rust
use es_fluent::FluentLabel as _;

let title = MyMessage::localize_label(&i18n);
```

If you have a [Language Enum](language_enum.md), you can pass it directly since it implements `Into<LanguageIdentifier>`:

```rust
let i18n = es_fluent_manager_embedded::EmbeddedI18n::try_new_with_language(Languages::En)?;
```

If the language is not known during startup, create the context first and switch
later with `select_language(...)`:

```rust
let i18n = es_fluent_manager_embedded::EmbeddedI18n::try_new()?;
i18n.select_language(Languages::FrFr)?;
```

Before a language is selected, raw lookup returns `None`. Typed
`localize_message(...)` uses its display fallback and returns the message ID for
missing messages until `select_language(...)` succeeds.

`select_language(...)` returns an error if no discovered module can serve the
requested locale, or if a supported locale's resources would build a broken
Fluent bundle. When some modules support the requested locale and others do
not, the default switch keeps the supporting modules active. Failed switches
keep the previous ready locale active.

When a locale has only some of a module's files, the available files can still
activate and missing messages fall back through the ICU4X locale fallback chain.
Utility modules such as localized language-name display follow successful
switches but do not make an otherwise unsupported locale count as supported.

Use `try_new_with_language_strict(...)` during startup or
`select_language_strict(...)` at runtime when every discovered module must
support the requested locale for selection to succeed.

`EmbeddedI18n` clones are cheap shared handles. Calling
`select_language(...)` through one clone changes the active language observed
by the other clones. Construct a separate `EmbeddedI18n` value when you need
isolated language state.

`EmbeddedI18n` intentionally exposes enum-first `localize_message(...)` for application lookup. It also implements `FluentLocalizer` so generated labels and integration code can resolve through the same explicit context.

For custom runtime integrations, `es-fluent-manager-core` exposes the same
strict discovery behavior through `FluentManager`. Construction does not select
a language, so select the initial language before handing the manager to
integration code:

```toml
[dependencies]
es-fluent-manager-core = "0.16"
```

```rust
use es_fluent_manager_core::FluentManager;
use unic_langid::langid;

let manager = FluentManager::try_new_with_discovered_modules()?;
manager.select_language(&langid!("en"))?;
// Use concrete manager crates for application-facing typed lookup.
```

Most applications should prefer a concrete manager crate instead of wiring a raw
`FluentManager` into application state manually. `FluentManager` remains a
low-level integration point; import `es_fluent::FluentLocalizerExt as _` if
custom integration code needs generic `localize_message(...)` or fallible
`try_localize_message(...)` on a raw manager. Typed rendering uses a
render-scoped lookup, so nested message arguments and the outer message use the
same active localizer set during a concurrent language switch. Most application
code should stay on derived messages and concrete manager handles.

The embedded manager also uses strict discovery and returns initialization
errors before the manager is returned:

```rust
es_fluent_manager_embedded::EmbeddedI18n::try_new_with_language(Languages::FrFr)
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
es-fluent-manager-dioxus = { version = "0.7", features = ["client"] }

# SSR
es-fluent-manager-dioxus = { version = "0.7", features = ["ssr"] }
```

The crate has no default runtime feature. The `define_i18n_module!` macro is always available.

- `client`: Dioxus provider, hook/context runtime, and signal-backed locale state for interactive rendering.
- `ssr`: request-scoped Dioxus SSR runtime with cached module discovery.

### Define the Module

Prefer a library-reachable module, usually `src/i18n.rs` declared from
`src/lib.rs`, so `cargo es-fluent generate` can discover localizable types from
the library target:

```rust
// a i18n.toml file must exist in the root of the crate
es_fluent_manager_dioxus::define_i18n_module!();
```

Putting the module macro only in `src/main.rs` is runtime-only. It is safe only
when derived message types are still reachable from a library target, or when
you accept that binary-only derived types are not discovered by the CLI.

### Client Quick Start

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

Client apps should localize through the `DioxusI18n` context provided by `I18nProvider`, `use_init_i18n(...)`, `use_init_i18n_strict(...)`, or `use_provide_i18n(...)`. Those hooks initialize once; changing the initial language, selection policy, or provided manager after the first render does not replace the installed context. Use `localize_message(...)` for typed context-bound lookup. `DioxusI18n` implements `FluentLocalizer`, so `#[derive(EsFluentLabel)]` values can call `MyType::localize_label(&i18n)` in client components. Raw string-ID lookup is not exposed as a client convenience API; keep application code on derived messages and labels. Startup selection defaults to best effort; pass `selection_policy: LanguageSelectionPolicy::Strict`, call `use_init_i18n_with_policy(..., LanguageSelectionPolicy::Strict)`, or call `use_init_i18n_strict(...)` when every discovered module must support the startup locale. Locale switches use fallible `select_language(...)` or `select_language_strict(...)`; after a manager is handed to the Dioxus provider, route language changes through those `DioxusI18n` methods so the Dioxus signal stays aligned with manager state. `ManagedI18n` clones are shared handles; language selection and requested-language updates are serialized, while localization reads use render-scoped manager snapshots so independent typed renders can run concurrently. `requested_language()` tracks the requested locale, while `peek_requested_language()` reads it without subscribing.

Dioxus localizes through explicit component or request context. Keeping lookup context-bound avoids cross-root, hot-reload, test, and SSR request leakage.

If initialization cannot complete, the hook still provides a failed context to keep hook order stable for callers that inspect the returned `Result` directly. `I18nProvider` logs that failure once per provider instance and renders `fallback` when one is supplied; without a fallback it renders children with a failed i18n context, so descendants that call `use_i18n()` receive the same initialization error. `I18nProviderStrict` is the fail-closed rendering variant: it renders fallback when one is supplied and otherwise renders an empty vnode. Strictness in the component name refers to rendering behavior; use `selection_policy: LanguageSelectionPolicy::Strict` for strict startup locale selection. Descendants can call `try_use_i18n()` to distinguish a missing provider from a failed provider. Event handlers and async tasks can call `consume_i18n()` or `try_consume_i18n()` while the Dioxus runtime is active.

### SSR Quick Start

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
    let mut dom = VirtualDom::new_with_props(
        App,
        AppProps {
            i18n: i18n.clone(),
        },
    );

    Ok(i18n.rebuild_and_render(&mut dom))
}
```

Create one `SsrI18nRuntime` during startup, then create one `SsrI18n` per request. The runtime caches the first validated module-discovery result for its lifetime, including discovery or validation failures; construct a new runtime to retry after a failed discovery. Each request creates fresh manager/localizer state so request languages remain isolated. `request(...)` uses best-effort initial language selection; use `request_strict(...)` when every discovered module must support the request locale.

The render helpers do not install context automatically; pass `SsrI18n` as a prop or call `provide_context()` from a component when using hook-based lookup.

SSR components should receive a cloned `SsrI18n` as a prop or through app-owned context and call `localize_message(...)` or `MyType::localize_label(&i18n)`. If SSR components use the Dioxus hook API, enable both `ssr` and `client` features because `SsrI18n::provide_context(...)` is compiled behind `client`, then call `i18n.provide_context()?` from an app-owned provider component.

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

Prefer a library-reachable module, usually `src/i18n.rs` declared from
`src/lib.rs`, so `cargo es-fluent generate` can discover localizable types from
the library target:

```rust
// a i18n.toml file must exist in the root of the crate
es_fluent_manager_bevy::define_i18n_module!();
```

Putting the module macro only in `src/main.rs` is runtime-only. It is safe only
when derived message types are still reachable from a library target, or when
you accept that binary-only derived types are not discovered by the CLI.

#### 2. Initialize & Use

Add the plugin to your `App`:

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

By default, `I18nPlugin` loads locales from `locales` relative to Bevy's asset
root, matching `assets_dir = "assets/locales"` in `i18n.toml`. If your Bevy
asset root is `assets` but translations live in `assets/i18n`, configure the
path explicitly:

```rust
use es_fluent_manager_bevy::{I18nPlugin, I18nPluginConfig};

app.add_plugins(I18nPlugin::with_config(
    I18nPluginConfig::new(langid!("en")).with_asset_path("i18n"),
));
```

`I18nPlugin` localizes `FluentText` components through Bevy resources and does
not install a process-wide localization hook.

#### Advanced behavior

Plugin startup also uses strict module discovery, so invalid or duplicate i18n
module registrations are reported through `I18nPluginStartupError` instead of
being normalized silently. When setup fails, the plugin skips localization
runtime setup and leaves the error resource in the app world for diagnostics.
Failed hot reloads or locale switches keep the last accepted locale active
instead of publishing a broken update. A failed hot reload records diagnostics
but keeps the previous ready cache selectable until a later rebuild succeeds.

Generated message lookup is domain-scoped. If separate domains define the same
message ID, Bevy keeps typed domain-scoped lookup available and leaves raw
unscoped lookup unavailable for the ambiguous merged locale.

Locales with only optional resources, or with missing optional resources, are
still treated as ready. They publish an empty Bevy cache instead of remaining
pending indefinitely.

Use `RequestedLanguageId` to read the latest user intent and `ActiveLanguageId`
to read the currently published locale. `LocaleChangedEvent` refers to
`ActiveLanguageId`, not merely the latest request. When a requested locale
falls back to a resolved locale, Bevy publishes the requested locale for change
events and ECS resources while using the resolved locale for ready bundle
lookup. Runtime fallback managers are best-effort: Bevy asks them to select the
requested locale first, then the resolved locale, but rejection does not block
Bevy asset-backed locale publication. Only metadata-only Bevy registrations
create Bevy asset availability; runtime localizer registrations are reserved
for the fallback manager and do not make a locale wait on Bevy asset bundles.
When attached, runtime fallback selection tells `FluentManager` that Bevy assets
have already proved application locale support, so follower-only utility modules
such as `es-fluent-lang` can be committed without making runtime-only locales
selectable. Generated embedded localizers are fallback-aware, while custom
runtime localizers should implement parent-locale fallback in
`select_language(...)` when they need it. Runtime fallback managers are attached
whenever runtime modules are discovered, even if they reject the startup locale.
A startup rejection leaves runtime localizers unselected until a later accepted
locale switch. Runtime fallback managers are used only after Bevy resolves a
locale through asset or ready-bundle availability during startup or a later
`LocaleChangeEvent`; runtime-only locales do not by themselves make a Bevy
locale switch selectable.

For direct localization inside a system, request `BevyI18n` like any other
Bevy system parameter:

```rust
use es_fluent::FluentLabel as _;
use es_fluent_manager_bevy::BevyI18n;

fn update_title(i18n: BevyI18n) {
    let title = i18n.localize_message(&UiMessage::Settings);
    // `SettingsPanel` is any type that derives `EsFluentLabel`.
    let section_title = SettingsPanel::localize_label(&i18n);
    // apply `title` to your Bevy UI, window, or gameplay state
    // use `section_title` for an `EsFluentLabel` type label
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

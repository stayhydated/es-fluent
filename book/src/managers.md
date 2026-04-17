# Runtime Managers

`es-fluent` is agnostic about how you load translations at runtime. The ecosystem provides two ready-made manager crates so you don't have to build your own asset pipeline.

| Manager                      | Best for                 | How it works                             |
| ---------------------------- | ------------------------ | ---------------------------------------- |
| `es-fluent-manager-embedded` | CLIs, TUIs, desktop apps | Compiles FTL files into the binary       |
| `es-fluent-manager-bevy`     | Bevy games and apps      | Loads FTL files via Bevy's `AssetServer` |

---

## Embedded Manager (`es-fluent-manager-embedded`)

Bundles your translations directly into the binary and exposes a global singleton. No external files needed at runtime.

### Features

- **Embedded Assets**: Compiles your FTL files into the binary (using `rust-embed` under the hood).
- **Global Access**: Once initialized, you can call `to_fluent_string()` anywhere in your code without passing context around.
- **Thread Safe**: Uses `OnceLock` and atomic swaps for safe concurrent access.

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

For larger apps that want explicit control over the shared context, manager-core
is strict by default:

```rust
use es_fluent::try_set_context;
use es_fluent_manager_core::FluentManager;

let manager = FluentManager::new_with_discovered_modules();
try_set_context(manager).expect("global context should only be installed once");
```

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

## Bevy Manager (`es-fluent-manager-bevy`)

Seamless [Bevy](https://bevyengine.org/) integration for `es-fluent`. This plugin connects type-safe localization with Bevy's ECS and Asset system, allowing `#[derive(EsFluent)]` types to serve as components that automatically update when the language changes.

### Features

- **Asset Loading**: Loads `.ftl` files via Bevy's `AssetServer`.
- **Hot Reloading**: Supports hot-reloading of translations during development.
- **Reactive UI**: The `FluentText` component automatically refreshes text when the locale changes.
- **Global Hook Ownership**: Can either let Bevy own `es-fluent`'s process-global localizer hook or fail fast when another integration already installed one.

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
another integration already owns the process-global `es_fluent::localize`
hook.

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
Malformed or conflicting Fluent bundle rebuilds are also rejected during asset
hot reloads and locale switches. When that happens, Bevy keeps the last ready
bundle active instead of publishing partial translations, and domain-scoped
lookups stay aligned with the accepted resource set.

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

# Bevy manager

Use `es-fluent-manager-bevy` to connect typed messages to Bevy ECS,
assets, and reactive UI text.

## Add the manager

~~~toml
[dependencies]
bevy = "0.20"
es-fluent = "0.21"
es-fluent-manager-bevy = "0.21"
unic-langid = "0.9"
~~~

The manager enables the Bevy asset, text, and UI APIs it uses. Select rendering
and window backend features in the application's Bevy dependency. The maintained
UI demo enables Bevy's `ui` feature with `default-features = false`.

Register package resources from a library-reachable module:

~~~rust
// src/i18n.rs
es_fluent_manager_bevy::define_i18n_module!();
~~~

## Install the plugin

~~~rust
use bevy::prelude::*;
use es_fluent_manager_bevy::I18nPlugin;
use unic_langid::langid;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(I18nPlugin::with_language(langid!("en")))
        .run();
}
~~~

Generated manager modules contribute their configured resources. Link every
owner library in a multi-crate application; the host does not copy dependency
FTL.

Fallback-locale message values are compile-time checked through
`es-fluent-build`. Set `missing_message_policy = "fallback-str"` in the owning
package's `i18n.toml` when `BevyI18n` and `FluentText<T>` should render
snake_case source names after locale fallback is exhausted.

## Localize UI text

Derive `BevyFluentText` for message types that should refresh when
the locale changes, then wrap values in `FluentText<T>`:

~~~rust
use bevy::prelude::*;
use es_fluent::EsFluent;
use es_fluent_manager_bevy::{BevyFluentText, FluentText};

#[derive(BevyFluentText, EsFluent)]
enum UiMessage {
    StartGame,
    Settings,
}

fn spawn_menu(mut commands: Commands) {
    commands.spawn((
        FluentText::new(UiMessage::StartGame),
        Text::new(""),
    ));
}
~~~

Only a type used directly as `FluentText<T>` needs registration.
Nested message fields are formatted when the parent value refreshes.
The wrapper is the ECS component; message values only need `FluentMessage`,
`Send`, `Sync`, and a `'static` lifetime for registration. Cloning a wrapper
requires a message value that implements `Clone`.

If a named struct field or named enum variant field depends on the requested
locale, mark it with `#[locale]`. Its type must implement
`TryFrom<&LanguageIdentifier>`. The derive generates locale refresh
behavior and registration.

For an external type that cannot derive `BevyFluentText`, register it
manually with `register_fluent_text::<T>()`.

## Localize in systems

Request `BevyI18n` as a system parameter:

~~~rust
use es_fluent_manager_bevy::BevyI18n;

fn update_title(i18n: BevyI18n) {
    let title = i18n.localize_message(&UiMessage::Settings);
    // Apply the title to application state.
    let _ = title;
}
~~~

Use `RequestedLanguageId` for the latest user request and
`ActiveLanguageId` for the published locale. Failed asset reloads or
locale switches keep the last accepted locale active.

Send `LocaleChangeMessage` to request a locale. Read `LocaleChangedMessage`
for successfully published locale changes and accepted current-locale reloads.

~~~rust
use bevy::prelude::*;
use es_fluent_manager_bevy::LocaleChangeMessage;
use unic_langid::langid;

fn select_french(mut requests: MessageWriter<LocaleChangeMessage>) {
    requests.write(LocaleChangeMessage(langid!("fr-FR")));
}
~~~

## Order application systems

The plugin labels localization phases with `I18nSet`. Use Bevy's
`.before(...)` and `.after(...)` APIs when an application
system must run around locale synchronization or text refresh:

Send locale requests in `Update` before `I18nSet::LocaleChange` to apply them in
the same frame. Update message values in `Update`; automatic text refresh runs
in `PostUpdate` before Bevy's text change detection and UI content measurement.
Changed output sends Bevy `RequestRedraw` messages for the host's window backend.

~~~rust
use bevy::prelude::*;
use es_fluent_manager_bevy::I18nSet;

fn persist_locale() {}
fn update_window_title() {}

app.add_systems(Update, select_french.before(I18nSet::LocaleChange));
app.add_systems(Update, persist_locale.after(I18nSet::LocaleSync));
app.add_systems(PostUpdate, update_window_title.after(I18nSet::TextUpdate));
~~~

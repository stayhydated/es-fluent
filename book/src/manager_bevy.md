# Bevy manager

Use `es-fluent-manager-bevy` to connect typed messages to Bevy ECS,
assets, and reactive UI text.

## Add the manager

~~~toml
[dependencies]
bevy = "0.19"
es-fluent = "0.18"
es-fluent-manager-bevy = "0.19"
unic-langid = "0.9"
~~~

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

## Localize UI text

Derive `BevyFluentText` for component types that should refresh when
the locale changes, then wrap values in `FluentText<T>`:

~~~rust
use bevy::prelude::*;
use es_fluent::EsFluent;
use es_fluent_manager_bevy::{BevyFluentText, FluentText};

#[derive(BevyFluentText, Clone, Component, EsFluent)]
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

## Order application systems

The plugin labels localization phases with `I18nSet`. Use Bevy's
`.before(...)` and `.after(...)` APIs when an application
system must run around locale synchronization or text refresh:

~~~rust
use bevy::prelude::*;
use es_fluent_manager_bevy::I18nSet;

fn persist_locale() {}
fn update_window_title() {}

app.add_systems(Update, persist_locale.after(I18nSet::LocaleSync));
app.add_systems(PostUpdate, update_window_title.after(I18nSet::TextUpdate));
~~~

use bevy::{color::palettes::basic::*, input_focus::InputFocus, prelude::*, winit::WinitSettings};
use es_fluent::EsFluent;
use es_fluent_manager_bevy::{
    CurrentLanguageId, FluentText, FluentTextRegistration as _, I18nPlugin, LocaleChangeEvent,
};
use example_shared_lib::{ButtonState, Languages};

#[allow(unused_imports)]
#[allow(clippy::single_component_path_imports)]
use bevy_example;

#[derive(Clone, Component, Copy, Debug, EsFluent)]
pub enum KbKeys {
    T,
}

#[derive(Clone, Component, Copy, Debug, EsFluent)]
pub enum BevyScreenMessages {
    ToggleLanguageHint {
        key: KbKeys,
        current_language: Languages,
    },
}

impl es_fluent_manager_bevy::RefreshForLocale for BevyScreenMessages {
    fn refresh_for_locale(&mut self, lang: &unic_langid::LanguageIdentifier) {
        match self {
            BevyScreenMessages::ToggleLanguageHint {
                key: _,
                current_language,
            } => {
                *current_language = Languages::from(lang);
            },
        }
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(AssetPlugin {
        watch_for_changes_override: Some(true),
        file_path: "../assets".to_string(),
        ..default()
    }))
    .insert_resource(WinitSettings::desktop_app())
    .add_plugins(I18nPlugin::with_language(Languages::default().into()))
    .init_resource::<InputFocus>();

    app.register_fluent_text::<ButtonState>()
        .register_fluent_text_from_locale::<BevyScreenMessages>();

    app.add_systems(Startup, setup)
        .add_systems(PostUpdate, (button_system, locale_change_system))
        .run();
}

fn locale_change_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut locale_change_events: MessageWriter<LocaleChangeEvent>,
    current_language: Res<CurrentLanguageId>,
) {
    if keyboard.just_pressed(KeyCode::KeyT) {
        locale_change_events.write(LocaleChangeEvent(
            Languages::from(&current_language.0).next().into(),
        ));
    }
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

fn button_system(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut FluentText<ButtonState>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color, mut border_color, mut localized) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                localized.value = ButtonState::Pressed;
                *color = PRESSED_BUTTON.into();
                *border_color = BorderColor::all(RED);
            },
            Interaction::Hovered => {
                localized.value = ButtonState::Hovered;
                *color = HOVERED_BUTTON.into();
                *border_color = BorderColor::all(Color::WHITE);
            },
            Interaction::None => {
                localized.value = ButtonState::Normal;
                *color = NORMAL_BUTTON.into();
                *border_color = BorderColor::all(Color::BLACK);
            },
        }
    }
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.spawn(button(&assets));
}

fn button(asset_server: &AssetServer) -> impl Bundle + use<> {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        children![
            (
                (Button, FluentText::new(ButtonState::Normal),),
                Node {
                    width: Val::Px(150.0),
                    height: Val::Px(65.0),
                    border: UiRect::all(Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
                BorderColor::all(Color::BLACK),
                BorderRadius::MAX,
                BackgroundColor(NORMAL_BUTTON),
                children![(
                    Text::new(""),
                    TextFont {
                        font_size: 33.0,
                        font: asset_server.load("fonts/NotoSansSC-Bold.ttf"),
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    TextShadow::default(),
                )]
            ),
            (
                FluentText::new(BevyScreenMessages::ToggleLanguageHint {
                    key: KbKeys::T,
                    current_language: Languages::default(),
                }),
                Text::new(""),
                TextFont {
                    font_size: 20.0,
                    font: asset_server.load("fonts/NotoSansSC-Bold.ttf"),
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                Node {
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
            )
        ],
    )
}

pub use bevy_example::fl;

use bevy::{color::palettes::basic::*, prelude::*, winit::WinitSettings};
use es_fluent::{EsFluent, ToFluentString};
use es_fluent_manager_bevy;
use strum::{Display, EnumIter, IntoEnumIterator};

#[derive(EsFluent)]
pub enum ButtonState {
    Normal,
    Hovered,
    Pressed,
}

#[derive(EsFluent)]
pub enum ScreenMessages {
    ToggleLanguageHint { current_language: Languages },
}

#[derive(Clone, Copy, Default, Display, EnumIter, EsFluent)]
pub enum Languages {
    #[strum(serialize = "en")]
    #[default]
    English,
    #[strum(serialize = "fr")]
    French,
    #[strum(serialize = "cn")]
    Chinese,
}

#[derive(Component)]
struct LocalizedButton {
    current_state: ButtonState,
}

#[derive(Component)]
struct LanguageHintText;

#[derive(Component)]
struct ButtonText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: "../assets".to_string(),
            ..default()
        }))
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, (setup, initialize_ui_text_system.after(setup)))
        .add_systems(
            Update,
            (
                button_system,
                update_button_text_system,
                example_locale_change_system,
            ),
        )
        .add_plugins(es_fluent_manager_bevy::I18nPlugin {
            default_languages: Languages::iter().map(|l| l.to_string()).collect(),
        })
        .add_observer(update_ui_on_locale_change_system)
        .run();
}

fn example_locale_change_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut locale_change_events: EventWriter<es_fluent_manager_bevy::LocaleChangeEvent>,
) {
    // Change locale when pressing 'T' key
    if keyboard.just_pressed(KeyCode::KeyT) {
        let current_locale = es_fluent_manager_bevy::get_current_locale()
            .unwrap_or(Languages::default().to_string());

        let languages: Vec<Languages> = Languages::iter().collect();
        let current_index = languages
            .iter()
            .position(|lang| lang.to_string() == current_locale)
            .unwrap_or(0);
        let next_index = (current_index + 1) % languages.len();
        let next_locale = languages[next_index];

        es_fluent_manager_bevy::change_locale(&next_locale.to_string(), &mut locale_change_events);
    }
}

fn update_ui_on_locale_change_system(
    trigger: Trigger<es_fluent_manager_bevy::LocaleChangedEvent>,
    button_query: Query<&LocalizedButton>,
    mut text_queries: ParamSet<(
        Query<&mut Text, With<ButtonText>>,
        Query<&mut Text, With<LanguageHintText>>,
    )>,
) {
    let event = trigger.event();
    info!("UI updating for new locale: {}", event.locale);

    if let Ok(button) = button_query.single() {
        if let Ok(mut text) = text_queries.p0().single_mut() {
            *text = Text::from(button.current_state.to_fluent_string());
        }
    }

    let current_language = Languages::iter()
        .find(|lang| lang.to_string() == event.locale)
        .unwrap_or_default();

    if let Ok(mut text) = text_queries.p1().single_mut() {
        *text =
            Text::from(ScreenMessages::ToggleLanguageHint { current_language }.to_fluent_string());
    }
}

fn initialize_ui_text_system(
    button_query: Query<&LocalizedButton>,
    mut text_queries: ParamSet<(
        Query<&mut Text, With<ButtonText>>,
        Query<&mut Text, With<LanguageHintText>>,
    )>,
) {
    if let Ok(button) = button_query.single() {
        if let Ok(mut text) = text_queries.p0().single_mut() {
            *text = Text::from(button.current_state.to_fluent_string());
        }
    }

    if let Ok(mut text) = text_queries.p1().single_mut() {
        *text = Text::from(
            ScreenMessages::ToggleLanguageHint {
                current_language: Languages::default(),
            }
            .to_fluent_string(),
        );
    }
}

fn update_button_text_system(
    button_query: Query<&LocalizedButton, Changed<LocalizedButton>>,
    mut text_query: Query<&mut Text, With<ButtonText>>,
) {
    if let Ok(button) = button_query.single() {
        if let Ok(mut text) = text_query.single_mut() {
            *text = Text::from(button.current_state.to_fluent_string());
        }
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
            &mut LocalizedButton,
        ),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color, mut border_color, mut localized_button) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                localized_button.current_state = ButtonState::Pressed;
                *color = PRESSED_BUTTON.into();
                border_color.0 = RED.into();
            },
            Interaction::Hovered => {
                localized_button.current_state = ButtonState::Hovered;
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            },
            Interaction::None => {
                localized_button.current_state = ButtonState::Normal;
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
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
                (
                    Button,
                    LocalizedButton {
                        current_state: ButtonState::Normal,
                    },
                ),
                Node {
                    width: Val::Px(150.0),
                    height: Val::Px(65.0),
                    border: UiRect::all(Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
                BorderColor(Color::BLACK),
                BorderRadius::MAX,
                BackgroundColor(NORMAL_BUTTON),
                children![(
                    ButtonText,
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
                LanguageHintText,
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

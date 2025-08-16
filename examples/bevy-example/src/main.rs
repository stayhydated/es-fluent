pub use bevy_example::fl;

use strum::{Display, EnumIter, IntoEnumIterator};
use bevy::{color::palettes::basic::*, prelude::*, winit::WinitSettings};
use es_fluent::{EsFluent, ToFluentString};
use es_fluent_manager_bevy;

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

#[derive(EsFluent, EnumIter, Display, Clone, Copy, Default)]
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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: "../assets".to_string(),
            ..default()
        }))
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(Update, button_system)
        .add_systems(Update, update_button_text_system)
        .add_systems(Update, initialize_button_text_system)
        .add_plugins(es_fluent_manager_bevy::I18nPlugin {
            default_languages: Languages::iter().map(|l| l.to_string()).collect(),
        })
        .add_systems(Update, example_locale_change_system)
        .add_systems(Update, update_ui_on_locale_change_system)
        .run();
}

fn example_locale_change_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut locale_change_events: EventWriter<es_fluent_manager_bevy::LocaleChangeEvent>,
) {
    // Change locale when pressing 'L' key
    if keyboard.just_pressed(KeyCode::KeyL) {
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
    mut locale_changed_events: EventReader<es_fluent_manager_bevy::LocaleChangedEvent>,
    button_query: Query<(&Children, &LocalizedButton), With<Button>>,
    mut text_query: Query<&mut Text>,
) {
    for event in locale_changed_events.read() {
        info!("UI updating for new locale: {}", event.locale);

        for (children, localized_button) in button_query.iter() {
            if let Ok(mut text) = text_query.get_mut(children[0]) {
                **text = localized_button.current_state.to_fluent_string();
            }
        }
    }
}

fn update_button_text_system(
    button_query: Query<(&Children, &LocalizedButton), (With<Button>, Changed<LocalizedButton>)>,
    mut text_query: Query<&mut Text>,
) {
    for (children, localized_button) in button_query.iter() {
        if let Ok(mut text) = text_query.get_mut(children[0]) {
            **text = localized_button.current_state.to_fluent_string();
        }
    }
}

fn initialize_button_text_system(
    button_query: Query<(&Children, &LocalizedButton), With<Button>>,
    mut text_query: Query<&mut Text>,
    mut initialized: Local<bool>,
) {
    if !*initialized {
        for (children, localized_button) in button_query.iter() {
            if let Ok(mut text) = text_query.get_mut(children[0]) {
                **text = localized_button.current_state.to_fluent_string();
                *initialized = true;
            }
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
    // ui camera
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
            ..default()
        },
        children![(
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
                // horizontally center child text
                justify_content: JustifyContent::Center,
                // vertically center child text
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor(Color::BLACK),
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
        )],
    )
}

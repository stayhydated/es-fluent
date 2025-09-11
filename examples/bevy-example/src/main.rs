use bevy::{color::palettes::basic::*, prelude::*, winit::WinitSettings};
use es_fluent::{EsFluent, ToFluentString};
use es_fluent_manager_bevy::{
    EsFluentBevyPlugin, EsFluentText, EsFluentTypeRegistration, I18nAssets, I18nPlugin,
    LocaleChangeEvent, LocaleChangedEvent,
};
use strum::{Display, EnumIter, IntoEnumIterator};
use unic_langid::{LanguageIdentifier, langid};

es_fluent_manager_bevy::define_i18n_module!();

#[derive(Clone, Copy, Debug, EsFluent, Component, PartialEq)]
pub enum ButtonState {
    Normal,
    Hovered,
    Pressed,
}

#[derive(Clone, Copy, Debug, EsFluent, Component)]
pub enum ScreenMessages {
    ToggleLanguageHint { current_language: Languages },
}

#[derive(Clone, Copy, Debug, Default, Display, EnumIter, EsFluent, PartialEq, Component)]
pub enum Languages {
    #[strum(serialize = "en")]
    #[default]
    English,
    #[strum(serialize = "fr")]
    French,
    #[strum(serialize = "cn")]
    Chinese,
}

impl From<Languages> for LanguageIdentifier {
    fn from(val: Languages) -> Self {
        match val {
            Languages::English => langid!("en"),
            Languages::French => langid!("fr"),
            Languages::Chinese => langid!("cn"),
        }
    }
}

#[derive(Resource)]
struct CurrentLanguage(Languages);

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, States)]
enum AppState {
    #[default]
    Loading,
    Ready,
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
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(AssetPlugin {
        watch_for_changes_override: Some(true),
        file_path: "../assets".to_string(),
        ..default()
    }))
    .insert_resource(WinitSettings::desktop_app())
    .insert_resource(CurrentLanguage(Languages::default()))
    .init_state::<AppState>()
    .add_plugins(I18nPlugin::with_language(Languages::default().into()))
    .add_plugins(EsFluentBevyPlugin);

    app.register_es_fluent_parent_type::<ButtonState>()
        .register_es_fluent_type::<ScreenMessages>();

    app.add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                check_assets_ready_system.run_if(in_state(AppState::Loading)),
                button_system.run_if(in_state(AppState::Ready)),
                example_locale_change_system.run_if(in_state(AppState::Ready)),
                update_ui_on_locale_change_system.run_if(in_state(AppState::Ready)),
            ),
        )
        .add_systems(OnEnter(AppState::Ready), initialize_ui_system)
        .run();
}

fn example_locale_change_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut locale_change_events: EventWriter<LocaleChangeEvent>,
    mut current_language: ResMut<CurrentLanguage>,
) {
    if keyboard.just_pressed(KeyCode::KeyT) {
        let languages: Vec<Languages> = Languages::iter().collect();
        let current_index = languages
            .iter()
            .position(|&lang| lang == current_language.0)
            .unwrap_or(0);
        let next_index = (current_index + 1) % languages.len();
        let next_language = languages[next_index];

        current_language.0 = next_language;
        locale_change_events.write(LocaleChangeEvent(next_language.into()));
    }
}

fn update_ui_on_locale_change_system(
    mut events: EventReader<LocaleChangedEvent>,
    mut text_query: Query<&mut Text, With<LanguageHintText>>,
    current_language: Res<CurrentLanguage>,
) {
    for event in events.read() {
        info!("UI updating for new locale: {}", event.0);

        if let Ok(mut text) = text_query.single_mut() {
            *text = Text::from(
                ScreenMessages::ToggleLanguageHint {
                    current_language: current_language.0,
                }
                .to_fluent_string(),
            );
        }
    }
}

fn check_assets_ready_system(
    i18n_assets: Res<I18nAssets>,
    current_language: Res<CurrentLanguage>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if i18n_assets.is_language_loaded(&current_language.0.into()) {
        info!("Assets ready, transitioning to Ready state");
        next_state.set(AppState::Ready);
    }
}

fn initialize_ui_system(
    mut text_queries: ParamSet<(
        Query<&mut Text, With<ButtonText>>,
        Query<&mut Text, With<LanguageHintText>>,
    )>,
    current_language: Res<CurrentLanguage>,
) {
    info!("Initializing UI text on app ready");

    if let Ok(mut text) = text_queries.p0().single_mut() {
        *text = Text::from(ButtonState::Normal.to_fluent_string());
    }

    if let Ok(mut text) = text_queries.p1().single_mut() {
        *text = Text::from(
            ScreenMessages::ToggleLanguageHint {
                current_language: current_language.0,
            }
            .to_fluent_string(),
        );
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
            &mut EsFluentText<ButtonState>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color, mut border_color, mut localized_button, mut es_fluent_text) in
        &mut interaction_query
    {
        let previous_state = localized_button.current_state;
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

        // If the state changed, update the EsFluentText component
        if localized_button.current_state != previous_state {
            es_fluent_text.value = localized_button.current_state;
            es_fluent_text.set_changed();
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
                    EsFluentText::new(ButtonState::Normal),
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

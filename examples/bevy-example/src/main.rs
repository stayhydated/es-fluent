use bevy::{color::palettes::basic::*, input_focus::InputFocus, prelude::*, winit::WinitSettings};
use es_fluent::EsFluent;
use es_fluent_manager_bevy::{
    FluentText, FluentTextRegistration as _, I18nAssets, I18nPlugin, LocaleChangeEvent,
};
use strum::{EnumIter, IntoEnumIterator as _};
use unic_langid::{LanguageIdentifier, langid};

es_fluent_manager_bevy::define_i18n_module!();

#[derive(Clone, Component, Copy, Debug, EsFluent, PartialEq)]
pub enum ButtonState {
    Normal,
    Hovered,
    Pressed,
}

#[derive(Clone, Component, Copy, Debug, EsFluent)]
pub enum ScreenMessages {
    ToggleLanguageHint { current_language: Languages },
}

#[derive(Resource)]
struct CurrentLanguage(Languages);

#[derive(Clone, Component, Copy, Debug, Default, EnumIter, EsFluent, PartialEq)]
pub enum Languages {
    #[default]
    English,
    French,
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

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, States)]
enum AppState {
    #[default]
    Loading,
    Ready,
}

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
    .add_plugins(I18nPlugin::with_language(Languages::default().into()));

    app.register_fluent_text::<ButtonState>()
        .register_fluent_text::<ScreenMessages>();

    app.add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                check_assets_ready_system.run_if(in_state(AppState::Loading)),
                button_system.run_if(in_state(AppState::Ready)),
                example_locale_change_system.run_if(in_state(AppState::Ready)),
            ),
        )
        .add_systems(OnEnter(AppState::Ready), initialize_ui_system)
        .run();
}

fn example_locale_change_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut locale_change_events: MessageWriter<LocaleChangeEvent>,
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
    mut localized_queries: ParamSet<(
        Query<&mut FluentText<ButtonState>>,
        Query<&mut FluentText<ScreenMessages>>,
    )>,
    current_language: Res<CurrentLanguage>,
) {
    info!("Initializing UI text on app ready");

    if let Ok(mut localized) = localized_queries.p0().single_mut() {
        localized.value = ButtonState::Normal;
    }

    if let Ok(mut localized) = localized_queries.p1().single_mut() {
        localized.value = ScreenMessages::ToggleLanguageHint {
            current_language: current_language.0,
        };
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
                FluentText::new(ScreenMessages::ToggleLanguageHint {
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

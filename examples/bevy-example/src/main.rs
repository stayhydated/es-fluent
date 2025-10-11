use bevy::{color::palettes::basic::*, input_focus::InputFocus, prelude::*, winit::WinitSettings};
use es_fluent::EsFluent;
use es_fluent_manager_bevy::{
    FluentText, FluentTextRegistration as _, I18nPlugin, LocaleChangeEvent,
};
use shared_lib::{ButtonState, CurrentLanguage, Languages};
use strum::IntoEnumIterator as _;

es_fluent_manager_bevy::define_i18n_module!();

#[derive(Clone, Copy, Debug, EsFluent, Component)]
pub enum BevyScreenMessages {
    ToggleLanguageHint { current_language: Languages },
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
    .add_plugins(I18nPlugin::with_language(Languages::default().into()))
    .init_resource::<InputFocus>();

    app.register_fluent_text::<ButtonState>()
        .register_fluent_text::<BevyScreenMessages>();

    app.add_systems(Startup, setup)
        .add_systems(PostUpdate, (button_system, locale_change_system))
        .run();
}

fn locale_change_system(
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

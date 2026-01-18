use bevy::{color::palettes::basic::*, input_focus::InputFocus, prelude::*, winit::WinitSettings};
use bevy_example::{BevyScreenMessages, KbKeys};
use es_fluent_manager_bevy::{CurrentLanguageId, FluentText, I18nPlugin, LocaleChangeEvent};
use example_shared_lib::{ButtonState, Languages};

// bring es_fluent_manager_bevy::define_i18n_module!(); in scope
#[allow(unused_imports)]
#[allow(clippy::single_component_path_imports)]
use bevy_example;

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
    mut input_focus: ResMut<InputFocus>,
    mut interaction_query: Query<
        (
            Entity,
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut Button,
            &mut FluentText<ButtonState>,
        ),
        Changed<Interaction>,
    >,
) {
    for (entity, interaction, mut color, mut border_color, mut button, mut localized) in
        &mut interaction_query
    {
        match *interaction {
            Interaction::Pressed => {
                input_focus.set(entity);
                localized.value = ButtonState::Pressed;
                *color = PRESSED_BUTTON.into();
                *border_color = BorderColor::all(RED);
                button.set_changed();
            },
            Interaction::Hovered => {
                input_focus.set(entity);
                localized.value = ButtonState::Hovered;
                *color = HOVERED_BUTTON.into();
                *border_color = BorderColor::all(Color::WHITE);
                button.set_changed();
            },
            Interaction::None => {
                input_focus.clear();
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

fn button(asset_server: &AssetServer) -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        children![
            (
                Button,
                FluentText::new(ButtonState::Normal),
                Node {
                    width: px(150),
                    height: px(65),
                    border: UiRect::all(px(5)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(px(20)),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BorderColor::all(Color::BLACK),
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

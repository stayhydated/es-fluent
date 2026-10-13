use bevy::{
    color::palettes::basic::*,
    input_focus::{FocusCause, InputFocus},
    picking::hover::Hovered,
    prelude::*,
    ui::Pressed,
    ui_widgets::{Activate, Button, observe},
    window::{Window, WindowPlugin},
    winit::WinitSettings,
};
use es_fluent::EsFluent;
use es_fluent_manager_bevy::{
    BevyFluentText, FluentText, I18nPlugin, I18nPluginConfig, I18nSet, LocaleChangeMessage,
    RequestedLanguageId,
};
use example_shared_lib::{ButtonState, Languages};

es_fluent_manager_bevy::define_i18n_module!();

#[derive(Clone, Component, Copy, Debug, EsFluent)]
#[fluent(namespace = "ui")]
pub enum KbKeys {
    T,
}

#[derive(BevyFluentText, Clone, Component, Copy, Debug, EsFluent)]
#[fluent(namespace = "ui")]
pub enum BevyScreenMessages {
    ToggleLanguageHint {
        key: KbKeys,
        #[locale]
        current_language: Languages,
    },
    ChangeLocaleButton,
}

#[derive(Component)]
struct LocaleButton;

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

pub fn run() {
    let mut app = App::new();

    let default_plugins = DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            fit_canvas_to_parent: true,
            ..default()
        }),
        ..default()
    });

    #[cfg(not(target_arch = "wasm32"))]
    app.add_plugins(default_plugins.set(AssetPlugin {
        watch_for_changes_override: Some(cfg!(debug_assertions)),
        file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/assets").to_string(),
        ..default()
    }));

    #[cfg(target_arch = "wasm32")]
    app.add_plugins(default_plugins.set(AssetPlugin {
        file_path: "assets".to_string(),
        meta_check: bevy::asset::AssetMetaCheck::Never,
        ..default()
    }));

    #[cfg(not(target_arch = "wasm32"))]
    app.insert_resource(WinitSettings::desktop_app());

    #[cfg(target_arch = "wasm32")]
    app.insert_resource(WinitSettings::game());

    app.add_plugins(I18nPlugin::with_config(
        I18nPluginConfig::new(Languages::default().into()).with_asset_path("i18n"),
    ))
    .init_resource::<InputFocus>()
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            button_system,
            locale_change_system.before(I18nSet::LocaleChange),
            locale_button_appearance,
        ),
    )
    .run();
}

fn locale_change_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut locale_change_messages: MessageWriter<LocaleChangeMessage>,
    requested_language: Res<RequestedLanguageId>,
) {
    if keyboard.just_pressed(KeyCode::KeyT)
        && let Ok(lang) = Languages::try_from(&requested_language.0)
    {
        locale_change_messages.write(LocaleChangeMessage(lang.next().into()));
    }
}

fn button_system(
    mut input_focus: ResMut<InputFocus>,
    mut interaction_query: Query<
        (
            Entity,
            &Hovered,
            Has<Pressed>,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut FluentText<ButtonState>,
        ),
        (With<Button>, Without<LocaleButton>),
    >,
) {
    for (entity, hovered, pressed, mut color, mut border_color, mut localized) in
        &mut interaction_query
    {
        let state = match (hovered.get(), pressed) {
            (_, true) => ButtonState::Pressed,
            (true, false) => ButtonState::Hovered,
            (false, false) => ButtonState::Normal,
        };

        if localized.value != state {
            match state {
                ButtonState::Pressed => input_focus.set(entity, FocusCause::Pressed),
                ButtonState::Normal => input_focus.clear(),
                ButtonState::Hovered => {},
            }
            localized.value = state;
        }

        match state {
            ButtonState::Pressed => {
                color.set_if_neq(PRESSED_BUTTON.into());
                border_color.set_if_neq(BorderColor::all(RED));
            },
            ButtonState::Hovered => {
                color.set_if_neq(HOVERED_BUTTON.into());
                border_color.set_if_neq(BorderColor::all(Color::WHITE));
            },
            ButtonState::Normal => {
                color.set_if_neq(NORMAL_BUTTON.into());
                border_color.set_if_neq(BorderColor::all(Color::BLACK));
            },
        }
    }
}

fn locale_button_appearance(
    mut interaction_query: Query<
        (
            &Hovered,
            Has<Pressed>,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<LocaleButton>,
    >,
) {
    for (hovered, pressed, mut color, mut border_color) in &mut interaction_query {
        match (hovered.get(), pressed) {
            (_, true) => {
                color.set_if_neq(PRESSED_BUTTON.into());
                border_color.set_if_neq(BorderColor::all(RED));
            },
            (true, false) => {
                color.set_if_neq(HOVERED_BUTTON.into());
                border_color.set_if_neq(BorderColor::all(Color::WHITE));
            },
            (false, false) => {
                color.set_if_neq(NORMAL_BUTTON.into());
                border_color.set_if_neq(BorderColor::all(Color::BLACK));
            },
        }
    }
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.insert_resource(ClearColor(Color::srgb(0.0, 0.0, 0.0)));
    commands.spawn(button(&assets));
}

fn button(asset_server: &AssetServer) -> impl Bundle {
    let font: FontSource = asset_server.load("fonts/NotoSansSC-Bold.ttf").into();

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
                Hovered::default(),
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
                        font: font.clone(),
                        font_size: FontSize::Px(33.0),
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
                    font: font.clone(),
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                Node {
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    margin: UiRect::bottom(px(20)),
                    ..default()
                },
            ),
            (
                Button,
                Hovered::default(),
                LocaleButton,
                FluentText::new(BevyScreenMessages::ChangeLocaleButton),
                observe(
                    |_activate: On<Activate>,
                     requested_language: Res<RequestedLanguageId>,
                     mut locale_change_messages: MessageWriter<LocaleChangeMessage>| {
                        if let Ok(lang) = Languages::try_from(&requested_language.0) {
                            locale_change_messages.write(LocaleChangeMessage(lang.next().into()));
                        }
                    },
                ),
                Node {
                    width: px(200),
                    height: px(50),
                    border: UiRect::all(px(3)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BorderColor::all(Color::BLACK),
                BackgroundColor(NORMAL_BUTTON),
                children![(
                    Text::new(""),
                    TextFont {
                        font,
                        font_size: FontSize::Px(20.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                )]
            )
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_buttons_preserve_visual_and_message_change_ticks() {
        let mut app = App::new();
        app.init_resource::<InputFocus>();
        app.add_systems(Update, (button_system, locale_button_appearance));
        let button = app
            .world_mut()
            .spawn((
                Button,
                Hovered::default(),
                FluentText::new(ButtonState::Normal),
                BackgroundColor(NORMAL_BUTTON),
                BorderColor::all(Color::BLACK),
            ))
            .id();
        let locale_button = app
            .world_mut()
            .spawn((
                Button,
                LocaleButton,
                Hovered::default(),
                BackgroundColor(NORMAL_BUTTON),
                BorderColor::all(Color::BLACK),
            ))
            .id();

        for pressed in [false, true] {
            if pressed {
                app.world_mut().entity_mut(button).insert(Pressed);
                app.world_mut().entity_mut(locale_button).insert(Pressed);
            }
            app.update();
            assert_eq!(
                app.world()
                    .get::<FluentText<ButtonState>>(button)
                    .unwrap()
                    .value,
                if pressed {
                    ButtonState::Pressed
                } else {
                    ButtonState::Normal
                },
            );
            let visual_ticks = [button, locale_button].map(|entity| {
                let entity = app.world().entity(entity);
                (
                    entity.get_ref::<BackgroundColor>().unwrap().last_changed(),
                    entity.get_ref::<BorderColor>().unwrap().last_changed(),
                )
            });
            let message_tick = app
                .world()
                .entity(button)
                .get_ref::<FluentText<ButtonState>>()
                .unwrap()
                .last_changed();

            app.update();

            for (entity, (color_tick, border_tick)) in
                [button, locale_button].into_iter().zip(visual_ticks)
            {
                let entity = app.world().entity(entity);
                assert_eq!(
                    entity.get_ref::<BackgroundColor>().unwrap().last_changed(),
                    color_tick
                );
                assert_eq!(
                    entity.get_ref::<BorderColor>().unwrap().last_changed(),
                    border_tick
                );
                assert_eq!(
                    entity.get::<BackgroundColor>().unwrap().0,
                    if pressed {
                        PRESSED_BUTTON
                    } else {
                        NORMAL_BUTTON
                    }
                );
            }
            assert_eq!(
                app.world()
                    .entity(button)
                    .get_ref::<FluentText<ButtonState>>()
                    .unwrap()
                    .last_changed(),
                message_tick
            );
        }
    }
}

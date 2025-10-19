use es_fluent::EsFluent;
use es_fluent_lang::es_fluent_language;
use strum::EnumIter;

#[cfg(feature = "bevy")]
use bevy::prelude::*;

#[cfg(feature = "bevy")]
es_fluent_manager_bevy::define_i18n_module!();

#[cfg(any(feature = "iced", feature = "gpui"))]
es_fluent_manager_embedded::define_i18n_module!();

#[derive(Clone, Copy, Debug, Default, EsFluent, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum ButtonState {
    #[default]
    Normal,
    Hovered,
    Pressed,
}

pub struct CurrentLanguage(pub Languages);

#[cfg(feature = "gpui")]
impl gpui::Global for CurrentLanguage {}

#[es_fluent_language]
#[derive(Clone, Copy, Debug, EnumIter, EsFluent, PartialEq)]
pub enum Languages {}

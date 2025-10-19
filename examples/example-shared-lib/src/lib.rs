use es_fluent::EsFluent;
use es_fluent_lang::es_fluent_language;
use strum::EnumIter;
use unic_langid::{LanguageIdentifier, langid};

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

#[derive(Clone, Copy, Debug, EnumIter, EsFluent, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum Languages2 {
    English,
    French,
    Chinese,
}

impl Default for Languages2 {
    fn default() -> Self {
        Languages2::English
    }
}

impl From<Languages2> for LanguageIdentifier {
    fn from(val: Languages2) -> Self {
        match val {
            Languages2::English => langid!("en"),
            Languages2::French => langid!("fr"),
            Languages2::Chinese => langid!("cn"),
        }
    }
}

impl From<&LanguageIdentifier> for Languages2 {
    fn from(lang: &LanguageIdentifier) -> Self {
        match lang.language.as_str() {
            "en" => Languages2::English,
            "fr" => Languages2::French,
            "cn" => Languages2::Chinese,
            _ => Languages2::English,
        }
    }
}

use es_fluent::EsFluent;
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

#[derive(Clone, Copy, Debug, Default, EnumIter, EsFluent, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
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

impl From<&LanguageIdentifier> for Languages {
    fn from(lang: &LanguageIdentifier) -> Self {
        match lang.language.as_str() {
            "en" => Languages::English,
            "fr" => Languages::French,
            "cn" => Languages::Chinese,
            _ => Languages::English,
        }
    }
}

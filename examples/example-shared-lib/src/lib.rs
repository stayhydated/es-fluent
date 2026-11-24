use es_fluent::{EsFluent, EsFluentLabel, EsFluentVariants};
use iso_types::{EnumCount as _, iso_locale};

#[cfg(feature = "bevy")]
use bevy::prelude::*;

#[cfg(feature = "bevy")]
es_fluent_manager_bevy::define_i18n_module!();

#[cfg(any(feature = "dioxus", feature = "embedded"))]
es_fluent_manager_embedded::define_i18n_module!();

#[derive(Clone, Copy, Debug, Default, Eq, EsFluent, PartialEq)]
#[cfg_attr(
    feature = "bevy",
    derive(Component, es_fluent_manager_bevy::BevyFluentText)
)]
pub enum ButtonState {
    #[default]
    Normal,
    Hovered,
    Pressed,
}

#[iso_locale]
pub enum Languages {}

impl Languages {
    pub fn next(self) -> Self {
        Self::from_repr((self as usize + 1) % Self::COUNT)
            .expect("wrapped ordinal is within the configured locales")
    }
}

pub struct CurrentLanguage(pub Languages);

#[derive(Clone, Debug, Default, EsFluent, EsFluentLabel)]
pub struct EmptyStruct;

#[derive(Clone, Debug, Default, EsFluentLabel, EsFluentVariants)]
pub struct EmptyStructVariants;

#[derive(Clone, Copy, Debug, EsFluent, EsFluentLabel)]
pub enum EmptyEnum {}

#[derive(Clone, EsFluent, EsFluentLabel, EsFluentVariants)]
#[fluent_variants(keys = ["description", "label"])]
pub enum Country {
    USA(USAState),
    Canada(CanadaProvince),
}

#[derive(Clone, EsFluent, EsFluentLabel)]
pub enum USAState {
    A,
}

#[derive(Clone, EsFluent, EsFluentLabel)]
pub enum CanadaProvince {
    A,
    B,
}

#[derive(EsFluent, EsFluentLabel, EsFluentVariants)]
#[fluent_variants(keys = ["description", "label"])]
pub struct SplitVariants {
    pub country: Country,
}

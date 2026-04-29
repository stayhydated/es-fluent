use es_fluent::{EsFluent, EsFluentLabel, EsFluentVariants};
use es_fluent_lang::es_fluent_language;
use strum::EnumIter;

#[cfg(feature = "bevy")]
use bevy::prelude::*;

#[cfg(feature = "bevy")]
es_fluent_manager_bevy::define_i18n_module!();

#[cfg(any(feature = "dioxus", feature = "embedded", feature = "gpui"))]
es_fluent_manager_embedded::define_i18n_module!();

/// Force the linker to include this crate and its dependencies.
pub fn force_link() {
    es_fluent_lang::force_link();
}

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

#[cfg(feature = "gpui")]
impl gpui::Global for CurrentLanguage {}

#[es_fluent_language]
#[derive(Clone, Copy, Debug, EnumIter, Eq, EsFluent, PartialEq)]
pub enum Languages {}

impl Languages {
    pub fn next(self) -> Self {
        use strum::IntoEnumIterator as _;
        let all = Self::iter().collect::<Vec<_>>();
        let current_index = all.iter().position(|&l| l == self).unwrap_or(0);
        all[(current_index + 1) % all.len()]
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
#[fluent_label(origin)]
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
#[fluent_label(variants)]
#[fluent_variants(keys = ["description", "label"])]
pub struct SplitVariants {
    pub country: Country,
}

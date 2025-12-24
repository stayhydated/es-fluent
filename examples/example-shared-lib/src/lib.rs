use es_fluent::{EsFluent, EsFluentKv, EsFluentThis};
use es_fluent_lang::es_fluent_language;
use strum::EnumIter;

#[cfg(feature = "bevy")]
use bevy::prelude::*;

#[cfg(feature = "bevy")]
es_fluent_manager_bevy::define_i18n_module!();

#[cfg(any(feature = "embedded", feature = "gpui"))]
es_fluent_manager_embedded::define_i18n_module!();

#[derive(Clone, Copy, Debug, Default, EsFluent, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum ButtonState {
    #[default]
    Normal,
    Hovered,
    Pressed,
}

#[cfg(feature = "gpui")]
impl gpui::Global for CurrentLanguage {}

#[es_fluent_language]
#[derive(Clone, Copy, Debug, EnumIter, EsFluent, PartialEq)]
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

#[derive(Clone, Debug, Default, EsFluent, EsFluentThis)]
pub struct EmptyStruct;

#[derive(Clone, Debug, Default, EsFluentKv, EsFluentThis)]
pub struct EmptyStructKv;

#[derive(Clone, Copy, Debug, EsFluent, EsFluentThis)]
pub enum EmptyEnum {}

#[derive(EsFluent, EsFluentKv, EsFluentThis)]
#[fluent_this(variants_this)] // Was keys_this
#[fluent_kv(keys = ["description", "label"])]
pub enum Country {
    USA(USAState),
    Canada(CanadaProvince),
}

#[derive(EsFluent, EsFluentThis)]
pub enum USAState {
    A,
}

#[derive(EsFluent, EsFluentThis)]
pub enum CanadaProvince {
    A,
    B,
}

#[derive(EsFluent, EsFluentKv, EsFluentThis)]
#[fluent_this(fields_this)] // Was keys_this
#[fluent_kv(keys = ["description", "label"])]
pub struct KvSplit {
    pub country: Country,
}

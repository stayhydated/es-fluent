extern crate self as es_fluent_manager_bevy;

use es_fluent_manager_macros::BevyFluentText;

pub mod unic_langid {
    #[derive(Clone)]
    pub struct LanguageIdentifier;
}

pub mod bevy {
    pub mod prelude {
        pub struct App;
    }
}

pub trait RefreshForLocale {
    fn refresh_for_locale(&mut self, lang: &unic_langid::LanguageIdentifier);
}

pub trait BevyFluentTextRegistration {
    fn register(&self, app: &mut bevy::prelude::App);
}

pub struct FluentTextRegistration;

impl FluentTextRegistration {
    pub fn register_fluent_text_from_locale<T>(_app: &mut bevy::prelude::App) {}
}

pub mod inventory {
    pub use crate::submit;
}

#[macro_export]
macro_rules! submit {
    ($($tt:tt)*) => {};
}

pub enum GeneratedLanguage {
    English,
}

impl TryFrom<&unic_langid::LanguageIdentifier> for GeneratedLanguage {
    type Error = ();

    fn try_from(_value: &unic_langid::LanguageIdentifier) -> Result<Self, Self::Error> {
        Ok(Self::English)
    }
}

pub struct ManualLanguage;

impl TryFrom<&unic_langid::LanguageIdentifier> for ManualLanguage {
    type Error = ();

    fn try_from(_value: &unic_langid::LanguageIdentifier) -> Result<Self, Self::Error> {
        Ok(Self)
    }
}

#[derive(BevyFluentText)]
pub struct StructMessage {
    #[locale]
    language: GeneratedLanguage,
}

#[derive(BevyFluentText)]
pub enum EnumMessage {
    Ready {
        #[locale]
        language: ManualLanguage,
        count: usize,
    },
}

fn main() {}

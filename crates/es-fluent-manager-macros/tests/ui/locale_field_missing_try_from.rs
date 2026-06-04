extern crate self as es_fluent_manager_bevy;

use es_fluent_manager_macros::BevyFluentText;

pub mod unic_langid {
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

pub struct NotRefreshable;

#[derive(BevyFluentText)]
pub struct Message {
    #[locale]
    language: NotRefreshable,
}

fn main() {}

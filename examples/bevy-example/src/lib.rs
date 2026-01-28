use bevy::prelude::Component;
use es_fluent::EsFluent;
use es_fluent_manager_bevy::BevyFluentText;
use example_shared_lib::Languages;

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

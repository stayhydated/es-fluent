use es_fluent_manager_embedded as i18n_manager;

es_fluent_manager_embedded::define_i18n_module!();

use es_fluent::EsFluent;
use es_fluent_lang::es_fluent_language;
use strum::EnumIter;

#[es_fluent_language]
#[derive(Clone, Copy, Debug, EnumIter, EsFluent, PartialEq)]
pub enum Languages {}

pub fn init() {
    i18n_manager::init();
}

pub fn change_locale(language: Languages) -> Result<(), unic_langid::LanguageIdentifierError> {
    i18n_manager::select_language(language);
    Ok(())
}

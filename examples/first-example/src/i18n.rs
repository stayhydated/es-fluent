use es_fluent_manager_embedded as i18n_manager;

use crate::Languages;

es_fluent_manager_embedded::define_i18n_module!();

pub fn init() {
    i18n_manager::init();
}

pub fn change_locale(language: Languages) -> Result<(), unic_langid::LanguageIdentifierError> {
    i18n_manager::select_language(language);
    Ok(())
}

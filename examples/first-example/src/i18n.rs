use es_fluent_manager_embedded as i18n_manager;

use example_shared_lib::Languages;

es_fluent_manager_embedded::define_i18n_module!();

pub fn init() {
    i18n_manager::init();
}

pub fn init_with_language(language: Languages) {
    i18n_manager::init_with_language(language);
}

pub fn change_locale(language: Languages) -> Result<(), i18n_manager::GlobalLocalizationError> {
    i18n_manager::select_language(language)
}

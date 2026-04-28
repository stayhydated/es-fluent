use es_fluent_manager_embedded as i18n_manager;

es_fluent_manager_embedded::define_i18n_module!();

use example_shared_lib::Languages;

pub type I18n = i18n_manager::EmbeddedI18n;

pub fn try_new_with_language(language: Languages) -> Result<I18n, i18n_manager::EmbeddedInitError> {
    i18n_manager::EmbeddedI18n::try_new_with_language(language)
}

pub fn change_locale(
    i18n: &I18n,
    language: Languages,
) -> Result<(), i18n_manager::LanguageSelectionError> {
    i18n.select_language(language)
}

use es_fluent_manager_singleton as i18n_manager;

es_fluent_manager_singleton::define_i18n_module!();

pub fn init() {
    i18n_manager::init();
}

pub fn change_locale(
    language: &unic_langid::LanguageIdentifier,
) -> Result<(), unic_langid::LanguageIdentifierError> {
    i18n_manager::select_language(&language);
    Ok(())
}

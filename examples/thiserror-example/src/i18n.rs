use es_fluent::{FluentManager, set_context, update_context};
use es_fluent_macros::define_i18n_module;

define_i18n_module!("../i18n/");

pub fn init() -> FluentManager {
    let manager = FluentManager::new_with_discovered_modules();
    // Set the global context for derive macros
    set_context(manager.clone());
    manager
}

pub fn change_locale(manager: &mut FluentManager, language: &str) -> Result<(), Box<dyn std::error::Error>> {
    let lang_id: unic_langid::LanguageIdentifier = language.parse()?;
    manager.select_language(&lang_id);
    // Update the global context for derive macros
    update_context(|ctx| *ctx = manager.clone());
    Ok(())
}

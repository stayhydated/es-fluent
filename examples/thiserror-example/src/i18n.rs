use es_fluent::localization::{self, LocalizationContext};
use es_fluent_macros::define_i18n_module;
use unic_langid::LanguageIdentifier;

define_i18n_module!("../i18n/");

pub fn init() {
    let context = LocalizationContext::new_with_discovered_modules();
    localization::set_context(context);
}

pub fn change_locale(language: &str) -> Result<(), Box<dyn std::error::Error>> {
    let lang_id: LanguageIdentifier = language.parse()?;
    localization::with_context(|ctx| {
        ctx.select_language(&lang_id);
    });
    Ok(())
}

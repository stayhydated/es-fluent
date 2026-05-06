use crate::site::i18n::SiteLanguage;
use crate::site::routing::AppRoute;
use dioxus::{document, prelude::*};
use es_fluent_manager_dioxus::I18nProvider;

#[component]
pub fn App() -> Element {
    let stylesheet_href = format!("{}assets/site.css", crate::site::routing::app_base_href());

    rsx! {
        document::Stylesheet { href: stylesheet_href }
        I18nProvider {
            initial_language: SiteLanguage::default().lang(),
            Router::<AppRoute> {}
        }
    }
}

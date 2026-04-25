use crate::site::i18n::SiteLanguage;
use crate::site::routing::AppRoute;
use crate::site::routing::app_base_href;
use dioxus::{document, prelude::*};
use es_fluent_manager_dioxus::use_init_i18n;

#[component]
pub fn App() -> Element {
    let _i18n = use_init_i18n(SiteLanguage::default().lang());
    let stylesheet_href = format!("{}assets/site.css", app_base_href());

    rsx! {
        document::Stylesheet { href: stylesheet_href }
        Router::<AppRoute> {}
    }
}

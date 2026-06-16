use crate::site::i18n::SiteLanguage;
use crate::site::routing::AppRoute;
use dioxus::prelude::*;
use stayhydated_dioxus::StayhydatedLocalizedRouterApp;

#[component]
pub fn App() -> Element {
    let base_href = crate::site::routing::app_base_href();

    rsx! {
        StayhydatedLocalizedRouterApp::<SiteLanguage, AppRoute> {
            base_href: base_href.to_string(),
        }
    }
}

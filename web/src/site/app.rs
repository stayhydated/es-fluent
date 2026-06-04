use crate::site::i18n::{SiteLanguage, app_dioxus_i18n_asset_modules};
use crate::site::routing::AppRoute;
use dioxus::{document, prelude::*};
use es_fluent_manager_dioxus::DioxusAssetI18nProvider;

#[component]
pub fn App() -> Element {
    let base_href = crate::site::routing::app_base_href();
    let stylesheet_href = format!("{base_href}assets/site.css");
    let components_theme_href = format!("{base_href}dx-components-theme.css");

    rsx! {
        stayhydated_dioxus::SharedStyles {}
        document::Stylesheet { href: stylesheet_href }
        document::Stylesheet { href: components_theme_href }
        stayhydated_dioxus::ShaderBackground {}
        DioxusAssetI18nProvider {
            modules: app_dioxus_i18n_asset_modules(),
            initial_language: SiteLanguage::default().lang(),
            Router::<AppRoute> {}
        }
    }
}

use crate::site::i18n::{SiteLanguage, app_dioxus_i18n_asset_modules};
use crate::site::routing::AppRoute;
use dioxus::{document, prelude::*};
use es_fluent_manager_dioxus::DioxusAssetI18nProvider;

#[component]
pub fn App() -> Element {
    let stylesheet_href = format!("{}assets/site.css", crate::site::routing::app_base_href());
    let components_theme_href = format!(
        "{}dx-components-theme.css",
        crate::site::routing::app_base_href()
    );

    rsx! {
        stayhydated_dioxus::SharedStyles {}
        document::Stylesheet { href: stylesheet_href }
        document::Stylesheet { href: components_theme_href }
        DioxusAssetI18nProvider {
            modules: app_dioxus_i18n_asset_modules(),
            initial_language: SiteLanguage::default().lang(),
            Router::<AppRoute> {}
        }
    }
}

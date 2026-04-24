use crate::site::routing::AppRoute;
use crate::site::routing::app_base_href;
use dioxus::{document, prelude::*};

#[component]
pub fn App() -> Element {
    let stylesheet_href = format!("{}assets/site.css", app_base_href());

    rsx! {
        document::Stylesheet { href: stylesheet_href }
        Router::<AppRoute> {}
    }
}

use crate::site::routing::AppRoute;
use dioxus::prelude::*;
use stayhydated_dioxus::StayhydatedRouterApp;

#[component]
pub fn App() -> Element {
    let base_href = crate::site::routing::app_base_href();

    rsx! {
        StayhydatedRouterApp::<AppRoute> {
            base_href: base_href.to_string(),
        }
    }
}

use crate::site::routing::AppRoute;
use dioxus::prelude::*;

const SITE_CSS: &str = include_str!("../../assets/site.css");

#[component]
pub fn App() -> Element {
    rsx! {
        style { "{SITE_CSS}" }
        Router::<AppRoute> {}
    }
}

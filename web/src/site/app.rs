use crate::site::routing::AppRoute;
use dioxus::{document, prelude::*};

#[component]
pub fn App() -> Element {
    rsx! {
        document::Stylesheet { href: asset!("/assets/site.css") }
        Router::<AppRoute> {}
    }
}

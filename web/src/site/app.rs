use crate::site::{constants::site, routing::AppRoute};
use dioxus::prelude::*;
use stayhydated_dioxus::StayhydatedProjectApp;

#[component]
pub fn App() -> Element {
    rsx! {
        StayhydatedProjectApp::<AppRoute> { site: site() }
    }
}

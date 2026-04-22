use crate::site::constants::DEV_SITE_STYLE;
use crate::site::routing::AppRoute;
use dioxus::prelude::Router;
use dioxus_core::Element;
use dioxus_core_macro::{component, rsx};
#[allow(unused_imports)]
use dioxus_html as dioxus_elements;

#[component]
pub fn DevApp() -> Element {
    rsx! {
        style { "{DEV_SITE_STYLE}" }
        Router::<AppRoute> {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_app_is_constructible() {
        let _ = rsx!(DevApp {});
    }
}

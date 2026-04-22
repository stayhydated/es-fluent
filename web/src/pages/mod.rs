mod bevy;
mod demos;
mod error;
mod home;

use crate::site::routing::{PageKind, SiteRoute};
use dioxus_core::Element;
use dioxus_core_macro::rsx;
#[allow(unused_imports)]
use dioxus_html as dioxus_elements;

pub(crate) use error::DevErrorPage;

pub(crate) fn route_content(route: SiteRoute) -> Element {
    match route.page {
        PageKind::Home => rsx!(home::HomePage {
            locale: route.locale
        }),
        PageKind::Demos => rsx!(demos::DemosPage {
            locale: route.locale
        }),
        PageKind::Bevy => rsx!(bevy::BevyPage {
            locale: route.locale
        }),
    }
}

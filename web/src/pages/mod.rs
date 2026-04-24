mod bevy;
mod demos;
mod error;
mod home;

use crate::site::routing::{PageKind, SiteRoute};
use dioxus::prelude::*;

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

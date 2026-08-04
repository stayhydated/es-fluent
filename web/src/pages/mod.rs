mod bevy;
mod demos;
mod dioxus_demo;
mod gpui;
mod home;
pub(crate) mod i18n;

use crate::site::routing::PageKind;
use dioxus::prelude::*;

pub(crate) fn route_content(page: PageKind) -> Element {
    match page {
        PageKind::Home => rsx!(home::HomePage {}),
        PageKind::Demos => rsx!(demos::DemosPage {}),
        PageKind::Dioxus => rsx!(dioxus_demo::DioxusPage {}),
        PageKind::Bevy => rsx!(bevy::BevyPage {}),
        PageKind::Gpui => rsx!(gpui::GpuiPage {}),
    }
}

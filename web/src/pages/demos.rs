use crate::components::{FooterPanel, PageCardLink, PageHeader};
use crate::site::i18n::{DemosPageMessage, SiteLanguage};
use crate::site::routing::PageKind;
use dioxus::prelude::*;

#[component]
pub(crate) fn DemosPage(locale: SiteLanguage) -> Element {
    let demos_style = crate::components::use_reveal_style(0, 24.0).into_string();
    let i18n = match es_fluent_manager_dioxus::use_asset_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { div { class: "page-shell", "failed: {error}" } },
    };
    let label = i18n.localize_message(&DemosPageMessage::BevyLabel);
    let title = i18n.localize_message(&DemosPageMessage::BevyTitle);
    let body = i18n.localize_message(&DemosPageMessage::BevyBody);
    let action = i18n.localize_message(&DemosPageMessage::BevyAction);
    let gpui_label = i18n.localize_message(&DemosPageMessage::GpuiLabel);
    let gpui_title = i18n.localize_message(&DemosPageMessage::GpuiTitle);
    let gpui_body = i18n.localize_message(&DemosPageMessage::GpuiBody);
    let gpui_action = i18n.localize_message(&DemosPageMessage::GpuiAction);

    rsx! {
        div { class: "page-shell",
            PageHeader { locale, current_page: PageKind::Demos }
            main { class: "stack",
                section {
                class: "grid motion-reveal",
                style: demos_style,
                PageCardLink {
                    locale,
                    page: PageKind::Bevy,
                    label,
                    title,
                    body,
                    action,
                }
                PageCardLink {
                    locale,
                    page: PageKind::Gpui,
                    label: gpui_label,
                    title: gpui_title,
                    body: gpui_body,
                    action: gpui_action,
                }
            }
        }
        FooterPanel {}
    }
    }
}

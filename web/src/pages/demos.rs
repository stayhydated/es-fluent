use crate::components::{FooterPanel, PageHeader};
use crate::site::i18n::{DemosPageMessage, SiteLanguage};
use crate::site::routing::PageKind;
use dioxus::prelude::*;
use stayhydated_dioxus::{DemoCard, DemoCardGrid, ProjectPageShell, page_entry_reveal_style};

#[component]
pub(crate) fn DemosPage(locale: SiteLanguage) -> Element {
    let demos_style = page_entry_reveal_style();
    let i18n = match es_fluent_manager_dioxus::use_i18n() {
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
        ProjectPageShell {
            header: rsx!(PageHeader { locale, current_page: PageKind::Demos }),
            footer: Some(rsx!(FooterPanel {})),
            DemoCardGrid::<crate::site::routing::AppRoute> {
                cards: vec![
                    DemoCard::route(
                        crate::site::routing::app_route(locale, PageKind::Bevy),
                        label,
                        title,
                        body,
                        action,
                    ),
                    DemoCard::route(
                        crate::site::routing::app_route(locale, PageKind::Gpui),
                        gpui_label,
                        gpui_title,
                        gpui_body,
                        gpui_action,
                    ),
                ],
                extra_class: "motion-reveal",
                style: demos_style,
                body_class: "card-copy",
            }
        }
    }
}

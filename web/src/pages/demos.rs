use crate::components::{FooterPanel, PageCardLink, PageHeader, use_reveal_style};
use crate::site::i18n::{DemosPageMessage, SiteLanguage};
use crate::site::routing::PageKind;
use dioxus::prelude::*;
use es_fluent_manager_dioxus::use_i18n;

#[component]
pub(crate) fn DemosPage(locale: SiteLanguage) -> Element {
    let demos_style = use_reveal_style(0, 24.0);
    let i18n = match use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { div { class: "page-shell", "failed: {error}" } },
    };
    let label = i18n.localize_message(&DemosPageMessage::Label);
    let title = i18n.localize_message(&DemosPageMessage::Title);
    let action = i18n.localize_message(&DemosPageMessage::Action);

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
                        body: "",
                        action,
                    }
                }
            }
            FooterPanel {}
        }
    }
}

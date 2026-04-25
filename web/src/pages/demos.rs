use crate::components::{FooterPanel, PageCardLink, PageHeader, use_reveal_style};
use crate::site::i18n::{DemosPageMessage, SiteLanguage};
use crate::site::routing::PageKind;
use dioxus::prelude::*;
use es_fluent::ToFluentString as _;

#[component]
pub(crate) fn DemosPage(locale: SiteLanguage) -> Element {
    let demos_style = use_reveal_style(0, 24.0);

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
                        label: DemosPageMessage::Label.to_fluent_string(),
                        title: DemosPageMessage::Title.to_fluent_string(),
                        body: "",
                        action: DemosPageMessage::Action.to_fluent_string(),
                    }
                }
            }
            FooterPanel {}
        }
    }
}

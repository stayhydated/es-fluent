use crate::components::{FooterPanel, PageCardLink, PageHeader};
use crate::site::i18n::{DemosPageMessage, SiteLanguage};
use crate::site::routing::PageKind;
use dioxus::prelude::*;
use es_fluent::ToFluentString as _;

#[component]
pub(crate) fn DemosPage(locale: SiteLanguage) -> Element {
    rsx! {
        div { class: "page-shell",
            PageHeader { locale, current_page: PageKind::Demos }
            main { class: "stack",
                section { class: "grid",
                    PageCardLink {
                        locale,
                        page: PageKind::Bevy,
                        label: DemosPageMessage::BevyLabel.to_fluent_string(),
                        title: DemosPageMessage::BevyTitle.to_fluent_string(),
                        body: "",
                        action: DemosPageMessage::BevyAction.to_fluent_string(),
                    }
                }
            }
            FooterPanel {}
        }
    }
}

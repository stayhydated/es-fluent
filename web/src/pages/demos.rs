use crate::components::{FooterPanel, PageCardLink, PageHeader};
use crate::site::i18n::{SiteLanguage, SiteMessage};
use crate::site::routing::PageKind;
use dioxus_core::Element;
use dioxus_core_macro::{Props, component, rsx};
#[allow(unused_imports)]
use dioxus_html as dioxus_elements;
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
                        label: SiteMessage::DemoBevyLabel.to_fluent_string(),
                        title: SiteMessage::DemoBevyTitle.to_fluent_string(),
                        body: SiteMessage::DemoBevyBody.to_fluent_string(),
                        action: SiteMessage::DemoBevyAction.to_fluent_string(),
                    }
                }
            }
            FooterPanel {}
        }
    }
}

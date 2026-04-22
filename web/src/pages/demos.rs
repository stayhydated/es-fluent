use crate::components::{DemoCard, FooterPanel, PageCardLink, PageHeader};
use crate::site::constants::DIOXUS_EXAMPLE_URL;
use crate::site::i18n::{SiteLanguage, SiteMessage};
use crate::site::routing::{PageKind, book_href};
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
                section { class: "section-panel",
                    h1 { class: "section-title", "{SiteMessage::DemosTitle.to_fluent_string()}" }
                    p { class: "section-lead", "{SiteMessage::DemosLead.to_fluent_string()}" }
                }
                section { class: "grid columns-3",
                    PageCardLink {
                        locale,
                        page: PageKind::Bevy,
                        label: SiteMessage::DemoBevyLabel.to_fluent_string(),
                        title: SiteMessage::DemoBevyTitle.to_fluent_string(),
                        body: SiteMessage::DemoBevyBody.to_fluent_string(),
                        action: SiteMessage::DemoBevyAction.to_fluent_string(),
                    }
                    DemoCard {
                        label: SiteMessage::DemoDioxusLabel.to_fluent_string(),
                        title: SiteMessage::DemoDioxusTitle.to_fluent_string(),
                        body: SiteMessage::DemoDioxusBody.to_fluent_string(),
                        action: SiteMessage::DemoDioxusAction.to_fluent_string(),
                        href: DIOXUS_EXAMPLE_URL.to_string(),
                        external: true,
                    }
                    DemoCard {
                        label: SiteMessage::DemoDocsLabel.to_fluent_string(),
                        title: SiteMessage::DemoDocsTitle.to_fluent_string(),
                        body: SiteMessage::DemoDocsBody.to_fluent_string(),
                        action: SiteMessage::DemoDocsAction.to_fluent_string(),
                        href: book_href(),
                        external: true,
                    }
                }
            }
            FooterPanel {}
        }
    }
}

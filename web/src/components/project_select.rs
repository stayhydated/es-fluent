use dioxus::prelude::*;
use stayhydated_dioxus::{
    DisplayText, Href as ComponentHref, ProjectId, stayhydated_project_options,
};
use stayhydated_site::routing::Href;

#[component]
pub(crate) fn ProjectSelect(href: Href) -> Element {
    let selected = ProjectId::EsFluent.option_with_href(ComponentHref::new(href.into_string()));

    rsx! {
        stayhydated_dioxus::ProjectSelect {
            selected,
            projects: stayhydated_project_options(),
            label: DisplayText::new("Project selector"),
        }
    }
}

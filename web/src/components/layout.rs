use crate::site::routing::PageKind;
use dioxus::prelude::*;
use stayhydated_dioxus::{
    LinkTarget, Project, ProjectFooterPanelForProject, StayhydatedProjectHeader,
    StayhydatedProjectHeaderConfig, stayhydated_header_labels,
};

#[component]
pub(crate) fn PageHeader(current_page: PageKind) -> Element {
    let config = StayhydatedProjectHeaderConfig::new(
        Project::EsFluent,
        crate::site::routing::page_href(PageKind::Home).into_string(),
        LinkTarget::route(crate::site::routing::app_route(PageKind::Home)),
        LinkTarget::route(crate::site::routing::app_route(PageKind::Demos)),
        crate::site::routing::book_href().as_str(),
        stayhydated_header_labels(),
        current_page.project_nav_item(),
    );

    rsx! {
        StayhydatedProjectHeader::<crate::site::routing::AppRoute> {
            config,
        }
    }
}

#[component]
pub(crate) fn FooterPanel() -> Element {
    rsx! {
        ProjectFooterPanelForProject {
            project: Project::EsFluent,
        }
    }
}

use crate::site::i18n::{SiteFooterMessage, SiteLanguage};
use crate::site::routing::{PageKind, app_route};
use dioxus::prelude::*;
use stayhydated_dioxus::{
    LinkTarget, Project, ProjectPackage, ProjectPackageFooterLink, ProjectPackagesFooterPanel,
    RouteLocalizedLanguageSelect, StayhydatedProjectHeader, StayhydatedProjectHeaderConfig,
    stayhydated_project_options_for_locale,
};

#[component]
pub(crate) fn PageHeader(locale: SiteLanguage, current_page: PageKind) -> Element {
    let i18n = match es_fluent_manager_dioxus::use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { header { class: "page-header", "failed: {error}" } },
    };
    let project_options =
        stayhydated_project_options_for_locale(locale, |message| i18n.localize_message(&message));
    let config = StayhydatedProjectHeaderConfig::localized_with_i18n(
        Project::EsFluent,
        crate::site::routing::page_href(locale, PageKind::Home).into_string(),
        LinkTarget::route(app_route(locale, PageKind::Home)),
        LinkTarget::route(app_route(locale, PageKind::Demos)),
        crate::site::routing::book_href().as_str(),
        current_page.project_nav_item(),
        &i18n,
    )
    .with_project_options(project_options);

    rsx! {
        StayhydatedProjectHeader::<crate::site::routing::AppRoute> {
            config,
            LocaleSwitcher { locale, current_page }
        }
    }
}

#[component]
fn LocaleSwitcher(locale: SiteLanguage, current_page: PageKind) -> Element {
    let i18n = match es_fluent_manager_dioxus::use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { div { class: "locale-switcher-dropdown", "failed: {error}" } },
    };
    rsx! {
        RouteLocalizedLanguageSelect::<SiteLanguage, _, crate::site::routing::AppRoute> {
            localizer: i18n,
            selected: locale,
            route_for_language: move |next_locale| app_route(next_locale, current_page),
        }
    }
}

#[component]
pub(crate) fn FooterPanel() -> Element {
    let i18n = match es_fluent_manager_dioxus::use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { footer { class: "site-footer", "failed: {error}" } },
    };
    let label = i18n.localize_message(&SiteFooterMessage::Label);
    let prefix = format!("{} ", i18n.localize_message(&SiteFooterMessage::BodyPrefix));
    let body_link_label = i18n.localize_message(&SiteFooterMessage::BodyLinkLabel);
    let packages = vec![
        ProjectPackageFooterLink::new(ProjectPackage::ES_FLUENT_MANAGER_DIOXUS)
            .with_label(body_link_label)
            .with_class("footer-link"),
    ];

    rsx! {
        ProjectPackagesFooterPanel {
            label,
            packages,
            prefix,
            suffix: ".",
        }
    }
}

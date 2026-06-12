use crate::components::LanguageSelect;
use crate::site::constants::ES_FLUENT_MANAGER_DIOXUS_CRATES_URL;
use crate::site::i18n::{SiteChromeMessage, SiteFooterMessage, SiteLanguage};
use crate::site::routing::{PageKind, app_route};
use dioxus::prelude::*;
use dioxus::router::{navigator, try_router};
use stayhydated_dioxus::{
    FooterPanel as SharedFooterPanel, LinkTarget, ProjectChromeHeader, ProjectId, ProjectNavConfig,
    ProjectNavLabels,
};

#[component]
pub(crate) fn PageHeader(locale: SiteLanguage, current_page: PageKind) -> Element {
    let i18n = match es_fluent_manager_dioxus::use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { header { class: "page-header", "failed: {error}" } },
    };
    let nav_home = i18n.localize_message(&SiteChromeMessage::NavHome);
    let nav_demos = i18n.localize_message(&SiteChromeMessage::NavDemos);
    let nav_docs = i18n.localize_message(&SiteChromeMessage::NavDocs);
    let nav_source = i18n.localize_message(&SiteChromeMessage::NavSource);
    let nav = ProjectNavConfig::new(
        ProjectId::EsFluent,
        crate::site::routing::page_href(locale, PageKind::Home).as_str(),
        LinkTarget::route(app_route(locale, PageKind::Home)),
        LinkTarget::route(app_route(locale, PageKind::Demos)),
        crate::site::routing::book_href().as_str(),
        ProjectNavLabels::new(nav_home, nav_demos, nav_docs, nav_source),
        current_page.project_nav_item(),
    );

    rsx! {
        ProjectChromeHeader::<crate::site::routing::AppRoute> {
            nav,
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
    let language_links = SiteLanguage::all()
        .map(|candidate| {
            let label = i18n.localize_message(&candidate);
            (candidate, label)
        })
        .collect::<Vec<_>>();
    let on_locale_changed = move |next_locale: SiteLanguage| {
        if next_locale == locale {
            return;
        }

        if try_router().is_some() {
            let _ = navigator().push(app_route(next_locale, current_page));
        }
    };

    rsx! {
        LanguageSelect::<SiteLanguage> {
            selected: locale,
            options: language_links,
            on_change: on_locale_changed,
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
    let body_prefix = i18n.localize_message(&SiteFooterMessage::BodyPrefix);
    let body_link_label = i18n.localize_message(&SiteFooterMessage::BodyLinkLabel);

    rsx! {
        SharedFooterPanel {
            p { class: "footer-copy",
                span { class: "footer-label", "{label}" }
                span { class: "footer-text",
                    "{body_prefix} "
                    a {
                        class: "footer-link",
                        href: ES_FLUENT_MANAGER_DIOXUS_CRATES_URL,
                        target: "_blank",
                        rel: "noreferrer",
                        "{body_link_label}"
                    }
                    "."
                }
            }
        }
    }
}

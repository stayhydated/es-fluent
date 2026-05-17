use crate::components::{PageLink, select};
use crate::site::constants::ES_FLUENT_MANAGER_DIOXUS_CRATES_URL;
use crate::site::i18n::{SiteChromeMessage, SiteFooterMessage, SiteLanguage};
use crate::site::routing::{PageKind, app_route};
use dioxus::prelude::*;
use dioxus::router::{navigator, try_router};

#[component]
pub(crate) fn PageHeader(locale: SiteLanguage, current_page: PageKind) -> Element {
    let i18n = match es_fluent_manager_dioxus::use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { header { class: "page-header", "failed: {error}" } },
    };
    let page_kicker = i18n.localize_message(&SiteChromeMessage::PageKicker);
    let site_name = i18n.localize_message(&SiteChromeMessage::SiteName);
    let nav_home = i18n.localize_message(&SiteChromeMessage::NavHome);
    let nav_demos = i18n.localize_message(&SiteChromeMessage::NavDemos);
    let nav_docs = i18n.localize_message(&SiteChromeMessage::NavDocs);
    let nav_source = i18n.localize_message(&SiteChromeMessage::NavSource);

    rsx! {
        header { class: "page-header",
            a { class: "brand", href: crate::site::routing::page_href(locale, PageKind::Home),
                    span { class: "brand-mark", "EF" }
                    span { class: "brand-copy",
                        span { class: "brand-kicker", "{page_kicker}" }
                        span { class: "brand-title", "{site_name}" }
                    }
                }
                div { class: "header-cluster",
                    HeaderNavLinks {
                    locale,
                    current_page,
                    nav_home,
                    nav_demos,
                    nav_docs,
                    nav_source,
                }
                LocaleSwitcher { locale, current_page }
            }
        }
    }
}

#[component]
fn HeaderNavLinks(
    locale: SiteLanguage,
    current_page: PageKind,
    nav_home: String,
    nav_demos: String,
    nav_docs: String,
    nav_source: String,
) -> Element {
    let is_home_active = current_page == PageKind::Home;
    let is_demos_active = matches!(
        current_page,
        PageKind::Demos | PageKind::Bevy | PageKind::Gpui
    );

    rsx! {
        nav { class: "header-nav-links",
            PageLink {
                locale,
                page: PageKind::Home,
                class: if is_home_active {
                    "header-nav-item is-active".to_string()
                } else {
                    "header-nav-item".to_string()
                },
                label: nav_home,
            }
            PageLink {
                locale,
                page: PageKind::Demos,
                class: if is_demos_active {
                    "header-nav-item is-active".to_string()
                } else {
                    "header-nav-item".to_string()
                },
                label: nav_demos,
            }
            ExternalNavLink {
                href: crate::site::routing::book_href(),
                class: "header-nav-item".to_string(),
                label: nav_docs,
            }
            ExternalNavLink {
                href: "https://github.com/stayhydated/es-fluent".to_string(),
                class: "header-nav-item".to_string(),
                label: nav_source,
            }
        }
    }
}

#[component]
fn ExternalNavLink(href: String, class: String, label: String) -> Element {
    rsx! {
        a {
            class,
            href,
            target: "_blank",
            rel: "noreferrer",
            "{label}"
        }
    }
}

#[component]
fn LocaleSwitcher(locale: SiteLanguage, current_page: PageKind) -> Element {
    let i18n = match es_fluent_manager_dioxus::use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { div { class: "locale-switcher-dropdown", "failed: {error}" } },
    };
    let locale_label = i18n.localize_message(&SiteChromeMessage::LocaleLabel);
    let language_links = SiteLanguage::all()
        .map(|candidate| {
            let label = i18n.localize_message(&candidate);
            (candidate, label)
        })
        .collect::<Vec<_>>();
    let mut selected_locale = use_signal(|| Some(locale));
    use_effect(move || {
        if selected_locale() != Some(locale) {
            selected_locale.set(Some(locale));
        }
    });

    let on_locale_changed = move |next_locale: Option<SiteLanguage>| {
        let Some(next_locale) = next_locale else {
            return;
        };

        if next_locale == locale {
            return;
        }

        selected_locale.set(Some(next_locale));

        if try_router().is_some() {
            let _ = navigator().push(app_route(next_locale, current_page));
        }
    };

    rsx! {
        div { class: "locale-switcher-dropdown",
            span { class: "locale-label", "{locale_label}" }
            select::Select::<SiteLanguage> {
                value: Some(selected_locale.into()),
                on_value_change: on_locale_changed,
                select::SelectTrigger {
                    select::SelectValue {
                        placeholder: locale_label,
                        class: Some("header-locale-value".to_string()),
                    }
                }
                select::SelectList {
                    for (index, (candidate, label)) in language_links.iter().enumerate() {
                        select::SelectOption::<SiteLanguage> {
                            index,
                            value: *candidate,
                            text_value: Some(label.clone()),
                            class: Some(if Some(*candidate) == selected_locale() {
                                "header-locale-option is-active".to_string()
                            } else {
                                "header-locale-option".to_string()
                            }),
                            "{label}"
                            if Some(*candidate) == selected_locale() {
                                select::SelectItemIndicator {}
                            }
                        }
                    }
                }
            }
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
        footer { class: "site-footer",
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

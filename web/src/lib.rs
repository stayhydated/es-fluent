use anyhow::{Context, Result};
use dioxus_core::Element;
#[cfg(feature = "web")]
use dioxus_core::use_hook;
use dioxus_core_macro::{Props, component, rsx};
#[allow(unused_imports)]
use dioxus_html as dioxus_elements;
use es_fluent::{EsFluent, ToFluentString as _};
#[cfg(feature = "web")]
use es_fluent_manager_dioxus::ManagedI18n;
use es_fluent_manager_dioxus::{GlobalLocalizerMode, ssr::SsrI18n};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use unic_langid::{LanguageIdentifier, langid};
use walkdir::WalkDir;
#[cfg(feature = "web")]
use web_sys::window;

es_fluent_manager_dioxus::define_i18n_module!();

const SITE_URL: &str = "https://stayhydated.github.io/es-fluent/";
const README_URL: &str = "https://github.com/stayhydated/es-fluent/blob/master/README.md";
const CRATES_URL: &str = "https://github.com/stayhydated/es-fluent/tree/master/crates";
const DIOXUS_EXAMPLE_URL: &str =
    "https://github.com/stayhydated/es-fluent/tree/master/examples/dioxus-example";
#[cfg(feature = "web")]
const DEV_SITE_STYLE: &str = include_str!("../assets/site.css");
#[cfg(any(feature = "web", test))]
const SITE_BASE_PATH_SEGMENT: &str = "es-fluent";
const INSTALL_SNIPPET: &str = r#"[dependencies]
es-fluent = { version = "*", features = ["derive"] }
unic-langid = "*"

# Pick one runtime manager:
es-fluent-manager-embedded = "*"
es-fluent-manager-bevy = "*"
es-fluent-manager-dioxus = { version = "*", features = ["web"] }"#;
const BEVY_BOOTSTRAP: &str = r#"const root = document.getElementById("bevy-loader");
if (!root) {
  throw new Error("Missing Bevy loader root");
}

const setState = (state) => {
  root.dataset.state = state;
};

(async () => {
  try {
    const moduleUrl = new URL("bevy-example.js", window.location.href);
    const wasmModule = await import(moduleUrl.href);
    if (typeof wasmModule.default !== "function") {
      throw new Error("Missing default init export");
    }
    await wasmModule.default();
    setState("ready");
  } catch (error) {
    console.error(error);
    setState("error");
  }
})();"#;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SiteLocale {
    EnUs,
    FrFr,
}

impl SiteLocale {
    fn all() -> [Self; 2] {
        [Self::EnUs, Self::FrFr]
    }

    fn lang(self) -> LanguageIdentifier {
        match self {
            Self::EnUs => langid!("en-US"),
            Self::FrFr => langid!("fr-FR"),
        }
    }

    fn html_lang(self) -> &'static str {
        match self {
            Self::EnUs => "en-US",
            Self::FrFr => "fr-FR",
        }
    }

    fn prefix(self) -> &'static str {
        match self {
            Self::EnUs => "",
            Self::FrFr => "fr/",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PageKind {
    Home,
    Demos,
    Bevy,
}

impl PageKind {
    fn route(self) -> &'static str {
        match self {
            Self::Home => "",
            Self::Demos => "demos/",
            Self::Bevy => "bevy-example/",
        }
    }

    fn title_message(self) -> SiteMessage {
        match self {
            Self::Home => SiteMessage::HomePageTitle,
            Self::Demos => SiteMessage::DemosPageTitle,
            Self::Bevy => SiteMessage::BevyPageTitle,
        }
    }

    fn description_message(self) -> SiteMessage {
        match self {
            Self::Home => SiteMessage::HeroBody,
            Self::Demos => SiteMessage::DemosLead,
            Self::Bevy => SiteMessage::BevyLead,
        }
    }

    fn is_fullscreen(self) -> bool {
        matches!(self, Self::Bevy)
    }
}

#[derive(Clone, Copy, Debug, EsFluent)]
enum SiteMessage {
    PageKicker,
    SiteName,
    NavHome,
    NavDemos,
    NavDocs,
    NavSource,
    LocaleLabel,
    LocaleEnglish,
    LocaleFrench,
    HeroEyebrow,
    HeroTitle,
    HeroBody,
    HeroPrimary,
    HeroSecondary,
    HeroPanelLabel,
    HeroPanelOneTitle,
    HeroPanelOneBody,
    HeroPanelTwoTitle,
    HeroPanelTwoBody,
    HeroPanelThreeTitle,
    HeroPanelThreeBody,
    InstallTitle,
    InstallLead,
    InstallNote,
    FeatureTitle,
    FeatureLead,
    FeatureOneTitle,
    FeatureOneBody,
    FeatureTwoTitle,
    FeatureTwoBody,
    FeatureThreeTitle,
    FeatureThreeBody,
    LinksTitle,
    LinksLead,
    LinksBook,
    LinksReadme,
    LinksCrates,
    DemosTitle,
    DemosLead,
    DemoBevyLabel,
    DemoBevyTitle,
    DemoBevyBody,
    DemoBevyAction,
    DemoDioxusLabel,
    DemoDioxusTitle,
    DemoDioxusBody,
    DemoDioxusAction,
    DemoDocsLabel,
    DemoDocsTitle,
    DemoDocsBody,
    DemoDocsAction,
    BevyTitle,
    BevyLead,
    BevyLoading,
    BevyError,
    BackToDemos,
    FooterLabel,
    FooterBody,
    HomePageTitle,
    DemosPageTitle,
    BevyPageTitle,
}

#[derive(Clone, Copy)]
struct RenderTarget {
    locale: SiteLocale,
    page: PageKind,
    output_dir: &'static str,
}

#[cfg_attr(not(any(feature = "web", test)), allow(dead_code))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SiteRoute {
    locale: SiteLocale,
    page: PageKind,
}

#[cfg(feature = "web")]
#[derive(Clone)]
struct ManagedI18nHandle(ManagedI18n);

#[cfg(feature = "web")]
impl PartialEq for ManagedI18nHandle {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

const RENDER_TARGETS: [RenderTarget; 6] = [
    RenderTarget {
        locale: SiteLocale::EnUs,
        page: PageKind::Home,
        output_dir: "",
    },
    RenderTarget {
        locale: SiteLocale::EnUs,
        page: PageKind::Demos,
        output_dir: "demos",
    },
    RenderTarget {
        locale: SiteLocale::EnUs,
        page: PageKind::Bevy,
        output_dir: "bevy-example",
    },
    RenderTarget {
        locale: SiteLocale::FrFr,
        page: PageKind::Home,
        output_dir: "fr",
    },
    RenderTarget {
        locale: SiteLocale::FrFr,
        page: PageKind::Demos,
        output_dir: "fr/demos",
    },
    RenderTarget {
        locale: SiteLocale::FrFr,
        page: PageKind::Bevy,
        output_dir: "fr/bevy-example",
    },
];

pub fn run() -> Result<()> {
    match std::env::args().nth(1).as_deref() {
        None | Some("build") => build_site(),
        Some(other) => anyhow::bail!("unsupported command: {other}"),
    }
}

pub fn build_site() -> Result<()> {
    let web_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dist_dir = web_dir.join("dist");
    build_site_into(&web_dir, &dist_dir)
}

fn build_site_into(web_dir: &Path, dist_dir: &Path) -> Result<()> {
    if dist_dir.exists() {
        fs::remove_dir_all(dist_dir)
            .with_context(|| format!("failed to remove {}", dist_dir.display()))?;
    }
    fs::create_dir_all(dist_dir)
        .with_context(|| format!("failed to create {}", dist_dir.display()))?;

    copy_directory(&web_dir.join("public"), dist_dir)?;
    fs::copy(web_dir.join("assets/site.css"), dist_dir.join("site.css")).with_context(|| {
        format!(
            "failed to copy {} to {}",
            web_dir.join("assets/site.css").display(),
            dist_dir.join("site.css").display()
        )
    })?;

    for target in RENDER_TARGETS {
        let page_dir = dist_dir.join(target.output_dir);
        fs::create_dir_all(&page_dir)
            .with_context(|| format!("failed to create {}", page_dir.display()))?;
        let page_html = render_page(
            target.locale,
            target.page,
            &site_root_prefix(target.output_dir),
        )?;
        fs::write(page_dir.join("index.html"), page_html).with_context(|| {
            format!("failed to write {}", page_dir.join("index.html").display())
        })?;
    }

    let home_404 = render_page(SiteLocale::EnUs, PageKind::Home, "./")?;
    fs::write(dist_dir.join("404.html"), home_404)
        .with_context(|| format!("failed to write {}", dist_dir.join("404.html").display()))?;
    fs::write(dist_dir.join("sitemap.xml"), render_sitemap())
        .with_context(|| format!("failed to write {}", dist_dir.join("sitemap.xml").display()))?;

    Ok(())
}

fn copy_directory(source: &Path, destination: &Path) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }

    for entry in WalkDir::new(source) {
        let entry = entry.with_context(|| format!("failed to walk {}", source.display()))?;
        let relative = entry
            .path()
            .strip_prefix(source)
            .with_context(|| format!("failed to strip prefix {}", source.display()))?;

        if relative.as_os_str().is_empty() {
            continue;
        }

        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)
                .with_context(|| format!("failed to create {}", target.display()))?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
            fs::copy(entry.path(), &target).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    entry.path().display(),
                    target.display()
                )
            })?;
        }
    }

    Ok(())
}

fn render_page(locale: SiteLocale, page: PageKind, site_root: &str) -> Result<String> {
    let i18n = SsrI18n::try_new_with_discovered_modules_and_mode(
        locale.lang(),
        GlobalLocalizerMode::ReplaceExisting,
    )
    .context("failed to initialize the Dioxus SSR localizer")?;

    let title = i18n.with_manager(|| {
        format!(
            "{} | {}",
            SiteMessage::SiteName.to_fluent_string(),
            page.title_message().to_fluent_string()
        )
    });
    let description = i18n.with_manager(|| page.description_message().to_fluent_string());
    let body = match page {
        PageKind::Home => i18n.render_element(rsx!(HomePage { locale })),
        PageKind::Demos => i18n.render_element(rsx!(DemosPage { locale })),
        PageKind::Bevy => i18n.render_element(rsx!(BevyPage { locale })),
    };

    Ok(render_document(
        locale,
        page,
        site_root,
        &title,
        &description,
        &body,
    ))
}

fn render_document(
    locale: SiteLocale,
    page: PageKind,
    site_root: &str,
    title: &str,
    description: &str,
    body: &str,
) -> String {
    let body_class = if page.is_fullscreen() {
        "fullscreen"
    } else {
        "standard"
    };

    format!(
        "<!doctype html>\n<html lang=\"{lang}\">\n<head>\n  <meta charset=\"utf-8\" />\n  \
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n  \
         <meta name=\"description\" content=\"{description}\" />\n  <base href=\"{site_root}\" />\n  \
         <title>{title}</title>\n  <link rel=\"stylesheet\" href=\"site.css\" />\n</head>\n\
         <body class=\"{body_class}\">\n{body}\n</body>\n</html>\n",
        lang = locale.html_lang(),
        description = escape_html(description),
        site_root = site_root,
        title = escape_html(title),
        body_class = body_class,
        body = body,
    )
}

fn render_sitemap() -> String {
    let mut entries = String::new();
    for route in [
        "",
        "demos/",
        "bevy-example/",
        "book/",
        "fr/",
        "fr/demos/",
        "fr/bevy-example/",
    ] {
        let _ = writeln!(
            entries,
            "  <url><loc>{SITE_URL}{route}</loc></url>",
            route = route
        );
    }

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n{entries}</urlset>\n"
    )
}

fn site_root_prefix(output_dir: &str) -> String {
    if output_dir.is_empty() {
        return "./".to_string();
    }

    let depth = output_dir.split('/').count();
    "../".repeat(depth)
}

fn app_base_href() -> &'static str {
    "/es-fluent/"
}

fn page_href(locale: SiteLocale, page: PageKind) -> String {
    let relative = format!("{}{}", locale.prefix(), page.route());
    let relative = relative.trim_end_matches('/');
    if relative.is_empty() {
        app_base_href().to_string()
    } else {
        format!("{}{relative}/", app_base_href())
    }
}

fn book_href() -> String {
    format!("{}book/", app_base_href())
}

fn locale_label(locale: SiteLocale) -> String {
    match locale {
        SiteLocale::EnUs => SiteMessage::LocaleEnglish.to_fluent_string(),
        SiteLocale::FrFr => SiteMessage::LocaleFrench.to_fluent_string(),
    }
}

#[cfg(any(feature = "web", test))]
fn route_for(locale: SiteLocale, page: PageKind) -> SiteRoute {
    SiteRoute { locale, page }
}

#[cfg(any(feature = "web", test))]
fn site_route_from_path(path: &str) -> SiteRoute {
    let segments = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .filter(|segment| *segment != SITE_BASE_PATH_SEGMENT)
        .collect::<Vec<_>>();

    match segments.as_slice() {
        ["fr"] => route_for(SiteLocale::FrFr, PageKind::Home),
        ["fr", "demos"] => route_for(SiteLocale::FrFr, PageKind::Demos),
        ["fr", "bevy-example"] => route_for(SiteLocale::FrFr, PageKind::Bevy),
        ["demos"] => route_for(SiteLocale::EnUs, PageKind::Demos),
        ["bevy-example"] => route_for(SiteLocale::EnUs, PageKind::Bevy),
        _ => route_for(SiteLocale::EnUs, PageKind::Home),
    }
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(feature = "web")]
fn current_site_route() -> SiteRoute {
    let pathname = window()
        .and_then(|window| window.location().pathname().ok())
        .unwrap_or_else(|| "/".to_string());
    site_route_from_path(&pathname)
}

#[cfg(feature = "web")]
#[component]
pub fn DevApp() -> Element {
    let route = current_site_route();
    let init_result =
        use_hook(
            move || match ManagedI18n::try_new_with_discovered_modules(route.locale.lang()) {
                Ok(managed) => Ok(ManagedI18nHandle(managed)),
                Err(error) => Err(error.to_string()),
            },
        );

    match init_result.as_ref() {
        Ok(managed) => rsx! {
            style { "{DEV_SITE_STYLE}" }
            DevLocalizedApp { route, managed: managed.clone() }
        },
        Err(error) => rsx! {
            style { "{DEV_SITE_STYLE}" }
            DevErrorPage { route, message: error.to_string() }
        },
    }
}

#[cfg(feature = "web")]
#[component]
fn DevLocalizedApp(route: SiteRoute, managed: ManagedI18nHandle) -> Element {
    let _i18n = es_fluent_manager_dioxus::web::use_provide_i18n_with_mode(
        managed.0.clone(),
        GlobalLocalizerMode::ReplaceExisting,
    );

    match route.page {
        PageKind::Home => rsx!(HomePage {
            locale: route.locale
        }),
        PageKind::Demos => rsx!(DemosPage {
            locale: route.locale
        }),
        PageKind::Bevy => rsx!(BevyPage {
            locale: route.locale
        }),
    }
}

#[cfg(feature = "web")]
#[component]
fn DevErrorPage(route: SiteRoute, message: String) -> Element {
    rsx! {
        div { class: "page-shell",
            PageHeader { locale: route.locale, current_page: route.page }
            main { class: "stack",
                section { class: "section-panel",
                    h1 { class: "section-title", "Dioxus web startup failed" }
                    p { class: "section-lead", "The client runtime could not initialize the localized app." }
                    pre { code { "{message}" } }
                }
            }
        }
    }
}

#[component]
fn PageHeader(locale: SiteLocale, current_page: PageKind) -> Element {
    rsx! {
        header { class: "page-header",
            a { class: "brand", href: page_href(locale, PageKind::Home),
                span { class: "brand-mark", "EF" }
                span { class: "brand-copy",
                    span { class: "brand-kicker", "{SiteMessage::PageKicker.to_fluent_string()}" }
                    span { class: "brand-title", "{SiteMessage::SiteName.to_fluent_string()}" }
                }
            }
            div { class: "header-cluster",
                nav { class: "nav-pill",
                    NavLink {
                        href: page_href(locale, PageKind::Home),
                        label: SiteMessage::NavHome.to_fluent_string(),
                        is_active: current_page == PageKind::Home,
                    }
                    NavLink {
                        href: page_href(locale, PageKind::Demos),
                        label: SiteMessage::NavDemos.to_fluent_string(),
                        is_active: current_page == PageKind::Demos || current_page == PageKind::Bevy,
                    }
                    NavLink {
                        href: book_href(),
                        label: SiteMessage::NavDocs.to_fluent_string(),
                        is_active: false,
                    }
                    NavLink {
                        href: "https://github.com/stayhydated/es-fluent".to_string(),
                        label: SiteMessage::NavSource.to_fluent_string(),
                        is_active: false,
                    }
                }
                LocaleSwitcher { locale, current_page }
            }
        }
    }
}

#[component]
fn NavLink(href: String, label: String, is_active: bool) -> Element {
    let active_class = if is_active {
        " nav-link is-active"
    } else {
        " nav-link"
    };
    let is_external = href.starts_with("http");

    rsx! {
        a {
            class: "{active_class}",
            href,
            target: if is_external { Some("_blank") } else { None },
            rel: if is_external { Some("noreferrer") } else { None },
            "{label}"
        }
    }
}

#[component]
fn LocaleSwitcher(locale: SiteLocale, current_page: PageKind) -> Element {
    rsx! {
        div { class: "locale-switcher",
            span { class: "locale-label", "{SiteMessage::LocaleLabel.to_fluent_string()}" }
            for candidate in SiteLocale::all() {
                a {
                    class: if candidate == locale { "locale-link is-active" } else { "locale-link" },
                    href: page_href(candidate, current_page),
                    "{locale_label(candidate)}"
                }
            }
        }
    }
}

#[component]
fn FooterPanel() -> Element {
    rsx! {
        footer { class: "footer-panel",
            div {
                div { class: "footer-label", "{SiteMessage::FooterLabel.to_fluent_string()}" }
                p { class: "footer-copy", "{SiteMessage::FooterBody.to_fluent_string()}" }
            }
            ul { class: "footer-links",
                li { a { class: "text-link", href: book_href(), "{SiteMessage::LinksBook.to_fluent_string()}" } }
                li { a { class: "text-link", href: README_URL, target: "_blank", rel: "noreferrer", "{SiteMessage::LinksReadme.to_fluent_string()}" } }
                li { a { class: "text-link", href: CRATES_URL, target: "_blank", rel: "noreferrer", "{SiteMessage::LinksCrates.to_fluent_string()}" } }
            }
        }
    }
}

#[component]
fn HomePage(locale: SiteLocale) -> Element {
    rsx! {
        div { class: "page-shell",
            PageHeader { locale, current_page: PageKind::Home }
            main { class: "stack",
                section { class: "hero",
                    div {
                        div { class: "eyebrow", "{SiteMessage::HeroEyebrow.to_fluent_string()}" }
                        h1 { "{SiteMessage::HeroTitle.to_fluent_string()}" }
                        p { "{SiteMessage::HeroBody.to_fluent_string()}" }
                        div { class: "hero-actions",
                            a { class: "button-link primary", href: book_href(), "{SiteMessage::HeroPrimary.to_fluent_string()}" }
                            a { class: "button-link secondary", href: page_href(locale, PageKind::Demos), "{SiteMessage::HeroSecondary.to_fluent_string()}" }
                        }
                    }
                    aside { class: "hero-panel",
                        h2 { class: "panel-label", "{SiteMessage::HeroPanelLabel.to_fluent_string()}" }
                        ul { class: "hero-list",
                            li {
                                strong { "{SiteMessage::HeroPanelOneTitle.to_fluent_string()}" }
                                span { class: "feature-copy", "{SiteMessage::HeroPanelOneBody.to_fluent_string()}" }
                            }
                            li {
                                strong { "{SiteMessage::HeroPanelTwoTitle.to_fluent_string()}" }
                                span { class: "feature-copy", "{SiteMessage::HeroPanelTwoBody.to_fluent_string()}" }
                            }
                            li {
                                strong { "{SiteMessage::HeroPanelThreeTitle.to_fluent_string()}" }
                                span { class: "feature-copy", "{SiteMessage::HeroPanelThreeBody.to_fluent_string()}" }
                            }
                        }
                    }
                }
                section { class: "grid columns-2",
                    article { class: "code-panel",
                        div { class: "panel-label", "{SiteMessage::InstallTitle.to_fluent_string()}" }
                        p { class: "section-lead", "{SiteMessage::InstallLead.to_fluent_string()}" }
                        pre { code { "{INSTALL_SNIPPET}" } }
                        p { class: "feature-copy", "{SiteMessage::InstallNote.to_fluent_string()}" }
                    }
                    article { class: "section-panel",
                        h2 { class: "section-title", "{SiteMessage::LinksTitle.to_fluent_string()}" }
                        p { class: "section-lead", "{SiteMessage::LinksLead.to_fluent_string()}" }
                        ul { class: "inline-links",
                            li { a { class: "button-link secondary", href: book_href(), "{SiteMessage::LinksBook.to_fluent_string()}" } }
                            li { a { class: "button-link secondary", href: README_URL, target: "_blank", rel: "noreferrer", "{SiteMessage::LinksReadme.to_fluent_string()}" } }
                            li { a { class: "button-link secondary", href: CRATES_URL, target: "_blank", rel: "noreferrer", "{SiteMessage::LinksCrates.to_fluent_string()}" } }
                        }
                    }
                }
                section { class: "section-panel",
                    h2 { class: "section-title", "{SiteMessage::FeatureTitle.to_fluent_string()}" }
                    p { class: "section-lead", "{SiteMessage::FeatureLead.to_fluent_string()}" }
                    div { class: "grid columns-3",
                        FeatureCard {
                            title: SiteMessage::FeatureOneTitle.to_fluent_string(),
                            body: SiteMessage::FeatureOneBody.to_fluent_string(),
                        }
                        FeatureCard {
                            title: SiteMessage::FeatureTwoTitle.to_fluent_string(),
                            body: SiteMessage::FeatureTwoBody.to_fluent_string(),
                        }
                        FeatureCard {
                            title: SiteMessage::FeatureThreeTitle.to_fluent_string(),
                            body: SiteMessage::FeatureThreeBody.to_fluent_string(),
                        }
                    }
                }
            }
            FooterPanel {}
        }
    }
}

#[component]
fn FeatureCard(title: String, body: String) -> Element {
    rsx! {
        article { class: "demo-card",
            h2 { "{title}" }
            p { class: "card-copy", "{body}" }
        }
    }
}

#[component]
fn DemosPage(locale: SiteLocale) -> Element {
    rsx! {
        div { class: "page-shell",
            PageHeader { locale, current_page: PageKind::Demos }
            main { class: "stack",
                section { class: "section-panel",
                    h1 { class: "section-title", "{SiteMessage::DemosTitle.to_fluent_string()}" }
                    p { class: "section-lead", "{SiteMessage::DemosLead.to_fluent_string()}" }
                }
                section { class: "grid columns-3",
                    DemoCard {
                        label: SiteMessage::DemoBevyLabel.to_fluent_string(),
                        title: SiteMessage::DemoBevyTitle.to_fluent_string(),
                        body: SiteMessage::DemoBevyBody.to_fluent_string(),
                        action: SiteMessage::DemoBevyAction.to_fluent_string(),
                        href: page_href(locale, PageKind::Bevy),
                        external: false,
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
                        external: false,
                    }
                }
            }
            FooterPanel {}
        }
    }
}

#[component]
fn DemoCard(
    label: String,
    title: String,
    body: String,
    action: String,
    href: String,
    external: bool,
) -> Element {
    rsx! {
        a {
            class: "demo-card",
            href,
            target: external.then_some("_blank"),
            rel: external.then_some("noreferrer"),
            div { class: "card-label", "{label}" }
            h2 { "{title}" }
            p { class: "card-copy", "{body}" }
            span { class: "card-link", "{action}" }
        }
    }
}

#[component]
fn BevyPage(locale: SiteLocale) -> Element {
    rsx! {
        div { class: "fullscreen-demo",
            a { class: "back-pill", href: page_href(locale, PageKind::Demos), "{SiteMessage::BackToDemos.to_fluent_string()}" }
            div { class: "loader-stage",
                div { class: "loader-card", id: "bevy-loader", "data-state": "loading",
                    div { class: "loader-kicker", "{SiteMessage::BevyPageTitle.to_fluent_string()}" }
                    h1 { class: "loader-title", "{SiteMessage::BevyTitle.to_fluent_string()}" }
                    p { class: "loader-copy", "{SiteMessage::BevyLead.to_fluent_string()}" }
                    p { class: "status-line", "data-state": "loading", "{SiteMessage::BevyLoading.to_fluent_string()}" }
                    p { class: "status-line", "data-state": "error", "{SiteMessage::BevyError.to_fluent_string()}" }
                }
            }
            script {
                r#type: "module",
                dangerous_inner_html: "{BEVY_BOOTSTRAP}"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn renders_english_home_page() {
        let html = render_page(SiteLocale::EnUs, PageKind::Home, "./").expect("page should render");
        assert!(html.contains("Ship localized Rust UIs without drifting out of sync."));
        assert!(html.contains("href=\"/es-fluent/demos/\""));
        assert!(html.contains("href=\"/es-fluent/book/\""));
    }

    #[test]
    #[serial]
    fn renders_french_demos_page() {
        let html =
            render_page(SiteLocale::FrFr, PageKind::Demos, "../../").expect("page should render");
        assert!(html.contains("Démos navigateur et pistes d’intégration"));
        assert!(html.contains("href=\"/es-fluent/fr/bevy-example/\""));
        assert!(html.contains("Ouvrir la source"));
    }

    #[test]
    fn computes_site_root_prefixes() {
        assert_eq!(site_root_prefix(""), "./");
        assert_eq!(site_root_prefix("demos"), "../");
        assert_eq!(site_root_prefix("fr/demos"), "../../");
    }

    #[test]
    fn parses_site_routes() {
        assert_eq!(
            site_route_from_path("/es-fluent/fr/demos/"),
            route_for(SiteLocale::FrFr, PageKind::Demos)
        );
        assert_eq!(
            site_route_from_path("/bevy-example/"),
            route_for(SiteLocale::EnUs, PageKind::Bevy)
        );
        assert_eq!(
            site_route_from_path("/unknown"),
            route_for(SiteLocale::EnUs, PageKind::Home)
        );
    }
}

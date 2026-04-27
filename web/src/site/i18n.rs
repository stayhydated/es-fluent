use es_fluent::EsFluent;
use es_fluent_lang::{LanguageIdentifier, es_fluent_language};
use strum::{EnumIter, IntoEnumIterator as _};

es_fluent_manager_dioxus::define_i18n_module!();

#[es_fluent_language]
#[derive(Clone, Copy, Debug, EnumIter, Eq, EsFluent, PartialEq)]
pub(crate) enum SiteLanguage {}

impl SiteLanguage {
    pub(crate) fn all() -> impl Iterator<Item = Self> {
        Self::iter()
    }

    pub(crate) fn lang(self) -> LanguageIdentifier {
        self.into()
    }

    pub(crate) fn html_lang(self) -> String {
        self.lang().to_string()
    }

    pub(crate) fn is_default(self) -> bool {
        self == Self::default()
    }

    pub(crate) fn route_slug(self) -> Option<String> {
        (!self.is_default()).then(|| self.lang().language.to_string())
    }

    pub(crate) fn from_route_slug(slug: &str) -> Option<Self> {
        Self::all().find(|locale| locale.route_slug().as_deref() == Some(slug))
    }
}

#[derive(Clone, Copy, Debug, EsFluent)]
pub(crate) enum SiteChromeMessage {
    PageKicker,
    SiteName,
    NavHome,
    NavDemos,
    NavDocs,
    NavSource,
    LocaleLabel,
}

#[derive(Clone, Copy, Debug, EsFluent)]
pub(crate) enum HomeHeroMessage {
    Eyebrow,
    Title,
    Body,
    PrimaryAction,
    SecondaryAction,
    PanelLabel,
    PanelOneTitle,
    PanelOneBody,
    PanelTwoTitle,
    PanelTwoBody,
    PanelThreeTitle,
    PanelThreeBody,
}

#[derive(Clone, Copy, Debug, EsFluent)]
pub(crate) enum HomeWorkflowMessage {
    Title,
    Lead,
    OneTitle,
    OneBody,
    TwoTitle,
    TwoBody,
    ThreeTitle,
    ThreeBody,
}

#[derive(Clone, Copy, Debug, EsFluent)]
pub(crate) enum DemosPageMessage {
    #[fluent(key = "BevyLabel")]
    Label,
    #[fluent(key = "BevyTitle")]
    Title,
    #[fluent(key = "BevyBody")]
    Body,
    #[fluent(key = "BevyAction")]
    Action,
}

#[derive(Clone, Copy, Debug, EsFluent)]
pub(crate) enum BevyPageMessage {
    Title,
    Lead,
    BackToDemos,
}

#[derive(Clone, Copy, Debug, EsFluent)]
pub(crate) enum SiteFooterMessage {
    Label,
    BodyPrefix,
    BodyLinkLabel,
}

#[derive(Clone, Copy, Debug, EsFluent)]
pub(crate) enum PageMetadataMessage {
    #[fluent(key = "HomeTitle")]
    Home,
    #[fluent(key = "DemosTitle")]
    Demos,
    #[fluent(key = "BevyTitle")]
    Bevy,
}

use es_fluent::{EsFluent, ToFluentString as _};
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

    pub(crate) fn display_label(self) -> String {
        self.to_fluent_string()
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

    #[cfg(any(feature = "web", test))]
    pub(crate) fn from_route_slug(slug: &str) -> Option<Self> {
        Self::all().find(|locale| locale.route_slug().as_deref() == Some(slug))
    }
}

#[derive(Clone, Copy, Debug, EsFluent)]
pub(crate) enum SiteMessage {
    PageKicker,
    SiteName,
    NavHome,
    NavDemos,
    NavDocs,
    NavSource,
    LocaleLabel,
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

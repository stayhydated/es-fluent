use es_fluent::EsFluent;
use es_fluent_lang::es_fluent_language;
use strum::EnumIter;

es_fluent_manager_dioxus::define_i18n_module!();

#[es_fluent_language]
#[derive(Clone, Copy, Debug, EnumIter, Eq, EsFluent, PartialEq)]
pub(crate) enum SiteLanguage {}

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
    OneLabel,
    OneTitle,
    OneBody,
    TwoLabel,
    TwoTitle,
    TwoBody,
    ThreeLabel,
    ThreeTitle,
    ThreeBody,
}

#[derive(Clone, Copy, Debug, EsFluent)]
pub(crate) enum DemosPageMessage {
    BevyLabel,
    BevyTitle,
    BevyBody,
    BevyAction,
    GpuiLabel,
    GpuiTitle,
    GpuiBody,
    GpuiAction,
}

#[derive(Clone, Copy, Debug, EsFluent)]
pub(crate) enum BevyPageMessage {
    Title,
    Lead,
    BackToDemos,
}

#[derive(Clone, Copy, Debug, EsFluent)]
pub(crate) enum GpuiPageMessage {
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
#[allow(clippy::enum_variant_names)]
pub(crate) enum PageMetadataMessage {
    HomeTitle,
    DemosTitle,
    DemosDescription,
    BevyTitle,
    GpuiTitle,
}

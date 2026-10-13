use es_fluent::EsFluent;
use iso_types::iso_locale;

es_fluent_manager_dioxus::define_i18n_module!();

#[iso_locale]
pub(crate) enum DemoLanguage {}

#[derive(Clone, Copy, Debug, EsFluent)]
pub(crate) enum EmberEchoCopy {
    RestaurantName,
    Kicker,
    Title,
    Manifesto,
    SignatureName,
    SignatureDescription,
    RivalryClaim,
    SelectLabel,
    ImageAlt,
    DoughTitle,
    DoughBody,
    FireTitle,
    FireBody,
    FinishTitle,
    FinishBody,
}

#[derive(Debug, EsFluent)]
pub(crate) struct CourtyardLocaleStatus<'a> {
    pub(crate) locale: &'a str,
}

#[derive(Debug, EsFluent)]
pub(crate) struct CourtyardWelcome<'a> {
    pub(crate) guest: &'a str,
}

#[derive(Debug, EsFluent)]
pub(crate) struct PiesAtDusk {
    pub(crate) count: i32,
}

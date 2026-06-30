use es_fluent::EsFluent;
use es_fluent_lang::es_fluent_language;
use strum::EnumIter;

es_fluent_manager_dioxus::define_i18n_module!();

#[es_fluent_language]
#[derive(Clone, Copy, Debug, EnumIter, Eq, EsFluent, PartialEq)]
pub(crate) enum DemoLanguage {}

#[derive(Clone, Copy, Debug, EsFluent)]
pub(crate) enum DioxusDemoMessage {
    PanelLabel,
    Title,
    Body,
    ResultLabel,
    ResultBody,
    RuntimeTitle,
    RuntimeBody,
    ResourceTitle,
    ResourceBody,
}

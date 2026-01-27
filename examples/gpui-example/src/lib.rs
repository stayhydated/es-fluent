use es_fluent::EsFluent;
use example_shared_lib::Languages;

pub mod i18n;

#[derive(Clone, Copy, Debug, EsFluent)]
pub enum GpuiScreenMessages {
    ToggleLanguageHint { current_language: Languages },
}

use es_fluent::EsFluent;

#[cfg(feature = "embedded")]
pub mod i18n {
    es_fluent_manager_embedded::define_i18n_module!();
}

#[derive(EsFluent)]
pub struct OwnerAGreeting<'a> {
    pub name: &'a str,
}

#[derive(EsFluent)]
#[fluent(domain = "ui")]
pub struct SharedUiGreeting<'a> {
    pub name: &'a str,
}

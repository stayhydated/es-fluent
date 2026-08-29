//! Rust-owned localization contract for the framework-neutral TypeScript demo.

use es_fluent::EsFluent;

#[derive(EsFluent)]
pub enum DemoCopy {
    Kicker,
    Title,
    Body,
    SwitchLocale,
}

#[derive(EsFluent)]
pub struct LocaleStatus<'a> {
    pub locale: &'a str,
}

#[derive(EsFluent)]
pub struct Greeting<'a> {
    pub name: &'a str,
}

#[derive(EsFluent)]
pub struct Inbox {
    pub count: i32,
}

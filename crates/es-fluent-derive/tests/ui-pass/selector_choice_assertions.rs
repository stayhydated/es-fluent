extern crate es_fluent;

use es_fluent::EsFluentChoice as _;
use es_fluent_derive::{EsFluent, EsFluentChoice};

#[derive(EsFluentChoice)]
pub enum DerivedTone {
    Friendly,
}

pub struct ManualTone;

impl es_fluent::EsFluentChoice for ManualTone {
    fn as_fluent_choice(&self) -> &'static str {
        "manual"
    }
}

pub mod nested {
    pub struct NestedTone;

    impl es_fluent::EsFluentChoice for NestedTone {
        fn as_fluent_choice(&self) -> &'static str {
            "nested"
        }
    }
}

#[derive(EsFluent)]
pub struct DerivedSelector {
    #[fluent(selector)]
    tone: DerivedTone,
}

#[derive(EsFluent)]
pub struct ManualSelector {
    #[fluent(selector)]
    tone: ManualTone,
}

#[derive(EsFluent)]
pub struct BorrowedSelector<'a> {
    #[fluent(selector)]
    tone: &'a ManualTone,
}

#[derive(EsFluent)]
pub struct NestedSelector {
    #[fluent(selector)]
    tone: nested::NestedTone,
}

#[derive(EsFluent)]
pub struct GenericSelector<T>
where
    T: es_fluent::EsFluentChoice,
{
    #[fluent(selector)]
    tone: T,
}

fn main() {
    let _ = ManualTone.as_fluent_choice();
}

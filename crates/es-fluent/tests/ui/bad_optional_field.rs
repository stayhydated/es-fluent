extern crate es_fluent;

use es_fluent_derive::EsFluent;

#[derive(EsFluent)]
pub struct BadOptionalField {
    #[fluent(optional)]
    value: String,
}

fn main() {}

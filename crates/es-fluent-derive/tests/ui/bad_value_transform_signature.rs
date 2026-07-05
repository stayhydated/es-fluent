extern crate es_fluent;

use es_fluent_derive::EsFluent;

#[derive(EsFluent)]
pub struct BadValueTransform {
    #[fluent(value = |value: u32| value)]
    value: String,
}

fn main() {}

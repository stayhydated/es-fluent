extern crate es_fluent;

use es_fluent_derive::EsFluent;

struct NotChoice;

#[derive(EsFluent)]
pub struct BadChoiceField {
    #[fluent(selector)]
    value: NotChoice,
}

fn main() {}

extern crate es_fluent;

use es_fluent_derive::EsFluentLabel;

#[derive(EsFluentLabel)]
#[fluent_label(variants)]
pub struct LoginForm {
    username: String,
}

fn main() {}

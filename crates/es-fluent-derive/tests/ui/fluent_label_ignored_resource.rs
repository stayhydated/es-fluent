extern crate es_fluent;

use es_fluent_derive::EsFluentLabel;

#[derive(EsFluentLabel)]
#[fluent(id = "login_error")]
pub enum LoginError {
    InvalidPassword,
}

fn main() {}

extern crate es_fluent;

use es_fluent_derive::EsFluentVariants;

#[derive(EsFluentVariants)]
#[fluent(resource = "login_error")]
pub enum LoginError {
    InvalidPassword,
}

fn main() {}

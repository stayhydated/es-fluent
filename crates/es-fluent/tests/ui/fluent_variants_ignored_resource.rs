extern crate es_fluent;

use es_fluent_derive::EsFluentVariants;

#[derive(EsFluentVariants)]
#[fluent(id = "login_error")]
pub enum LoginError {
    InvalidPassword,
}

fn main() {}

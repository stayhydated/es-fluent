extern crate es_fluent;

use es_fluent_derive::EsFluentVariants;

#[derive(EsFluentVariants)]
#[fluent_label(origin)]
pub struct LoginForm {
    username: String,
}

fn main() {}

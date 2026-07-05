extern crate es_fluent;

use es_fluent_derive::EsFluentVariants;

#[derive(EsFluentVariants)]
#[fluent_variants(keys = ["label"])]
pub struct LoginForm {
    #[fluent_variants(skip)]
    username: String,
}

fn main() {}
